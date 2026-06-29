// Linking phase (whole program): L1 load, L2 resolve entities, L3 merge relationships,
// L4 synthesize definitions + validate merges, L5 semantic review, L6 checks.
use crate::cache::{entity_slug, Store};
use crate::compile::{hash_hex, rel_rank};
use crate::llm::Llm;
use crate::model::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Serialize, Deserialize)]
struct L4Cache {
    coherent: bool,
    definition: String,
    reasoning: String,
}

#[derive(Serialize, Deserialize)]
struct L5Item {
    rule: String,
    severity: String,
    message: String,
    reasoning: Option<String>,
}

pub fn link(
    objects: &[(String, ObjectArtifact)],
    roots: &BTreeSet<String>,
    llm: &Llm,
    store: Option<&Store>,
) -> (LinkedArtifact, ReviewedArtifact) {
    // L2: resolve entities across files by normalized name (conservative, deterministic).
    let mut groups: BTreeMap<String, (String, Vec<Member>)> = BTreeMap::new();
    let mut local_to_global: BTreeMap<(String, String), String> = BTreeMap::new();
    for (obj, art) in objects {
        for (lid, ent) in &art.entities {
            let gid = format!("ent:{}", crate::md::slug(&ent.name));
            let g = groups.entry(gid.clone()).or_insert((ent.name.clone(), vec![]));
            g.1.push(Member {
                object: obj.clone(),
                local_id: lid.clone(),
                role: Some(ent.role.clone()),
            });
            local_to_global.insert((obj.clone(), lid.clone()), gid);
        }
    }

    let mut resolve_diags: Vec<Diagnostic> = Vec::new();
    let mut entities: Vec<GlobalEntity> = Vec::new();
    for (gid, val) in &groups {
        let name = &val.0;
        let members = &val.1;
        let distinct: BTreeSet<&String> = members.iter().map(|m| &m.object).collect();
        let resolved_by = if distinct.len() > 1 { "name-only" } else { "single" };
        if distinct.len() > 1 {
            resolve_diags.push(Diagnostic {
                id: format!("name-only-link:{}", gid),
                rule: "name-only-link".into(),
                severity: "warning".into(),
                subjects: vec![gid.clone()],
                message: format!("'{}' linked across {} files by name only; add an explicit link to confirm", name, distinct.len()),
                reasoning: None,
            });
        }
        entities.push(GlobalEntity {
            global_id: gid.clone(),
            canonical_name: name.clone(),
            aliases: vec![],
            scope: None,
            members: members.clone(),
            resolved_by: resolved_by.into(),
            confidence: 0.7,
            global_definition: None,
        });
    }

    // L3: re-key edges to global entities and merge.
    let mut gedges: BTreeMap<(String, String), (String, Vec<String>)> = BTreeMap::new();
    for (obj, art) in objects {
        for rel in &art.relationships {
            if rel.members.len() < 2 {
                continue;
            }
            let ga = local_to_global.get(&(obj.clone(), rel.members[0].clone()));
            let gb = local_to_global.get(&(obj.clone(), rel.members[1].clone()));
            if let (Some(ga), Some(gb)) = (ga, gb) {
                if ga == gb {
                    continue;
                }
                let (x, y) = if ga <= gb { (ga.clone(), gb.clone()) } else { (gb.clone(), ga.clone()) };
                let reqs: Vec<String> = rel.requirements.iter().map(|r| format!("req:{}:{}", obj, r)).collect();
                let ent = gedges.entry((x, y)).or_insert((rel.kind.clone(), vec![]));
                if rel_rank(&rel.kind) < rel_rank(&ent.0) {
                    ent.0 = rel.kind.clone();
                }
                for r in reqs {
                    if !ent.1.contains(&r) {
                        ent.1.push(r);
                    }
                }
            }
        }
    }
    let mut relationships: Vec<GlobalRel> = Vec::new();
    for (key, val) in gedges.iter() {
        relationships.push(GlobalRel {
            global_id: format!("rel:{}~{}", key.0.trim_start_matches("ent:"), key.1.trim_start_matches("ent:")),
            kind: val.0.clone(),
            members: vec![key.0.clone(), key.1.clone()],
            requirements: val.1.clone(),
        });
    }

    // Global requirement index, and group requirements per entity for L5.
    let mut requirements: Vec<GlobalReq> = Vec::new();
    let mut ent_reqs: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (obj, art) in objects {
        for req in &art.requirements {
            let ents: Vec<String> = req
                .entity_refs
                .iter()
                .filter_map(|l| local_to_global.get(&(obj.clone(), l.clone())).cloned())
                .collect();
            for e in &ents {
                ent_reqs.entry(e.clone()).or_default().push(req.ears_text.clone());
            }
            requirements.push(GlobalReq {
                global_id: format!("req:{}:{}", obj, req.id),
                ears_text: req.ears_text.clone(),
                entities: ents,
                source_section: req.source_section.clone(),
            });
        }
    }

    let linked = LinkedArtifact {
        entities: entities.clone(),
        relationships: relationships.clone(),
        requirements: requirements.clone(),
        diagnostics: resolve_diags.clone(),
    };

    // L4: synthesize global definitions and validate merges. Single-definition entities are set
    // directly; multi-definition entities each call the LLM, run in parallel.
    let mut review_diags: Vec<Diagnostic> = resolve_diags.clone();
    let mut rev_entities = entities.clone();
    struct L4Job {
        idx: usize,
        gid: String,
        name: String,
        key: String,
        user: String,
        fallback: String,
    }
    let mut l4_jobs: Vec<L4Job> = Vec::new();
    for (idx, ge) in rev_entities.iter_mut().enumerate() {
        let mut defs: Vec<(String, String)> = Vec::new();
        for m in &ge.members {
            if let Some((_, art)) = objects.iter().find(|(o, _)| o == &m.object) {
                if let Some(le) = art.entities.get(&m.local_id) {
                    if !le.local_definition.is_empty() {
                        defs.push((m.object.clone(), le.local_definition.clone()));
                    }
                }
            }
        }
        if defs.len() <= 1 {
            ge.global_definition = defs.into_iter().next().map(|(_, d)| d);
            continue;
        }
        let user = format!(
            "Entity: {}\nDefinitions:\n{}",
            ge.canonical_name,
            defs.iter().map(|(o, d)| format!("- ({}): {}", o, d)).collect::<Vec<_>>().join("\n")
        );
        let key = format!(
            "L4\u{0}{}\u{0}{}\u{0}{}",
            ge.global_id,
            ge.resolved_by,
            defs.iter().map(|(o, d)| format!("{}={}", o, d)).collect::<Vec<_>>().join("|")
        );
        l4_jobs.push(L4Job {
            idx,
            gid: ge.global_id.clone(),
            name: ge.canonical_name.clone(),
            key,
            user,
            fallback: defs[0].1.clone(),
        });
    }
    let l4_sys = "Given several documents' definitions of the same named entity, decide if they describe one coherent thing. Return ONLY JSON: {\"coherent\":true|false,\"definition\":string,\"reasoning\":string}. 'definition' is one coherent sentence if coherent, else an empty string.";
    let l4_schema = serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["coherent", "definition", "reasoning"],
        "properties": {
            "coherent": {"type": "boolean"},
            "definition": {"type": "string"},
            "reasoning": {"type": "string"}
        }
    });
    enum L4Out {
        Def(String),
        False(String),
        Err(String),
    }
    let workers = crate::parallel::max_workers();
    let l4_outs = crate::parallel::par_map(&l4_jobs, workers, |_, j| {
        let slug = entity_slug(&j.gid);
        let key = hash_hex(&j.key);
        let r = match store.and_then(|s| s.get_link::<L4Cache>(&slug, "synthesis", &key)) {
            Some(r) => r,
            None => match llm.chat_json(l4_sys, &j.user, "synthesis", &l4_schema, &format!("link · L4 synthesize {}", j.name)) {
                Ok(v) => {
                    let r = L4Cache {
                        coherent: v.get("coherent").and_then(|x| x.as_bool()).unwrap_or(true),
                        definition: v.get("definition").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                        reasoning: v.get("reasoning").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                    };
                    if let Some(s) = store {
                        s.put_link(&slug, "synthesis", &key, &r);
                    }
                    r
                }
                Err(e) => return L4Out::Err(e),
            },
        };
        if r.coherent {
            L4Out::Def(if r.definition.is_empty() { j.fallback.clone() } else { r.definition })
        } else {
            L4Out::False(r.reasoning)
        }
    });
    for (j, out) in l4_jobs.iter().zip(l4_outs.into_iter()) {
        match out {
            L4Out::Def(d) => rev_entities[j.idx].global_definition = Some(d),
            L4Out::False(reasoning) => review_diags.push(Diagnostic {
                id: format!("false-merge:{}", j.gid),
                rule: "false-merge".into(),
                severity: "error".into(),
                subjects: vec![j.gid.clone()],
                message: format!("definitions of '{}' across files are not coherent; likely clashing names. Rename one, or add an explicit link.", j.name),
                reasoning: Some(reasoning),
            }),
            L4Out::Err(e) => review_diags.push(Diagnostic {
                id: format!("synthesize-error:{}", j.gid),
                rule: "synthesize-error".into(),
                severity: "info".into(),
                subjects: vec![j.gid.clone()],
                message: format!("could not synthesize definition: {}", e),
                reasoning: None,
            }),
        }
    }

    // L5: semantic review for cross-doc entities, one LLM call per entity, run in parallel.
    let l5_sys = "Given the requirements for one entity gathered across documents, find real problems. Return ONLY JSON: {\"diagnostics\":[{\"rule\":string,\"severity\":\"error\"|\"warning\"|\"info\",\"message\":string,\"reasoning\":string}]}. Use rules like cross-doc-contradiction, redefinition, overlapping-requirements, incompleteness. Return an empty array if there are no problems.";
    let l5_schema = serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["diagnostics"],
        "properties": {
            "diagnostics": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["rule", "severity", "message", "reasoning"],
                    "properties": {
                        "rule": {"type": "string"},
                        "severity": {"type": "string", "enum": ["error", "warning", "info"]},
                        "message": {"type": "string"},
                        "reasoning": {"type": "string"}
                    }
                }
            }
        }
    });
    struct L5Job {
        gid: String,
        key: String,
        user: String,
    }
    let mut l5_jobs: Vec<L5Job> = Vec::new();
    for ge in rev_entities.iter() {
        let distinct: BTreeSet<&String> = ge.members.iter().map(|m| &m.object).collect();
        if distinct.len() <= 1 {
            continue;
        }
        let reqs = match ent_reqs.get(&ge.global_id) {
            Some(r) if !r.is_empty() => r,
            _ => continue,
        };
        let user = format!(
            "Entity: {}\nRequirements:\n{}",
            ge.canonical_name,
            reqs.iter().map(|r| format!("- {}", r)).collect::<Vec<_>>().join("\n")
        );
        let key = format!(
            "L5\u{0}{}\u{0}{}\u{0}{}",
            ge.global_id,
            ge.global_definition.clone().unwrap_or_default(),
            reqs.join("|")
        );
        l5_jobs.push(L5Job { gid: ge.global_id.clone(), key, user });
    }
    let l5_outs = crate::parallel::par_map(&l5_jobs, workers, |_, j| -> Vec<L5Item> {
        let slug = entity_slug(&j.gid);
        let key = hash_hex(&j.key);
        if let Some(items) = store.and_then(|s| s.get_link::<Vec<L5Item>>(&slug, "review", &key)) {
            return items;
        }
        match llm.chat_json(l5_sys, &j.user, "review", &l5_schema, &format!("link · L5 review {}", slug)) {
            Ok(v) => {
                let items: Vec<L5Item> = v
                    .get("diagnostics")
                    .and_then(|x| x.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|d| {
                                let msg = d.get("message").and_then(|x| x.as_str()).unwrap_or("").to_string();
                                if msg.is_empty() {
                                    return None;
                                }
                                let sev = d.get("severity").and_then(|x| x.as_str()).unwrap_or("warning");
                                let sev = if ["error", "warning", "info"].contains(&sev) { sev } else { "warning" };
                                Some(L5Item {
                                    rule: d.get("rule").and_then(|x| x.as_str()).unwrap_or("semantic").to_string(),
                                    severity: sev.to_string(),
                                    message: msg,
                                    reasoning: d.get("reasoning").and_then(|x| x.as_str()).map(|s| s.to_string()),
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                if let Some(s) = store {
                    s.put_link(&slug, "review", &key, &items);
                }
                items
            }
            Err(_) => Vec::new(),
        }
    });
    for (j, items) in l5_jobs.iter().zip(l5_outs.into_iter()) {
        for (i, it) in items.into_iter().enumerate() {
            review_diags.push(Diagnostic {
                id: format!("{}:{}:{}", it.rule, j.gid, i),
                rule: it.rule,
                severity: it.severity,
                subjects: vec![j.gid.clone()],
                message: it.message,
                reasoning: it.reasoning,
            });
        }
    }

    // L6: checks. Coverage, unused-entity (in no relationship), and reachability from roots.
    let mut in_rel: BTreeSet<String> = BTreeSet::new();
    let mut adj: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for r in &relationships {
        for m in &r.members {
            in_rel.insert(m.clone());
        }
        if r.members.len() >= 2 {
            adj.entry(r.members[0].clone()).or_default().push(r.members[1].clone());
            adj.entry(r.members[1].clone()).or_default().push(r.members[0].clone());
        }
    }

    // Root entities: those with a member defined in a roots file. Reachability is a BFS over edges.
    let mut reachable: BTreeSet<String> = BTreeSet::new();
    if !roots.is_empty() {
        let mut frontier: Vec<String> = Vec::new();
        for ge in &rev_entities {
            if ge.members.iter().any(|m| roots.contains(&m.object)) {
                reachable.insert(ge.global_id.clone());
                frontier.push(ge.global_id.clone());
            }
        }
        while let Some(n) = frontier.pop() {
            if let Some(ns) = adj.get(&n) {
                for nb in ns.clone() {
                    if reachable.insert(nb.clone()) {
                        frontier.push(nb);
                    }
                }
            }
        }
    }

    let mut coverage: Vec<Coverage> = Vec::new();
    for ge in &rev_entities {
        let n = ent_reqs.get(&ge.global_id).map(|r| r.len()).unwrap_or(0);
        coverage.push(Coverage {
            entity: ge.global_id.clone(),
            tests_derivable: n,
        });
        let defined = ge.members.iter().any(|m| m.role.as_deref() == Some("definition"));
        if defined && !in_rel.contains(&ge.global_id) {
            review_diags.push(Diagnostic {
                id: format!("unused-entity:{}", ge.global_id),
                rule: "unused-entity".into(),
                severity: "warning".into(),
                subjects: vec![ge.global_id.clone()],
                message: format!("entity '{}' is defined but referenced by no relationship", ge.canonical_name),
                reasoning: None,
            });
        }
        if !roots.is_empty() && !reachable.contains(&ge.global_id) {
            review_diags.push(Diagnostic {
                id: format!("unreachable-entity:{}", ge.global_id),
                rule: "unreachable-entity".into(),
                severity: "warning".into(),
                subjects: vec![ge.global_id.clone()],
                message: format!("entity '{}' is not reachable from any root entity", ge.canonical_name),
                reasoning: None,
            });
        }
    }

    let reviewed = ReviewedArtifact {
        entities: rev_entities,
        relationships,
        requirements,
        coverage,
        diagnostics: review_diags,
    };
    (linked, reviewed)
}

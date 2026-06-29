// Compilation phase (per file): A1 parse, A2 entities, A3 requirements + implied edges,
// A4 consolidate relationships, A5 local definitions (folded into A2). Produces an object artifact.
use crate::cache::Store;
use crate::llm::Llm;
use crate::md;
use crate::model::*;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::path::Path;

// Bump when any LLM-stage prompt changes, to invalidate cached results.
pub const PROMPT_VERSION: &str = "4";

// A3 per-section result, cached by section text + entity table. Entity references are stored by
// name (stable across runs) and resolved to local ids after extraction.
#[derive(Serialize, Deserialize, Clone)]
struct A3Edge {
    members: Vec<String>,
    kind: String,
}
#[derive(Serialize, Deserialize, Clone)]
struct A3Req {
    ears: String,
    entities: Vec<String>,
    evidence: String,
    edges: Vec<A3Edge>,
}

// Per-stage cache/artifact payloads written to the source-mirrored out dir (see cache.rs).
// `sections.yaml` (A1) and `entities.yaml` (A2) are slices of the object artifact; `requirements.yaml`
// (A3) keeps a per-section key so editing one section regenerates only that section.
#[derive(Serialize, Deserialize)]
struct SectionsArtifact {
    sections: BTreeMap<String, SectionBody>,
}
#[derive(Serialize, Deserialize)]
struct EntitiesArtifact {
    entities: BTreeMap<String, LocalEntity>,
}
#[derive(Serialize, Deserialize, Clone)]
struct SectionReqs {
    key: String,
    requirements: Vec<A3Req>,
}
#[derive(Serialize, Deserialize)]
struct RequirementsArtifact {
    sections: BTreeMap<String, SectionReqs>,
}

pub const REL_TYPES: [&str; 7] = [
    "generalization",
    "realization",
    "composition",
    "aggregation",
    "association",
    "dependency",
    "reference",
];

// 0 is strongest, 6 is weakest.
pub fn rel_rank(t: &str) -> usize {
    REL_TYPES.iter().position(|&x| x == t).unwrap_or(6)
}

fn norm_type(t: &str) -> String {
    let t = t.trim().to_lowercase();
    if REL_TYPES.contains(&t.as_str()) {
        t
    } else {
        "reference".to_string()
    }
}

pub fn hash_hex(s: &str) -> String {
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    format!("{:016x}", h.finish())
}

// Parse an A3 LLM response into the cacheable per-section requirement list.
fn parse_a3(v: &serde_json::Value) -> Vec<A3Req> {
    let arr = match v.get("requirements").and_then(|x| x.as_array()) {
        Some(a) => a,
        None => return Vec::new(),
    };
    arr.iter()
        .filter_map(|item| {
            let ears = item
                .get("ears")
                .and_then(|v| v.as_str())
                .or_else(|| item.get("requirement").and_then(|v| v.as_str()))
                .or_else(|| item.get("text").and_then(|v| v.as_str()))
                .unwrap_or("")
                .trim()
                .to_string();
            if ears.is_empty() {
                return None;
            }
            let entities = item
                .get("entities")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|e| e.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            let evidence = item.get("evidence").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            let edges = item
                .get("edges")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|ed| {
                            let members: Vec<String> = ed
                                .get("members")
                                .and_then(|v| v.as_array())
                                .map(|m| m.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
                                .unwrap_or_default();
                            if members.len() < 2 {
                                return None;
                            }
                            let kind = ed.get("type").and_then(|v| v.as_str()).unwrap_or("reference").to_string();
                            Some(A3Edge { members, kind })
                        })
                        .collect()
                })
                .unwrap_or_default();
            Some(A3Req { ears, entities, evidence, edges })
        })
        .collect()
}

#[allow(dead_code)] // convenience wrapper that reads from disk; engine uses compile_text directly
pub fn compile_file(path: &Path, object_id: &str, llm: &Llm) -> Result<ObjectArtifact, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("read {}: {}", path.display(), e))?;
    compile_text(&text, path, object_id, llm, None)
}

pub fn compile_text(
    text: &str,
    path: &Path,
    object_id: &str,
    llm: &Llm,
    store: Option<&Store>,
) -> Result<ObjectArtifact, String> {
    let text = text.to_string();
    let chash = hash_hex(&text);
    let sections = md::parse_sections(&text);

    // A1: persist the section tree (deterministic, keyed by file content hash).
    if let Some(s) = store {
        s.put_stage(object_id, "sections", &chash, &SectionsArtifact { sections: sections.clone() });
    }

    // A2 + A5: extract entities with a local definition.
    let esys = "You extract domain entities from a software documentation file. An entity is any named concept: a component, a type, a field, a thing. Return ONLY a JSON object, no prose, no markdown fences. Shape: {\"entities\":[{\"name\":string,\"linkage\":\"internal\"|\"external\",\"role\":\"definition\"|\"reference\",\"definition\":string}]}. 'definition' is one short sentence describing the entity as this document presents it. 'role' is 'definition' if this document defines or specifies the entity, otherwise 'reference'. 'linkage' is 'external' if the entity is a shared concept other documents likely use, otherwise 'internal'.";
    let eschema = serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["entities"],
        "properties": {
            "entities": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["name", "linkage", "role", "definition"],
                    "properties": {
                        "name": {"type": "string"},
                        "linkage": {"type": "string", "enum": ["internal", "external"]},
                        "role": {"type": "string", "enum": ["definition", "reference"]},
                        "definition": {"type": "string"}
                    }
                }
            }
        }
    });
    // A2 + A5: cached by file content hash (plus model + prompt version, folded in by the store).
    let mut entities: BTreeMap<String, LocalEntity> =
        match store.and_then(|s| s.get_stage::<EntitiesArtifact>(object_id, "entities", &chash)) {
            Some(a) => a.entities,
            None => {
                let ev = llm.chat_json(esys, &text, "entities", &eschema, &format!("{} · A2 entities", object_id))?;
                let mut entities: BTreeMap<String, LocalEntity> = BTreeMap::new();
                if let Some(arr) = ev.get("entities").and_then(|v| v.as_array()) {
                    for (i, item) in arr.iter().enumerate() {
                        let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                        if name.is_empty() {
                            continue;
                        }
                        let lid = format!("e{}", i);
                        let linkage = item.get("linkage").and_then(|v| v.as_str()).unwrap_or("external");
                        let role = item.get("role").and_then(|v| v.as_str()).unwrap_or("reference");
                        let def = item.get("definition").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        entities.insert(
                            lid,
                            LocalEntity {
                                name,
                                aliases: vec![],
                                linkage: if linkage == "internal" { "internal".into() } else { "external".into() },
                                role: if role == "definition" { "definition".into() } else { "reference".into() },
                                local_definition: def,
                                confidence: 0.8,
                                provenance: vec![],
                            },
                        );
                    }
                }
                if let Some(s) = store {
                    s.put_stage(object_id, "entities", &chash, &EntitiesArtifact { entities: entities.clone() });
                }
                entities
            }
        };

    // Resolve entity names to local ids. Sorted by numeric id so a name reused across entities maps
    // deterministically (the highest-indexed entity wins, matching extraction order).
    let mut name_to_local: BTreeMap<String, String> = BTreeMap::new();
    let mut lids: Vec<&String> = entities.keys().collect();
    lids.sort_by_key(|k| k[1..].parse::<usize>().unwrap_or(0));
    for lid in lids {
        name_to_local.insert(entities[lid].name.to_lowercase(), lid.clone());
    }

    // A3: extract requirements per section, in parallel, cached per section. Each section is a
    // small, well-defined prompt (the small-prompt thesis applied to the compiler itself).
    let names: Vec<String> = entities.values().map(|e| e.name.clone()).collect();
    let entity_table = names.join(", ");
    let rsys = "You extract requirements from one section of a software documentation file, given the file's entities. A requirement is a single testable statement in EARS style (e.g. 'The system shall ...', 'When X, the system shall Y'). Return ONLY JSON, no prose, no fences. Shape: {\"requirements\":[{\"ears\":string,\"entities\":[string],\"evidence\":string,\"edges\":[{\"members\":[string,string],\"type\":string}]}]}. Extract only requirements stated in THIS section; return an empty array if it states none. 'entities' are names taken only from the provided list that the requirement is about. 'evidence' MUST be the exact, verbatim sentence or phrase copied character-for-character from the section that this requirement is based on (so it can be located in the text) — do not paraphrase it. 'edges' tie two of those entities together; 'type' is one of generalization, realization, composition, aggregation, association, dependency, reference (use reference if unsure). Use only names from the provided list.";
    let rschema = serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["requirements"],
        "properties": {
            "requirements": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["ears", "entities", "evidence", "edges"],
                    "properties": {
                        "ears": {"type": "string"},
                        "entities": {"type": "array", "items": {"type": "string"}},
                        "evidence": {"type": "string"},
                        "edges": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "additionalProperties": false,
                                "required": ["members", "type"],
                                "properties": {
                                    "members": {"type": "array", "items": {"type": "string"}},
                                    "type": {"type": "string", "enum": REL_TYPES}
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    // One job per section that has a body. Sorted by section reference for deterministic ids.
    struct SecJob {
        reference: String,
        body: String,
        key: String,
    }
    let mut sec_jobs: Vec<SecJob> = Vec::new();
    for (reference, sec) in &sections {
        if sec.raw.trim().is_empty() {
            continue;
        }
        // Per-section key: section text + entity table. Model and prompt version are folded in by the
        // store's coarse header (key ""), so they invalidate the whole file when they change.
        let key = hash_hex(&format!("{}\u{0}{}\u{0}{}", reference, entity_table, sec.raw));
        sec_jobs.push(SecJob { reference: reference.clone(), body: sec.raw.clone(), key });
    }

    // Prior requirements.yaml: reuse a section's result when its key still matches.
    let prior: BTreeMap<String, SectionReqs> = store
        .and_then(|s| s.get_stage::<RequirementsArtifact>(object_id, "requirements", ""))
        .map(|a| a.sections)
        .unwrap_or_default();

    let workers = crate::parallel::max_workers();
    let sec_results: Vec<Vec<A3Req>> = crate::parallel::par_map(&sec_jobs, workers, |_, j| {
        if let Some(sr) = prior.get(&j.reference) {
            if sr.key == j.key {
                return sr.requirements.clone();
            }
        }
        let user = format!("Entities: {}\n\nSection:\n{}", entity_table, j.body);
        match llm.chat_json(rsys, &user, "requirements", &rschema, &format!("{} · A3 §{}", object_id, j.reference)) {
            Ok(v) => parse_a3(&v),
            Err(_) => Vec::new(),
        }
    });

    // Persist all sections (keyed) back to requirements.yaml.
    if let Some(s) = store {
        let mut secmap: BTreeMap<String, SectionReqs> = BTreeMap::new();
        for (job, reqs) in sec_jobs.iter().zip(sec_results.iter()) {
            secmap.insert(
                job.reference.clone(),
                SectionReqs { key: job.key.clone(), requirements: reqs.clone() },
            );
        }
        s.put_stage(object_id, "requirements", "", &RequirementsArtifact { sections: secmap });
    }

    // Assemble requirements (sequential ids in section order), edges, and diagnostics.
    let mut requirements: Vec<Requirement> = Vec::new();
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    let mut edge_map: BTreeMap<(String, String), (String, Vec<String>)> = BTreeMap::new();
    let mut ridx = 0usize;
    for (job, reqs) in sec_jobs.iter().zip(sec_results) {
        for a3 in reqs {
            let ears = a3.ears.trim().to_string();
            if ears.is_empty() {
                continue;
            }
            let rid = format!("r{}", ridx);
            ridx += 1;
            let mut refs: Vec<String> = Vec::new();
            for n in &a3.entities {
                if let Some(lid) = name_to_local.get(&n.to_lowercase()) {
                    if !refs.contains(lid) {
                        refs.push(lid.clone());
                    }
                }
            }
            let mut implied: Vec<Edge> = Vec::new();
            for ed in &a3.edges {
                let mems: Vec<String> = ed
                    .members
                    .iter()
                    .filter_map(|n| name_to_local.get(&n.to_lowercase()).cloned())
                    .collect();
                if mems.len() >= 2 {
                    let ty = norm_type(&ed.kind);
                    let (x, y) = if mems[0] <= mems[1] {
                        (mems[0].clone(), mems[1].clone())
                    } else {
                        (mems[1].clone(), mems[0].clone())
                    };
                    if x == y {
                        continue;
                    }
                    implied.push(Edge { members: vec![x.clone(), y.clone()], kind: ty.clone() });
                    let ent = edge_map.entry((x, y)).or_insert((ty.clone(), vec![]));
                    if rel_rank(&ty) < rel_rank(&ent.0) {
                        ent.0 = ty.clone();
                    }
                    if !ent.1.contains(&rid) {
                        ent.1.push(rid.clone());
                    }
                }
            }
            if refs.is_empty() {
                diagnostics.push(Diagnostic {
                    id: format!("requirement-without-entity:{}:{}", object_id, rid),
                    rule: "requirement-without-entity".into(),
                    severity: "warning".into(),
                    subjects: vec![rid.clone()],
                    message: format!("requirement references no entity: {}", ears),
                    reasoning: None,
                });
            }
            requirements.push(Requirement {
                id: rid,
                ears_text: ears,
                entity_refs: refs,
                implied_edges: implied,
                // Per-section extraction means the source section is known exactly.
                source_section: job.reference.clone(),
                evidence: a3.evidence.trim().to_string(),
                confidence: 0.8,
            });
        }
    }

    // Provenance (deterministic): for each entity, which sections mention it.
    // The defining section is the first containing section, preferring the doc order.
    for (_lid, ent) in entities.iter_mut() {
        let mut prov: Vec<(usize, String)> = Vec::new();
        for (reference, sec) in &sections {
            let mut hit = !md::occurrences(&sec.raw, &ent.name).is_empty();
            if !hit {
                hit = ent.aliases.iter().any(|a| !md::occurrences(&sec.raw, a).is_empty());
            }
            if hit {
                prov.push((sec.start_line, reference.clone()));
            }
        }
        prov.sort();
        ent.provenance = prov.into_iter().map(|(_, r)| r).collect();
    }

    // A4: consolidate edges into relationships.
    let mut relationships: Vec<Relationship> = Vec::new();
    for (i, (key, val)) in edge_map.iter().enumerate() {
        relationships.push(Relationship {
            local_id: format!("rel{}", i),
            kind: val.0.clone(),
            members: vec![key.0.clone(), key.1.clone()],
            requirements: val.1.clone(),
        });
    }

    let externals: Vec<External> = entities
        .iter()
        .filter(|(_, e)| e.role == "reference")
        .map(|(lid, e)| External {
            local_id: lid.clone(),
            name: e.name.clone(),
            relocation: None,
        })
        .collect();

    Ok(ObjectArtifact {
        doc: DocMeta {
            source_file: format!("file://{}", path.display()),
            format: "markdown".into(),
            content_hash: chash,
        },
        sections,
        entities,
        requirements,
        relationships,
        externals,
        diagnostics,
    })
}

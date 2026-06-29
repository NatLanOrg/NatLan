// Language server over stdio. Thin protocol layer over the compiler engine, mirroring
// docs/lsp.md and docs/lsp/capabilities/*. Hand-rolled JSON-RPC (no async runtime).
use crate::cache::Store;
use crate::engine::{self, Build};
use crate::jsonrpc::{read_message, write_message};
use crate::llm::Llm;
use crate::md;
use crate::model::*;
use crate::project::Project;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::{self, BufReader, Write};
use std::path::PathBuf;

pub struct Server {
    proj: Project,
    cache: Store,
    llm: Llm,
    out_dir: PathBuf,
    overlay: BTreeMap<String, String>, // oid -> in-memory text
    build: Option<Build>,
    last_sig: Option<String>, // fingerprint of the inputs the current build was made from
}

impl Server {
    pub fn new(proj: Project, llm: Llm, out_dir: PathBuf) -> Server {
        let cache = Store::new(&out_dir, &llm.model);
        Server {
            proj,
            cache,
            llm,
            out_dir,
            overlay: BTreeMap::new(),
            build: None,
            last_sig: None,
        }
    }

    pub fn run(&mut self) {
        let stdin = io::stdin();
        let mut reader = BufReader::new(stdin.lock());
        let stdout = io::stdout();
        loop {
            let msg = match read_message(&mut reader) {
                Some(m) => m,
                None => break,
            };
            let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("").to_string();
            let id = msg.get("id").cloned();
            let params = msg.get("params").cloned().unwrap_or(Value::Null);
            let mut out = stdout.lock();
            match method.as_str() {
                "initialize" => {
                    self.on_initialize(&params);
                    reply(&mut out, id, self.capabilities());
                }
                "initialized" => self.rebuild_and_publish(&mut out),
                "shutdown" => reply(&mut out, id, Value::Null),
                "exit" => break,
                "textDocument/didOpen" => {
                    // Opening a file doesn't change content (overlay == disk), so this only
                    // rebuilds if something actually changed; either way we (re)publish so the
                    // newly opened file shows the current build's diagnostics.
                    self.on_did_open(&params);
                    if !self.maybe_rebuild_and_publish(&mut out) {
                        self.publish_all(&mut out);
                    }
                }
                "textDocument/didChange" => {
                    self.on_did_change(&params);
                    self.maybe_rebuild_and_publish(&mut out);
                }
                "textDocument/didSave" => {
                    self.maybe_rebuild_and_publish(&mut out);
                }
                "textDocument/didClose" => self.on_did_close(&params),
                "textDocument/definition" => {
                    let r = self.on_definition(&params);
                    reply(&mut out, id, r);
                }
                "textDocument/references" => {
                    let r = self.on_references(&params);
                    reply(&mut out, id, r);
                }
                "textDocument/hover" => {
                    let r = self.on_hover(&params);
                    reply(&mut out, id, r);
                }
                "textDocument/completion" => {
                    let r = self.on_completion(&params);
                    reply(&mut out, id, r);
                }
                "textDocument/semanticTokens/full" => {
                    let r = self.on_semantic_tokens(&params);
                    reply(&mut out, id, r);
                }
                _ => {
                    if id.is_some() {
                        reply(&mut out, id, Value::Null);
                    }
                }
            }
        }
    }

    fn capabilities(&self) -> Value {
        json!({
            "capabilities": {
                "textDocumentSync": 1, // full
                "definitionProvider": true,
                "referencesProvider": true,
                "hoverProvider": true,
                "completionProvider": { "triggerCharacters": ["`", "["] },
                "semanticTokensProvider": {
                    "legend": {
                        "tokenTypes": ["entity"],
                        "tokenModifiers": ["definition", "external", "unresolved"]
                    },
                    "full": true
                }
            },
            "serverInfo": { "name": "jazyk", "version": env!("CARGO_PKG_VERSION") }
        })
    }

    fn on_initialize(&mut self, params: &Value) {
        // Honor the workspace root if the client supplies one and it contains a project.
        if let Some(uri) = params.get("rootUri").and_then(|u| u.as_str()) {
            if let Some(path) = uri_to_path(uri) {
                if let Some(root) = crate::project::find_root(&path) {
                    let llm = self.llm.clone();
                    self.proj = Project::load(&root);
                    // Keep CLI LLM overrides (base_url/model already set on self.llm).
                    self.llm = llm;
                }
            }
        }
        self.log("jazyk language server initialized");
    }

    // Fingerprint of all current inputs (overlay text where open, else disk), so we can skip
    // rebuilds when nothing actually changed.
    fn inputs_sig(&self) -> String {
        let mut parts = Vec::new();
        for f in self.proj.doc_files() {
            let oid = engine::object_id(&self.proj, &f);
            let content = self
                .overlay
                .get(&oid)
                .cloned()
                .unwrap_or_else(|| std::fs::read_to_string(&f).unwrap_or_default());
            parts.push(format!("{}\u{0}{}", oid, content));
        }
        parts.join("\u{1}")
    }

    // Rebuild only if inputs changed since the last build; publishes progressively. Returns
    // true if it rebuilt.
    fn maybe_rebuild_and_publish<W: Write>(&mut self, out: &mut W) -> bool {
        let sig = self.inputs_sig();
        if self.last_sig.as_deref() == Some(sig.as_str()) {
            eprintln!("[jazyk-lsp] no document changes; skipping rebuild");
            return false;
        }
        self.rebuild_and_publish(out);
        true
    }

    // Two-phase build: compile files (parallel) and publish per-file diagnostics, then link and
    // publish the cross-file diagnostics — so feedback appears without waiting for linking.
    fn rebuild_and_publish<W: Write>(&mut self, out: &mut W) {
        self.last_sig = Some(self.inputs_sig());
        let n = self.proj.doc_files().len();
        eprintln!(
            "[jazyk-lsp] compiling {} file(s) with model {} at {} …",
            n, self.llm.model, self.llm.base_url
        );
        // Phase 1: compile and publish per-file diagnostics immediately.
        let objects = engine::compile_files(&self.proj, Some(&self.cache), &self.llm, &self.overlay, false);
        self.build = Some(Build {
            objects: objects.clone(),
            linked: LinkedArtifact::default(),
            reviewed: ReviewedArtifact::default(),
        });
        self.publish_all(out);
        eprintln!("[jazyk-lsp] compiled {} file(s); linking …", objects.len());

        // Phase 2: link and publish the cross-file diagnostics.
        let (linked, reviewed) = engine::link_objects(&self.proj, &objects, Some(&self.cache), &self.llm);
        let diag_count: usize =
            objects.iter().map(|(_, a)| a.diagnostics.len()).sum::<usize>() + reviewed.diagnostics.len();
        eprintln!(
            "[jazyk-lsp] build complete: {} entities, {} relationships, {} requirements, {} diagnostics",
            reviewed.entities.len(),
            reviewed.relationships.len(),
            reviewed.requirements.len(),
            diag_count
        );
        let build = Build { objects, linked, reviewed };
        // Persist the whole-program finals so a separate MCP process (or CI) reads the same completed
        // build the editor sees (see docs/lsp/lifecycle.md#persisted-output).
        crate::cli::write_artifacts(&build, &self.out_dir);
        self.build = Some(build);
        self.publish_all(out);
    }

    fn log(&self, msg: &str) {
        eprintln!("[jazyk-lsp] {}", msg);
    }

    // ---- file sync ----

    fn on_did_open(&mut self, params: &Value) {
        if let Some(td) = params.get("textDocument") {
            if let (Some(uri), Some(text)) =
                (td.get("uri").and_then(|u| u.as_str()), td.get("text").and_then(|t| t.as_str()))
            {
                if let Some(oid) = self.uri_to_oid(uri) {
                    self.overlay.insert(oid, text.to_string());
                }
            }
        }
    }

    fn on_did_change(&mut self, params: &Value) {
        let uri = params.get("textDocument").and_then(|t| t.get("uri")).and_then(|u| u.as_str());
        // Full sync: take the last content change's full text.
        let text = params
            .get("contentChanges")
            .and_then(|c| c.as_array())
            .and_then(|a| a.last())
            .and_then(|c| c.get("text"))
            .and_then(|t| t.as_str());
        if let (Some(uri), Some(text)) = (uri, text) {
            if let Some(oid) = self.uri_to_oid(uri) {
                self.overlay.insert(oid, text.to_string());
            }
        }
    }

    fn on_did_close(&mut self, params: &Value) {
        if let Some(uri) = params.get("textDocument").and_then(|t| t.get("uri")).and_then(|u| u.as_str()) {
            if let Some(oid) = self.uri_to_oid(uri) {
                self.overlay.remove(&oid);
            }
        }
    }

    // ---- uri/oid helpers ----

    fn uri_to_oid(&self, uri: &str) -> Option<String> {
        let path = uri_to_path(uri)?;
        Some(engine::object_id(&self.proj, &path))
    }

    fn oid_to_uri(&self, oid: &str) -> String {
        let abs = Build::abs_path(&self.proj, oid);
        path_to_uri(&abs)
    }

    fn file_text(&self, oid: &str) -> String {
        if let Some(t) = self.overlay.get(oid) {
            return t.clone();
        }
        std::fs::read_to_string(Build::abs_path(&self.proj, oid)).unwrap_or_default()
    }

    // ---- graph helpers ----

    // Global entity whose name/alias occurrence in `oid` covers (line, character).
    fn entity_at(&self, oid: &str, line: usize, character: usize) -> Option<GlobalEntity> {
        let build = self.build.as_ref()?;
        let text = self.file_text(oid);
        let mut best: Option<(usize, GlobalEntity)> = None; // (name length, entity)
        for ge in &build.reviewed.entities {
            // Only consider entities that have a member in this file.
            if !ge.members.iter().any(|m| m.object == oid) {
                continue;
            }
            let mut names = vec![ge.canonical_name.clone()];
            names.extend(ge.aliases.iter().cloned());
            for n in names {
                for (l, c, len) in md::occurrences(&text, &n) {
                    if l == line && character >= c && character < c + len {
                        // Prefer the longest matching name.
                        if best.as_ref().map(|(bl, _)| len > *bl).unwrap_or(true) {
                            best = Some((len, ge.clone()));
                        }
                    }
                }
            }
        }
        best.map(|(_, e)| e)
    }

    fn object<'a>(&self, build: &'a Build, oid: &str) -> Option<&'a ObjectArtifact> {
        build.objects.iter().find(|(o, _)| o == oid).map(|(_, a)| a)
    }

    // The (oid, line) where an entity is defined.
    fn entity_def(&self, build: &Build, ge: &GlobalEntity) -> Option<(String, usize)> {
        let member = ge
            .members
            .iter()
            .find(|m| m.role.as_deref() == Some("definition"))
            .or_else(|| ge.members.first())?;
        let art = self.object(build, &member.object)?;
        let le = art.entities.get(&member.local_id)?;
        let section = le.provenance.first()?;
        let line = md::section_line(&art.sections, section);
        Some((member.object.clone(), line))
    }

    fn loc(&self, oid: &str, line: usize) -> Value {
        json!({
            "uri": self.oid_to_uri(oid),
            "range": { "start": {"line": line, "character": 0}, "end": {"line": line, "character": 0} }
        })
    }

    // ---- request handlers ----

    fn on_definition(&self, params: &Value) -> Value {
        let (oid, line, ch) = match self.pos(params) {
            Some(v) => v,
            None => return Value::Null,
        };
        let build = match &self.build {
            Some(b) => b,
            None => return Value::Null,
        };
        if let Some(ge) = self.entity_at(&oid, line, ch) {
            if let Some((doid, dline)) = self.entity_def(build, &ge) {
                return self.loc(&doid, dline);
            }
        }
        Value::Null
    }

    fn on_references(&self, params: &Value) -> Value {
        let (oid, line, ch) = match self.pos(params) {
            Some(v) => v,
            None => return json!([]),
        };
        let build = match &self.build {
            Some(b) => b,
            None => return json!([]),
        };
        let ge = match self.entity_at(&oid, line, ch) {
            Some(e) => e,
            None => return json!([]),
        };
        let mut locs: Vec<Value> = Vec::new();
        let mut seen: std::collections::BTreeSet<(String, usize)> = std::collections::BTreeSet::new();
        for m in &ge.members {
            if let Some(art) = self.object(build, &m.object) {
                if let Some(le) = art.entities.get(&m.local_id) {
                    for sec in &le.provenance {
                        let l = md::section_line(&art.sections, sec);
                        if seen.insert((m.object.clone(), l)) {
                            locs.push(self.loc(&m.object, l));
                        }
                    }
                }
            }
        }
        json!(locs)
    }

    fn on_hover(&self, params: &Value) -> Value {
        let (oid, line, ch) = match self.pos(params) {
            Some(v) => v,
            None => return Value::Null,
        };
        let build = match &self.build {
            Some(b) => b,
            None => return Value::Null,
        };
        let ge = match self.entity_at(&oid, line, ch) {
            Some(e) => e,
            None => return Value::Null,
        };
        let mut md_text = format!("**{}**", ge.canonical_name);
        if let Some(s) = &ge.scope {
            md_text.push_str(&format!("  \n*scope:* {}", s));
        }
        let def = ge.global_definition.clone().or_else(|| {
            ge.members.first().and_then(|m| {
                self.object(build, &m.object)
                    .and_then(|a| a.entities.get(&m.local_id))
                    .map(|le| le.local_definition.clone())
            })
        });
        if let Some(d) = def {
            if !d.is_empty() {
                md_text.push_str(&format!("\n\n{}", d));
            }
        }
        // Relationships.
        let rels: Vec<String> = build
            .reviewed
            .relationships
            .iter()
            .filter(|r| r.members.contains(&ge.global_id))
            .map(|r| {
                let other = r.members.iter().find(|m| *m != &ge.global_id).cloned().unwrap_or_default();
                format!("- {} → {}", r.kind, other.trim_start_matches("ent:"))
            })
            .collect();
        if !rels.is_empty() {
            md_text.push_str(&format!("\n\n**Relationships**\n{}", rels.join("\n")));
        }
        // Requirements.
        let reqs: Vec<String> = build
            .reviewed
            .requirements
            .iter()
            .filter(|r| r.entities.contains(&ge.global_id))
            .map(|r| format!("- {}", r.ears_text))
            .collect();
        if !reqs.is_empty() {
            md_text.push_str(&format!("\n\n**Requirements**\n{}", reqs.join("\n")));
        }
        // Diagnostics.
        let diags: Vec<String> = build
            .reviewed
            .diagnostics
            .iter()
            .filter(|d| d.subjects.contains(&ge.global_id))
            .map(|d| format!("- ⚠ {} — {}", d.rule, d.message))
            .collect();
        if !diags.is_empty() {
            md_text.push_str(&format!("\n\n**Diagnostics**\n{}", diags.join("\n")));
        }
        json!({ "contents": { "kind": "markdown", "value": md_text } })
    }

    fn on_completion(&self, _params: &Value) -> Value {
        let build = match &self.build {
            Some(b) => b,
            None => return json!({ "isIncomplete": false, "items": [] }),
        };
        let mut items: Vec<Value> = Vec::new();
        let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for (oid, art) in &build.objects {
            for le in art.entities.values() {
                if le.linkage != "external" {
                    continue;
                }
                if !seen.insert(le.name.clone()) {
                    continue;
                }
                items.push(json!({
                    "label": le.name,
                    "kind": 6, // Variable
                    "detail": le.local_definition,
                    "documentation": format!("defined in {}", oid),
                    "insertText": le.name
                }));
            }
        }
        json!({ "isIncomplete": false, "items": items })
    }

    // Semantic tokens: color every entity-name occurrence in the file, so the spans that are
    // command-clickable (see `entity_at`) are visible. Modifiers carry three facets: `definition`
    // (this file defines the entity), `external` (shared concept), `unresolved` (a reference with
    // nothing defining it anywhere). Mirrors the enumeration `entity_at` does, over the whole file.
    fn on_semantic_tokens(&self, params: &Value) -> Value {
        let oid = match params
            .get("textDocument")
            .and_then(|t| t.get("uri"))
            .and_then(|u| u.as_str())
            .and_then(|u| self.uri_to_oid(u))
        {
            Some(o) => o,
            None => return json!({ "data": [] }),
        };
        let build = match &self.build {
            Some(b) => b,
            None => return json!({ "data": [] }),
        };
        let text = self.file_text(&oid);
        let art = self.object(build, &oid);

        // Candidate tokens: (line, col, len, modifier_bitset). Bits match the legend order:
        // definition = 1<<0, external = 1<<1, unresolved = 1<<2.
        let mut toks: Vec<(usize, usize, usize, u32)> = Vec::new();
        for ge in &build.reviewed.entities {
            let member = match ge.members.iter().find(|m| m.object == oid) {
                Some(m) => m,
                None => continue,
            };
            let mut mods: u32 = 0;
            if member.role.as_deref() == Some("definition") {
                mods |= 1 << 0;
            }
            if let Some(le) = art.and_then(|a| a.entities.get(&member.local_id)) {
                if le.linkage == "external" {
                    mods |= 1 << 1;
                }
            }
            if !ge.members.iter().any(|m| m.role.as_deref() == Some("definition")) {
                mods |= 1 << 2;
            }
            let mut names = vec![ge.canonical_name.clone()];
            names.extend(ge.aliases.iter().cloned());
            for n in names {
                for (l, c, len) in md::occurrences(&text, &n) {
                    toks.push((l, c, len, mods));
                }
            }
        }

        json!({ "data": encode_semantic_tokens(toks) })
    }

    fn pos(&self, params: &Value) -> Option<(String, usize, usize)> {
        let uri = params.get("textDocument")?.get("uri")?.as_str()?;
        let oid = self.uri_to_oid(uri)?;
        let p = params.get("position")?;
        let line = p.get("line")?.as_u64()? as usize;
        let ch = p.get("character")?.as_u64()? as usize;
        Some((oid, line, ch))
    }

    // ---- diagnostics ----

    // Range of a requirement: its verbatim evidence snippet, else its source-section heading.
    fn req_range(&self, art: &ObjectArtifact, text: &str, req: &Requirement) -> (usize, usize, usize, usize) {
        if !req.evidence.is_empty() {
            if let Some(r) = md::locate(text, &req.evidence) {
                return r;
            }
        }
        let line = md::section_line(&art.sections, &req.source_section);
        (line, 0, line, 0)
    }

    // Range of an entity in one file: its first whole-word name occurrence, then a substring
    // fallback (handles plural/morphology like "Products"), then its defining section heading.
    fn entity_range(&self, art: &ObjectArtifact, text: &str, local_id: &str) -> (usize, usize, usize, usize) {
        if let Some(le) = art.entities.get(local_id) {
            if let Some((l, c, len)) = md::occurrences(text, &le.name).into_iter().next() {
                return (l, c, l, c + len);
            }
            if let Some((sl, sc, _, _)) = md::locate(text, &le.name) {
                let len = le.name.chars().count();
                return (sl, sc, sl, sc + len);
            }
            if let Some(sec_ref) = le.provenance.first() {
                let line = md::section_line(&art.sections, sec_ref);
                return (line, 0, line, 0);
            }
        }
        (0, 0, 0, 0)
    }

    fn publish_all<W: Write>(&self, out: &mut W) {
        let build = match &self.build {
            Some(b) => b,
            None => return,
        };
        // file oid -> diagnostics
        let mut by_file: BTreeMap<String, Vec<Value>> = BTreeMap::new();
        // Ensure every known file gets a (possibly empty) publish so resolved diagnostics clear.
        for (oid, _) in &build.objects {
            by_file.entry(oid.clone()).or_default();
        }
        for oid in self.overlay.keys() {
            by_file.entry(oid.clone()).or_default();
        }

        // Per-object diagnostics (A2/A3): subjects are local requirement ids. Anchor to the
        // requirement's verbatim evidence snippet where available.
        for (oid, art) in &build.objects {
            let text = self.file_text(oid);
            for d in &art.diagnostics {
                let range = d
                    .subjects
                    .iter()
                    .find_map(|s| art.requirements.iter().find(|r| &r.id == s))
                    .map(|r| self.req_range(art, &text, r))
                    .unwrap_or((0, 0, 0, 0));
                by_file.entry(oid.clone()).or_default().push(lsp_diag(d, range));
            }
        }

        // Global diagnostics: subjects are entity ids; anchor to the entity-name occurrence in
        // each member file.
        for d in &build.reviewed.diagnostics {
            let mut placed = false;
            for subj in &d.subjects {
                for ge in build.reviewed.entities.iter().filter(|e| &e.global_id == subj) {
                    for m in &ge.members {
                        if let Some(art) = self.object(build, &m.object) {
                            let text = self.file_text(&m.object);
                            let range = self.entity_range(art, &text, &m.local_id);
                            by_file.entry(m.object.clone()).or_default().push(lsp_diag(d, range));
                            placed = true;
                        }
                    }
                }
            }
            // Unplaceable diagnostics go to the first known file at line 0.
            if !placed {
                if let Some((oid, _)) = build.objects.first() {
                    by_file.entry(oid.clone()).or_default().push(lsp_diag(d, (0, 0, 0, 0)));
                }
            }
        }

        for (oid, diags) in by_file {
            let msg = json!({
                "jsonrpc": "2.0",
                "method": "textDocument/publishDiagnostics",
                "params": { "uri": self.oid_to_uri(&oid), "diagnostics": diags }
            });
            write_message(out, &msg);
        }
    }
}

// Turn candidate entity spans (line, col, len, modifier_bitset) into the flat, delta-encoded
// semantic-tokens array the LSP spec expects. Overlapping spans collapse to the longest match at a
// given start (mirroring `entity_at`), since semantic tokens may not overlap.
fn encode_semantic_tokens(mut toks: Vec<(usize, usize, usize, u32)>) -> Vec<u32> {
    // Order by position, longest first at a given start so the kept span is the longest one.
    toks.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(b.2.cmp(&a.2)));
    let mut kept: Vec<(usize, usize, usize, u32)> = Vec::new();
    for t in toks {
        if let Some(last) = kept.last() {
            // Skip anything starting inside the previously kept token on the same line.
            if last.0 == t.0 && t.1 < last.1 + last.2 {
                continue;
            }
        }
        kept.push(t);
    }
    // Delta-encode: [deltaLine, deltaStartChar, length, tokenType, modifiers].
    let mut data: Vec<u32> = Vec::new();
    let (mut prev_line, mut prev_col) = (0usize, 0usize);
    for (l, c, len, mods) in kept {
        let dl = l - prev_line;
        let dc = if dl == 0 { c - prev_col } else { c };
        data.extend_from_slice(&[dl as u32, dc as u32, len as u32, 0, mods]);
        prev_line = l;
        prev_col = c;
    }
    data
}

fn lsp_diag(d: &Diagnostic, range: (usize, usize, usize, usize)) -> Value {
    let severity = match d.severity.as_str() {
        "error" => 1,
        "warning" => 2,
        "info" => 3,
        _ => 4,
    };
    let mut message = d.message.clone();
    if let Some(r) = &d.reasoning {
        if !r.is_empty() {
            message.push_str(&format!("\n\n{}", r));
        }
    }
    let (sl, sc, el, ec) = range;
    json!({
        "range": { "start": {"line": sl, "character": sc}, "end": {"line": el, "character": ec} },
        "severity": severity,
        "source": "jazyk",
        "code": d.rule,
        "message": message
    })
}

fn reply<W: Write>(out: &mut W, id: Option<Value>, result: Value) {
    let msg = json!({ "jsonrpc": "2.0", "id": id.unwrap_or(Value::Null), "result": result });
    write_message(out, &msg);
}

// file:// URI -> path (handles the common file:///abs/path form).
fn uri_to_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let decoded = percent_decode(rest);
    Some(PathBuf::from(decoded))
}

fn path_to_uri(path: &std::path::Path) -> String {
    let s = path.to_string_lossy().replace('\\', "/");
    if s.starts_with('/') {
        format!("file://{}", s)
    } else {
        format!("file:///{}", s)
    }
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

#[cfg(test)]
mod tests {
    use super::encode_semantic_tokens;

    #[test]
    fn delta_encodes_in_position_order() {
        // Two entities on different lines; input order is reversed to prove sorting.
        let toks = vec![(2, 4, 5, 0b010), (0, 2, 8, 0b001)];
        let data = encode_semantic_tokens(toks);
        assert_eq!(
            data,
            vec![
                // line 0, col 2, len 8, type 0, mods 1
                0, 2, 8, 0, 0b001, //
                // line 2 (delta 2), col 4 (absolute, new line), len 5, type 0, mods 2
                2, 4, 5, 0, 0b010,
            ]
        );
    }

    #[test]
    fn same_line_uses_delta_column() {
        let toks = vec![(0, 0, 3, 0), (0, 10, 4, 0)];
        let data = encode_semantic_tokens(toks);
        // second token's deltaStartChar is 10 - 0 = 10.
        assert_eq!(data, vec![0, 0, 3, 0, 0, 0, 10, 4, 0, 0]);
    }

    #[test]
    fn overlap_keeps_longest_match() {
        // "Product" (col 5, len 7) overlaps "Product ID" (col 5, len 10) at the same start.
        let toks = vec![(0, 5, 7, 0), (0, 5, 10, 0)];
        let data = encode_semantic_tokens(toks);
        // Only the longer span survives.
        assert_eq!(data, vec![0, 5, 10, 0, 0]);
    }

    #[test]
    fn overlap_skips_token_starting_inside_previous() {
        // A long span (col 2, len 10 -> covers 2..12) and a later one starting inside it (col 8).
        let toks = vec![(0, 2, 10, 0), (0, 8, 4, 0)];
        let data = encode_semantic_tokens(toks);
        assert_eq!(data, vec![0, 2, 10, 0, 0]);
    }

    #[test]
    fn adjacent_non_overlapping_tokens_both_kept() {
        // col 2..5 and col 5..9 touch but do not overlap.
        let toks = vec![(0, 2, 3, 0), (0, 5, 4, 0)];
        let data = encode_semantic_tokens(toks);
        assert_eq!(data, vec![0, 2, 3, 0, 0, 0, 3, 4, 0, 0]);
    }
}

// MCP server over stdio (newline-delimited JSON-RPC). Exposes the compiler and the build
// graph to agents. Mirrors docs/mcp.md.
use crate::cache::Store;
use crate::engine::{self, Build};
use crate::llm::Llm;
use crate::model::{LinkedArtifact, ReviewedArtifact};
use crate::project::Project;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

const NO_BUILD: &str =
    "no build found in the out directory; run the `compile` tool first (or `jazyk build`)";

pub struct Mcp {
    proj: Project,
    cache: Store,
    llm: Llm,
    out_dir: PathBuf,
    build: Option<Build>,
}

impl Mcp {
    pub fn new(proj: Project, llm: Llm, out_dir: PathBuf) -> Mcp {
        let cache = Store::new(&out_dir, &llm.model);
        Mcp {
            proj,
            cache,
            llm,
            out_dir,
            build: None,
        }
    }

    pub fn run(&mut self) {
        let stdin = io::stdin();
        let stdout = io::stdout();
        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };
            if line.trim().is_empty() {
                continue;
            }
            let msg: Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
            let id = msg.get("id").cloned();
            let params = msg.get("params").cloned().unwrap_or(Value::Null);
            let mut out = stdout.lock();
            match method {
                "initialize" => self.send(&mut out, id, self.initialize()),
                "notifications/initialized" | "initialized" => {}
                "tools/list" => self.send(&mut out, id, json!({ "tools": tools() })),
                "tools/call" => {
                    let res = self.tools_call(&params);
                    self.send(&mut out, id, res);
                }
                "ping" => self.send(&mut out, id, json!({})),
                _ => {
                    if id.is_some() {
                        self.send_err(&mut out, id, -32601, "method not found");
                    }
                }
            }
        }
    }

    fn initialize(&self) -> Value {
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "jazyk", "version": env!("CARGO_PKG_VERSION") },
            "instructions": format!(
                "Jazyk MCP server for project {}. Compile the project, then navigate the entity graph: get_entity, requirements_for, relationships_for, search, diagnostics.",
                self.proj.root.display()
            )
        })
    }

    // Read the latest persisted build (linked.yaml + reviewed.yaml) from the out directory. Exploration
    // and diagnostics tools use this; they never recompile (see docs/mcp.md). `objects` is left empty
    // because the whole-program globals are all these tools read. Returns None if no build is on disk.
    fn load_persisted(&self) -> Option<Build> {
        let fmt = crate::serialize::DEFAULT;
        let linked: LinkedArtifact = fmt
            .from_str(&std::fs::read_to_string(self.out_dir.join("linked.yaml")).ok()?)
            .ok()?;
        let reviewed: ReviewedArtifact = fmt
            .from_str(&std::fs::read_to_string(self.out_dir.join("reviewed.yaml")).ok()?)
            .ok()?;
        Some(Build {
            objects: Vec::new(),
            linked,
            reviewed,
        })
    }

    // The build to serve exploration/diagnostics from: the one compiled this session if present,
    // otherwise the latest persisted build loaded from disk. None means nothing has been built yet.
    fn loaded(&mut self) -> Option<&Build> {
        if self.build.is_none() {
            self.build = self.load_persisted();
        }
        self.build.as_ref()
    }

    fn tools_call(&mut self, params: &Value) -> Value {
        let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
        let args = params.get("arguments").cloned().unwrap_or(Value::Null);
        let text = match name {
            "compile" => {
                let build = engine::compile_project(
                    &self.proj,
                    Some(&self.cache),
                    &self.llm,
                    &BTreeMap::new(),
                    false,
                );
                // Persist the whole-program artifacts so exploration tools (and the next session) can
                // read this as the latest build without recompiling.
                crate::cli::write_artifacts(&build, &self.out_dir);
                self.build = Some(build);
                let b = self.build.as_ref().unwrap();
                let errors = b.reviewed.diagnostics.iter().filter(|d| d.severity == "error").count();
                let warnings = b.reviewed.diagnostics.iter().filter(|d| d.severity == "warning").count();
                format!(
                    "Compiled {} files: {} entities, {} relationships, {} requirements. {} error(s), {} warning(s).",
                    b.objects.len(),
                    b.reviewed.entities.len(),
                    b.reviewed.relationships.len(),
                    b.reviewed.requirements.len(),
                    errors,
                    warnings
                )
            }
            "diagnostics" => {
                let b = match self.loaded() {
                    Some(b) => b,
                    None => return json!({ "content": [{ "type": "text", "text": NO_BUILD }] }),
                };
                let filter = args.get("entity").and_then(|e| e.as_str());
                let diags: Vec<Value> = b
                    .reviewed
                    .diagnostics
                    .iter()
                    .filter(|d| filter.map(|f| d.subjects.iter().any(|s| s.contains(f))).unwrap_or(true))
                    .map(|d| json!({"rule": d.rule, "severity": d.severity, "subjects": d.subjects, "message": d.message}))
                    .collect();
                serde_json::to_string_pretty(&diags).unwrap_or_default()
            }
            "get_entity" => {
                let b = match self.loaded() {
                    Some(b) => b,
                    None => return json!({ "content": [{ "type": "text", "text": NO_BUILD }] }),
                };
                let q = args.get("name").and_then(|n| n.as_str()).unwrap_or("");
                match find_entity(b, q) {
                    Some(ge) => {
                        let rels: Vec<Value> = b
                            .reviewed
                            .relationships
                            .iter()
                            .filter(|r| r.members.contains(&ge.global_id))
                            .map(|r| json!({"type": r.kind, "members": r.members}))
                            .collect();
                        let reqs: Vec<String> = b
                            .reviewed
                            .requirements
                            .iter()
                            .filter(|r| r.entities.contains(&ge.global_id))
                            .map(|r| r.ears_text.clone())
                            .collect();
                        serde_json::to_string_pretty(&json!({
                            "id": ge.global_id,
                            "name": ge.canonical_name,
                            "definition": ge.global_definition,
                            "relationships": rels,
                            "requirements": reqs
                        }))
                        .unwrap_or_default()
                    }
                    None => format!("no entity matching '{}'", q),
                }
            }
            "requirements_for" => {
                let b = match self.loaded() {
                    Some(b) => b,
                    None => return json!({ "content": [{ "type": "text", "text": NO_BUILD }] }),
                };
                let a = args.get("entity").and_then(|n| n.as_str()).unwrap_or("");
                let other = args.get("other").and_then(|n| n.as_str());
                let ea = find_entity(b, a).map(|e| e.global_id.clone());
                let eb = other.and_then(|o| find_entity(b, o)).map(|e| e.global_id.clone());
                let reqs: Vec<String> = b
                    .reviewed
                    .requirements
                    .iter()
                    .filter(|r| {
                        let has_a = ea.as_ref().map(|x| r.entities.contains(x)).unwrap_or(false);
                        let has_b = eb.as_ref().map(|x| r.entities.contains(x)).unwrap_or(true);
                        has_a && has_b
                    })
                    .map(|r| r.ears_text.clone())
                    .collect();
                serde_json::to_string_pretty(&reqs).unwrap_or_default()
            }
            "relationships_for" => {
                let b = match self.loaded() {
                    Some(b) => b,
                    None => return json!({ "content": [{ "type": "text", "text": NO_BUILD }] }),
                };
                let a = args.get("entity").and_then(|n| n.as_str()).unwrap_or("");
                match find_entity(b, a) {
                    Some(ge) => {
                        let rels: Vec<Value> = b
                            .reviewed
                            .relationships
                            .iter()
                            .filter(|r| r.members.contains(&ge.global_id))
                            .map(|r| json!({"type": r.kind, "members": r.members, "requirements": r.requirements}))
                            .collect();
                        serde_json::to_string_pretty(&rels).unwrap_or_default()
                    }
                    None => format!("no entity matching '{}'", a),
                }
            }
            "search" => {
                let b = match self.loaded() {
                    Some(b) => b,
                    None => return json!({ "content": [{ "type": "text", "text": NO_BUILD }] }),
                };
                let q = args.get("query").and_then(|n| n.as_str()).unwrap_or("").to_lowercase();
                let hits: Vec<Value> = b
                    .reviewed
                    .entities
                    .iter()
                    .filter(|e| {
                        e.canonical_name.to_lowercase().contains(&q)
                            || e.global_definition.as_deref().unwrap_or("").to_lowercase().contains(&q)
                    })
                    .map(|e| json!({"id": e.global_id, "name": e.canonical_name, "definition": e.global_definition}))
                    .collect();
                serde_json::to_string_pretty(&hits).unwrap_or_default()
            }
            _ => format!("unknown tool: {}", name),
        };
        json!({ "content": [{ "type": "text", "text": text }] })
    }

    fn send<W: Write>(&self, out: &mut W, id: Option<Value>, result: Value) {
        let msg = json!({ "jsonrpc": "2.0", "id": id.unwrap_or(Value::Null), "result": result });
        let _ = writeln!(out, "{}", msg);
        let _ = out.flush();
    }

    fn send_err<W: Write>(&self, out: &mut W, id: Option<Value>, code: i64, message: &str) {
        let msg = json!({ "jsonrpc": "2.0", "id": id.unwrap_or(Value::Null), "error": {"code": code, "message": message} });
        let _ = writeln!(out, "{}", msg);
        let _ = out.flush();
    }
}

use crate::model::GlobalEntity;

fn find_entity<'a>(b: &'a Build, query: &str) -> Option<&'a GlobalEntity> {
    let q = query.to_lowercase();
    b.reviewed
        .entities
        .iter()
        .find(|e| e.global_id == query || e.canonical_name.to_lowercase() == q)
        .or_else(|| {
            b.reviewed
                .entities
                .iter()
                .find(|e| e.canonical_name.to_lowercase().contains(&q))
        })
}

fn tools() -> Vec<Value> {
    vec![
        json!({
            "name": "compile",
            "description": "Compile and link the project; returns a build summary with error/warning counts.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "diagnostics",
            "description": "List diagnostics from the latest build, optionally filtered to an entity id/name substring.",
            "inputSchema": { "type": "object", "properties": { "entity": {"type": "string"} } }
        }),
        json!({
            "name": "get_entity",
            "description": "Fetch an entity: its definition, requirements, and relationships.",
            "inputSchema": { "type": "object", "properties": { "name": {"type": "string"} }, "required": ["name"] }
        }),
        json!({
            "name": "requirements_for",
            "description": "List requirements for an entity, or between two entities (pass 'other').",
            "inputSchema": { "type": "object", "properties": { "entity": {"type": "string"}, "other": {"type": "string"} }, "required": ["entity"] }
        }),
        json!({
            "name": "relationships_for",
            "description": "List the relationships an entity participates in.",
            "inputSchema": { "type": "object", "properties": { "entity": {"type": "string"} }, "required": ["entity"] }
        }),
        json!({
            "name": "search",
            "description": "Find entities whose name or definition contains the query.",
            "inputSchema": { "type": "object", "properties": { "query": {"type": "string"} }, "required": ["query"] }
        }),
    ]
}

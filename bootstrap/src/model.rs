// Build artifact data model. Mirrors docs/compiler/model and docs/compiler/artifacts.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct DocMeta {
    pub source_file: String,
    pub format: String,
    pub content_hash: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct SectionBody {
    pub title: String,
    pub kind: String,
    pub order: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    pub raw: String,
    // 0-based inclusive start line, exclusive end line (for LSP range mapping).
    #[serde(default)]
    pub start_line: usize,
    #[serde(default)]
    pub end_line: usize,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct LocalEntity {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    pub linkage: String,
    pub role: String,
    pub local_definition: String,
    pub confidence: f64,
    // Section references in this doc where the entity appears (first is the defining section).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provenance: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Edge {
    pub members: Vec<String>,
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Requirement {
    pub id: String,
    pub ears_text: String,
    pub entity_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub implied_edges: Vec<Edge>,
    pub source_section: String,
    // Verbatim snippet of the document this requirement was extracted from (LLM-chosen),
    // used to anchor diagnostics to the exact text rather than the section heading.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub evidence: String,
    pub confidence: f64,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Relationship {
    pub local_id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub members: Vec<String>,
    pub requirements: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct External {
    pub local_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relocation: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub id: String,
    pub rule: String,
    pub severity: String,
    pub subjects: Vec<String>,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ObjectArtifact {
    pub doc: DocMeta,
    pub sections: BTreeMap<String, SectionBody>,
    pub entities: BTreeMap<String, LocalEntity>,
    pub requirements: Vec<Requirement>,
    pub relationships: Vec<Relationship>,
    pub externals: Vec<External>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Member {
    pub object: String,
    pub local_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GlobalEntity {
    pub global_id: String,
    pub canonical_name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    pub members: Vec<Member>,
    pub resolved_by: String,
    pub confidence: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub global_definition: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GlobalRel {
    pub global_id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub members: Vec<String>,
    pub requirements: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GlobalReq {
    pub global_id: String,
    pub ears_text: String,
    pub entities: Vec<String>,
    pub source_section: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct LinkedArtifact {
    pub entities: Vec<GlobalEntity>,
    pub relationships: Vec<GlobalRel>,
    pub requirements: Vec<GlobalReq>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Coverage {
    pub entity: String,
    pub tests_derivable: usize,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ReviewedArtifact {
    pub entities: Vec<GlobalEntity>,
    pub relationships: Vec<GlobalRel>,
    pub requirements: Vec<GlobalReq>,
    pub coverage: Vec<Coverage>,
    pub diagnostics: Vec<Diagnostic>,
}

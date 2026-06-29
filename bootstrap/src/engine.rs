// The build engine: discover docs, compile each file (cached), then link. Shared by every
// frontend (CLI build/check/watch, LSP, MCP).
use crate::cache::Store;
use crate::llm::Llm;
use crate::model::*;
use crate::project::Project;
use crate::{compile, link};
use std::collections::BTreeSet;
use std::path::PathBuf;

pub struct Build {
    pub objects: Vec<(String, ObjectArtifact)>,
    pub linked: LinkedArtifact,
    pub reviewed: ReviewedArtifact,
}

impl Build {
    // Absolute path of an object id (relative posix path) within the project.
    pub fn abs_path(proj: &Project, object_id: &str) -> PathBuf {
        proj.root.join(object_id)
    }

    // True if any diagnostic in the reviewed artifact is an error.
    pub fn has_error(&self) -> bool {
        self.reviewed.diagnostics.iter().any(|d| d.severity == "error")
    }
}

// Relative posix object id for a file within the project root.
pub fn object_id(proj: &Project, path: &std::path::Path) -> String {
    path.strip_prefix(&proj.root)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| path.to_string_lossy().to_string())
}

// Compile every documentation file (in parallel, cached). `overlay` provides in-memory contents
// for files the editor has open (path -> text), overriding disk; used by the LSP.
pub fn compile_files(
    proj: &Project,
    store: Option<&Store>,
    llm: &Llm,
    overlay: &std::collections::BTreeMap<String, String>,
    verbose: bool,
) -> Vec<(String, ObjectArtifact)> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    // Turn on per-call LLM logging when asked. We only ever enable it here (never force it off), so a
    // frontend that passes `false` still honors a JAZYK_VERBOSE env opt-in (e.g. the LSP).
    if verbose {
        crate::llm::set_verbose(true);
    }
    let files = proj.doc_files();
    let total = files.len();
    let workers = crate::parallel::max_workers();
    let done = AtomicUsize::new(0);
    let results = crate::parallel::par_map(&files, workers, |_, f| {
        let oid = object_id(proj, f);
        let content = overlay
            .get(&oid)
            .cloned()
            .unwrap_or_else(|| std::fs::read_to_string(f).unwrap_or_default());
        // Progress logging (parallel, so lines interleave; the (i/N) counter is completion order).
        // Fast path: object.yaml keyed by the file content hash skips compiling an unchanged file.
        let chash = compile::hash_hex(&content);
        let cached = store.and_then(|s| s.get_stage::<ObjectArtifact>(&oid, "object", &chash));
        let result = match cached {
            Some(a) => Ok((a, true)),
            None => match compile::compile_text(&content, f, &oid, llm, store) {
                Ok(a) => {
                    if let Some(s) = store {
                        s.put_stage(&oid, "object", &chash, &a);
                    }
                    Ok((a, false))
                }
                Err(e) => Err(e),
            },
        };
        let i = done.fetch_add(1, Ordering::Relaxed) + 1;
        match result {
            Ok((a, was_cached)) => {
                eprintln!(
                    "  [{:>3}/{}] {} {} — {} entities, {} requirements, {} relationships{}",
                    i,
                    total,
                    if was_cached { "·" } else { "✓" },
                    oid,
                    a.entities.len(),
                    a.requirements.len(),
                    a.relationships.len(),
                    if was_cached { " (cached)" } else { "" }
                );
                Some((oid, a))
            }
            Err(e) => {
                eprintln!("  [{:>3}/{}] ✗ {} — ERROR: {}", i, total, oid, truncate(&e, 160));
                None
            }
        }
    });
    results.into_iter().flatten().collect()
}

fn truncate(s: &str, n: usize) -> String {
    let flat: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() > n {
        format!("{}…", flat.chars().take(n).collect::<String>())
    } else {
        flat
    }
}

// Link a set of compiled objects into the global graph.
pub fn link_objects(
    proj: &Project,
    objects: &[(String, ObjectArtifact)],
    store: Option<&Store>,
    llm: &Llm,
) -> (LinkedArtifact, ReviewedArtifact) {
    let roots: BTreeSet<String> = objects
        .iter()
        .map(|(oid, _)| oid.clone())
        .filter(|oid| proj.is_root_file(oid))
        .collect();
    link::link(objects, &roots, llm, store)
}

// Compile and link the whole project (combined; used by the CLI and MCP).
pub fn compile_project(
    proj: &Project,
    store: Option<&Store>,
    llm: &Llm,
    overlay: &std::collections::BTreeMap<String, String>,
    verbose: bool,
) -> Build {
    let objects = compile_files(proj, store, llm, overlay, verbose);
    let (linked, reviewed) = link_objects(proj, &objects, store, llm);
    Build {
        objects,
        linked,
        reviewed,
    }
}

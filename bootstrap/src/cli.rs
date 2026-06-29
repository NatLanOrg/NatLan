// CLI command implementations over the engine. Mirrors docs/cli.md.
use crate::cache::Store;
use crate::engine::{self, Build};
use crate::llm::Llm;
use crate::model::Diagnostic;
use crate::project::{find_root, Project};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub struct Options {
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub api_key: Option<String>,
    pub out: Option<String>,
}

// Resolve the project, LLM settings, and output directory from positional paths + overrides.
pub fn resolve(paths: &[String], opts: &Options) -> (Project, Llm, PathBuf) {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut proj = if !paths.is_empty() {
        // Ad-hoc: explicit paths, no jazyk.toml required.
        let mut files = Vec::new();
        for p in paths {
            let pb = PathBuf::from(p);
            if pb.is_dir() {
                collect_md(&pb, &mut files);
            } else if pb.extension().map(|e| e == "md").unwrap_or(false) {
                files.push(pb);
            }
        }
        files.sort();
        files.dedup();
        let mut pr = Project::default();
        pr.root = cwd.clone();
        pr.explicit_files = Some(files);
        pr
    } else if let Some(root) = find_root(&cwd) {
        Project::load(&root)
    } else {
        let mut pr = Project::default();
        pr.root = cwd.clone();
        pr
    };

    // LLM settings precedence (highest first):
    //   CLI flag > env var > global config (~/.jazyk) > project [llm] / built-in default.
    // The endpoint/model/auth are machine-level, so the global config is the canonical home;
    // project [llm] remains a low-priority fallback for backward compatibility.
    let g = crate::project::load_global_llm();
    let base_url = opts
        .base_url
        .clone()
        .or_else(|| std::env::var("JAZYK_LLM_BASE_URL").ok())
        .or(g.base_url.clone())
        .unwrap_or_else(|| proj.llm.base_url.clone());
    let model = opts
        .model
        .clone()
        .or_else(|| std::env::var("JAZYK_MODEL").ok())
        .or(g.model.clone())
        .unwrap_or_else(|| proj.llm.model.clone());
    proj.llm.base_url = base_url.clone();
    proj.llm.model = model.clone();
    // The env var holding the key: global config's api_key_env, else the project's.
    let key_env = g.api_key_env.clone().unwrap_or_else(|| proj.llm.api_key_env.clone());
    let api_key = opts
        .api_key
        .clone()
        .or(g.api_key.clone())
        .or_else(|| std::env::var(&key_env).ok())
        .or_else(|| std::env::var("JAZYK_API_KEY").ok())
        .unwrap_or_default();

    // Temperature: env override > global config > default 0 (deterministic). A negative value
    // means "omit the field" for models that only accept their default.
    let temperature = std::env::var("JAZYK_TEMPERATURE")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .or(g.temperature)
        .unwrap_or(0.0);
    let temperature = if temperature < 0.0 { None } else { Some(temperature) };
    let llm = Llm {
        base_url,
        model,
        api_key,
        temperature,
    };
    let out_dir = opts
        .out
        .clone()
        .map(PathBuf::from)
        .unwrap_or_else(|| proj.root.join("jazyk-out"));
    (proj, llm, out_dir)
}

fn collect_md(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                collect_md(&p, out);
            } else if p.extension().map(|x| x == "md").unwrap_or(false) {
                out.push(p);
            }
        }
    }
}

fn build_once(proj: &Project, llm: &Llm, out_dir: &Path, verbose: bool) -> Build {
    let store = Store::new(out_dir, &llm.model);
    engine::compile_project(proj, Some(&store), llm, &BTreeMap::new(), verbose)
}

pub fn run_build(paths: &[String], opts: &Options) -> i32 {
    let (proj, llm, out_dir) = resolve(paths, opts);
    let files = proj.doc_files();
    if files.is_empty() {
        eprintln!("jazyk: no documentation files found");
        return 1;
    }
    eprintln!(
        "jazyk: compiling {} file(s) with model {} at {}",
        files.len(),
        llm.model,
        llm.base_url
    );
    let build = build_once(&proj, &llm, &out_dir, true);
    write_artifacts(&build, &out_dir);
    print_diagnostics(&build);
    eprintln!(
        "jazyk: done. {} entities, {} relationships, {} requirements. output: {}/",
        build.reviewed.entities.len(),
        build.reviewed.relationships.len(),
        build.reviewed.requirements.len(),
        out_dir.display()
    );
    if build.has_error() {
        1
    } else {
        0
    }
}

pub fn run_check(paths: &[String], opts: &Options) -> i32 {
    let (proj, llm, out_dir) = resolve(paths, opts);
    let build = build_once(&proj, &llm, &out_dir, false);
    print_diagnostics(&build);
    if build.has_error() {
        1
    } else {
        0
    }
}

pub fn run_watch(paths: &[String], opts: &Options) -> i32 {
    let (proj, llm, out_dir) = resolve(paths, opts);
    eprintln!("jazyk: watching for changes (Ctrl-C to stop)");
    let mut last = String::new();
    loop {
        let sig = fingerprint(&proj);
        if sig != last {
            last = sig;
            let build = build_once(&proj, &llm, &out_dir, false);
            write_artifacts(&build, &out_dir);
            eprintln!("---- rebuild ----");
            print_diagnostics(&build);
        }
        std::thread::sleep(std::time::Duration::from_millis(800));
    }
}

// A cheap content fingerprint over all doc files, to detect changes for `watch`.
fn fingerprint(proj: &Project) -> String {
    let mut parts = Vec::new();
    for f in proj.doc_files() {
        if let Ok(meta) = std::fs::metadata(&f) {
            if let Ok(modt) = meta.modified() {
                if let Ok(d) = modt.duration_since(std::time::UNIX_EPOCH) {
                    parts.push(format!("{}:{}", f.display(), d.as_millis()));
                }
            }
        }
    }
    parts.join("|")
}

pub fn run_codegen(paths: &[String], opts: &Options) -> i32 {
    let (proj, llm, out_dir) = resolve(paths, opts);
    let build = build_once(&proj, &llm, &out_dir, false);
    // Generate a code stub per entity from its assembled requirements (small-prompt regime).
    let dir = out_dir.join("codegen");
    std::fs::create_dir_all(&dir).ok();
    for ge in &build.reviewed.entities {
        let reqs: Vec<String> = build
            .reviewed
            .requirements
            .iter()
            .filter(|r| r.entities.contains(&ge.global_id))
            .map(|r| r.ears_text.clone())
            .collect();
        if reqs.is_empty() {
            continue;
        }
        let sys = "You are a code generator. Given an entity name, its definition, and its requirements, produce a single self-contained code module (choose an idiomatic language) implementing it. Return ONLY code, no prose, no markdown fences.";
        let user = format!(
            "Entity: {}\nDefinition: {}\nRequirements:\n{}",
            ge.canonical_name,
            ge.global_definition.clone().unwrap_or_default(),
            reqs.iter().map(|r| format!("- {}", r)).collect::<Vec<_>>().join("\n")
        );
        match llm.chat(sys, &user, &format!("codegen {}", ge.canonical_name)) {
            Ok(code) => {
                let path = dir.join(format!("{}.txt", crate::md::slug(&ge.canonical_name)));
                std::fs::write(&path, strip_fences(&code)).ok();
                eprintln!("  codegen {} -> {}", ge.canonical_name, path.display());
            }
            Err(e) => eprintln!("  codegen {}: ERROR {}", ge.canonical_name, e),
        }
    }
    0
}

pub fn run_testgen(paths: &[String], opts: &Options) -> i32 {
    let (proj, llm, out_dir) = resolve(paths, opts);
    let build = build_once(&proj, &llm, &out_dir, false);
    let dir = out_dir.join("testgen");
    std::fs::create_dir_all(&dir).ok();
    for ge in &build.reviewed.entities {
        let reqs: Vec<String> = build
            .reviewed
            .requirements
            .iter()
            .filter(|r| r.entities.contains(&ge.global_id))
            .map(|r| r.ears_text.clone())
            .collect();
        if reqs.is_empty() {
            continue;
        }
        let sys = "You generate tests from EARS requirements. For each requirement produce one test case (event->scenario, invariant->property check, unwanted->negative test). Return ONLY code, no prose, no markdown fences.";
        let user = format!(
            "Entity: {}\nRequirements:\n{}",
            ge.canonical_name,
            reqs.iter().map(|r| format!("- {}", r)).collect::<Vec<_>>().join("\n")
        );
        match llm.chat(sys, &user, &format!("testgen {}", ge.canonical_name)) {
            Ok(code) => {
                let path = dir.join(format!("{}_test.txt", crate::md::slug(&ge.canonical_name)));
                std::fs::write(&path, strip_fences(&code)).ok();
                eprintln!("  testgen {} -> {}", ge.canonical_name, path.display());
            }
            Err(e) => eprintln!("  testgen {}: ERROR {}", ge.canonical_name, e),
        }
    }
    0
}

fn strip_fences(s: &str) -> String {
    let t = s.trim();
    if let Some(rest) = t.strip_prefix("```") {
        // Drop the opening fence (and an optional language tag) and the closing fence.
        let rest = rest.splitn(2, '\n').nth(1).unwrap_or("");
        return rest.trim_end_matches("```").trim().to_string();
    }
    t.to_string()
}

pub fn write_artifacts(build: &Build, out_dir: &Path) {
    // Per-file object.yaml (and its stage slices) are written into the mirrored tree during
    // compilation by the store. Here we write the whole-program globals.
    let fmt = crate::serialize::DEFAULT;
    std::fs::create_dir_all(out_dir).ok();
    if let Ok(s) = fmt.to_string(&build.linked) {
        std::fs::write(out_dir.join("linked.yaml"), s).ok();
    }
    if let Ok(s) = fmt.to_string(&build.reviewed) {
        std::fs::write(out_dir.join("reviewed.yaml"), s).ok();
    }
    // Combined diagnostics store (object + global).
    let all = all_diagnostics(build);
    if let Ok(s) = fmt.to_string(&all) {
        std::fs::write(out_dir.join("diagnostics.yaml"), s).ok();
    }
}

// Merge per-object (A2/A3) diagnostics with the global (link/review) diagnostics.
pub fn all_diagnostics(build: &Build) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for (oid, art) in &build.objects {
        for d in &art.diagnostics {
            let mut d = d.clone();
            d.message = format!("{}: {}", oid, d.message);
            out.push(d);
        }
    }
    out.extend(build.reviewed.diagnostics.iter().cloned());
    out
}

pub fn print_diagnostics(build: &Build) {
    let all = all_diagnostics(build);
    let errors = all.iter().filter(|d| d.severity == "error").count();
    let warnings = all.iter().filter(|d| d.severity == "warning").count();
    if all.is_empty() {
        eprintln!("jazyk: no diagnostics");
    }
    for d in &all {
        if d.severity == "none" {
            continue;
        }
        let tag = match d.severity.as_str() {
            "error" => "error",
            "warning" => "warning",
            "info" => "info",
            other => other,
        };
        eprintln!("{} [{}] {}", tag, d.rule, d.message);
        if let Some(r) = &d.reasoning {
            if !r.is_empty() {
                eprintln!("      reason: {}", r);
            }
        }
    }
    eprintln!("jazyk: {} error(s), {} warning(s)", errors, warnings);
}

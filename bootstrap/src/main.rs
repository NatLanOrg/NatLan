mod benchmark;
mod cache;
mod cli;
mod compile;
mod engine;
mod jsonrpc;
mod link;
mod llm;
mod lsp;
mod mcp;
mod md;
mod model;
mod parallel;
mod project;
mod serialize;

use llm::Llm;
use std::path::Path;

// Load a .env file by walking up from the current directory. Does not override existing env vars.
fn load_dotenv() {
    let mut dir = std::env::current_dir().ok();
    while let Some(d) = dir {
        let f = d.join(".env");
        if f.exists() {
            if let Ok(content) = std::fs::read_to_string(&f) {
                for line in content.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    if let Some((k, v)) = line.split_once('=') {
                        let k = k.trim();
                        let v = v.trim().trim_matches('"');
                        if std::env::var(k).is_err() {
                            std::env::set_var(k, v);
                        }
                    }
                }
            }
            break;
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }
}

// The top-level usage text, shared by the error path and `--help`.
fn top_usage() -> String {
    let mut s = String::new();
    s.push_str("jazyk — natural language compiler\n\n");
    s.push_str("usage:\n");
    s.push_str("  jazyk build [path...]      compile + link, write artifacts\n");
    s.push_str("  jazyk check [path...]      compile, report diagnostics only (CI)\n");
    s.push_str("  jazyk watch [path...]      recompile on change\n");
    s.push_str("  jazyk lsp [--stdio]        language server over stdio\n");
    s.push_str("  jazyk mcp                  MCP server over stdio\n");
    s.push_str("  jazyk benchmark            grade the configured model\n");
    s.push_str("  jazyk codegen [path...]    generate code from entities\n");
    s.push_str("  jazyk testgen [path...]    generate tests from requirements\n");
    s.push_str("  jazyk gen <desc.md>        generate an SVG asset from a description\n");
    s.push_str("\noptions: --llm-base-url URL  --model M  --api-key K  --out DIR\n");
    s.push_str("         --help, -h          print help and exit\n");
    s.push_str("\nrun `jazyk <command> --help` for command-specific help.");
    s
}

// Help for a single command, or None if unknown.
fn command_help(cmd: &str) -> Option<String> {
    let llm_opts = "  --llm-base-url URL   override the LLM endpoint\n  --model M            override the model\n  --api-key K          override the API key\n  --out DIR            output directory (default <root>/jazyk-out)";
    let body = match cmd {
        "build" => format!(
            "jazyk build [path...]\n\nCompile and link the project, then write build artifacts to the out\ndirectory. Prints warnings and errors. Exits non-zero on error.\n\nWith no paths, compiles the project found by walking up to a jazyk.toml.\nPassing explicit paths compiles them ad-hoc without a jazyk.toml.\n\noptions:\n{}",
            llm_opts
        ),
        "check" => format!(
            "jazyk check [path...]\n\nCompile and report diagnostics only, without writing artifacts. Suitable\nfor CI and pre-commit hooks. Exits non-zero on any error diagnostic.\n\noptions:\n{}",
            llm_opts
        ),
        "watch" => format!(
            "jazyk watch [path...]\n\nRecompile as files change until interrupted (Ctrl-C).\n\noptions:\n{}",
            llm_opts
        ),
        "lsp" => format!(
            "jazyk lsp [--stdio]\n\nStart the language server over stdio for editor integration. Serves\ndiagnostics, definition, references, hover, and completion.\n\noptions:\n  --stdio              use stdio transport (default)\n{}",
            llm_opts
        ),
        "mcp" => format!(
            "jazyk mcp\n\nStart the MCP server over stdio for agent integration. Embeds the\ncompiler and serves the build graph (compile, get_entity, …).\n\noptions:\n{}",
            llm_opts
        ),
        "benchmark" => format!(
            "jazyk benchmark\n\nGrade whether a model is good enough to compile Jazyk. Runs predefined\ncases against the LLM stages and reports a score and a verdict. Exits\nnon-zero if the model fails the verdict.\n\nWith no flags, grades the default model resolved from the global config\nor environment. Pass --model, --llm-base-url, or --api-key (alone or in\ncombination) to grade a specific model or endpoint.\n\noptions:\n{}",
            llm_opts
        ),
        "codegen" => format!(
            "jazyk codegen [path...]\n\nGenerate a code stub per entity from its assembled requirements. Writes\nto <out>/codegen.\n\noptions:\n{}",
            llm_opts
        ),
        "testgen" => format!(
            "jazyk testgen [path...]\n\nGenerate tests per requirement. Writes to <out>/testgen.\n\noptions:\n{}",
            llm_opts
        ),
        "gen" => "jazyk gen <description.md> [--out FILE]\n\nGenerate an asset (e.g. an SVG logo) from a natural language description.\n\noptions:\n  --out FILE            output file (default logo.svg)\n  --llm-base-url URL    override the LLM endpoint\n  --model M            override the model\n  --api-key K          override the API key".to_string(),
        _ => return None,
    };
    Some(body)
}

// Print help and exit 0. Empty cmd prints the top-level usage; an unknown
// command falls back to the top-level usage too.
fn help(cmd: &str) -> ! {
    match command_help(cmd) {
        Some(h) => println!("{}", h),
        None => println!("{}", top_usage()),
    }
    std::process::exit(0);
}

fn usage() -> ! {
    eprintln!("{}", top_usage());
    std::process::exit(2);
}

fn main() {
    load_dotenv();
    let args: Vec<String> = std::env::args().collect();
    let mut opts = cli::Options {
        base_url: None,
        model: None,
        api_key: None,
        out: None,
    };
    let mut paths: Vec<String> = Vec::new();
    let mut cmd = String::new();
    let mut want_help = false;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => want_help = true,
            // `help` / `help <command>`: treat as a help request, with the next
            // positional read as the command (handled by the cmd-empty arm below).
            "help" if cmd.is_empty() => want_help = true,
            "--llm-base-url" => {
                i += 1;
                opts.base_url = args.get(i).cloned();
            }
            "--model" => {
                i += 1;
                opts.model = args.get(i).cloned();
            }
            "--api-key" => {
                i += 1;
                opts.api_key = args.get(i).cloned();
            }
            "--out" => {
                i += 1;
                opts.out = args.get(i).cloned();
            }
            "--stdio" => {} // accepted, default transport
            s if cmd.is_empty() => cmd = s.to_string(),
            s => paths.push(s.to_string()),
        }
        i += 1;
    }

    // `--help`/`-h`/`help` prints usage and exits 0, before running anything.
    if want_help {
        help(&cmd);
    }

    let code = match cmd.as_str() {
        "build" => cli::run_build(&paths, &opts),
        "check" => cli::run_check(&paths, &opts),
        "watch" => cli::run_watch(&paths, &opts),
        "codegen" => cli::run_codegen(&paths, &opts),
        "testgen" => cli::run_testgen(&paths, &opts),
        "benchmark" => {
            let (_proj, llm, _out) = cli::resolve(&paths, &opts);
            benchmark::run(&llm)
        }
        "lsp" => {
            let (proj, llm, out) = cli::resolve(&paths, &opts);
            lsp::Server::new(proj, llm, out).run();
            0
        }
        "mcp" => {
            let (proj, llm, out) = cli::resolve(&paths, &opts);
            mcp::Mcp::new(proj, llm, out).run();
            0
        }
        "gen" => {
            let (_proj, llm, _out) = cli::resolve(&[], &opts);
            run_gen(&paths, opts.out.as_deref().unwrap_or("logo.svg"), &llm)
        }
        _ => usage(),
    };
    std::process::exit(code);
}

// Generate an asset (e.g. an SVG logo) from a natural language description.
fn run_gen(paths: &[String], out_file: &str, llm: &Llm) -> i32 {
    let input = match paths.first() {
        Some(p) => p,
        None => {
            eprintln!("usage: jazyk gen <description.md> [--out FILE]");
            return 2;
        }
    };
    let desc = match std::fs::read_to_string(input) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("read {}: {}", input, e);
            return 1;
        }
    };
    let sys = "You are an SVG illustrator. Given a description, produce one clean, minimal, valid SVG. Use an outline style: strokes only with fill none, rounded line caps and joins, a single stroke color, and a viewBox so it scales. Return ONLY the SVG markup starting with '<svg' and ending with '</svg>'. No prose, no markdown code fences.";
    eprintln!("jazyk: generating SVG from {}", input);
    match llm.chat(sys, &desc, "gen svg") {
        Ok(content) => match llm::extract_svg(&content) {
            Some(svg) => {
                if let Some(parent) = Path::new(out_file).parent() {
                    std::fs::create_dir_all(parent).ok();
                }
                std::fs::write(out_file, svg).ok();
                eprintln!("jazyk: wrote {}", out_file);
                0
            }
            None => {
                eprintln!("ERROR: no <svg> in model output:\n{}", content);
                1
            }
        },
        Err(e) => {
            eprintln!("ERROR: {}", e);
            1
        }
    }
}

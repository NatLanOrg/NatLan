// Source-mirrored stage store. This is the cache: each compilation/linking stage writes one YAML
// file mirroring the source tree, and a stage is skipped on rebuild when its stored key still
// matches. There is no separate hash-named cache dir. See docs/compiler/artifacts.md#storage-layout
// and docs/compiler/concepts/determinism.md#process-only-changed.
//
// Layout: out_dir/target/<object_id>/<stage>.yaml for per-file compile stages (object_id is the
// relative posix path, e.g. docs/cli.md, so the dir keeps the source extension), and
// out_dir/target/link/<slug>.<stage>.yaml for the whole-program link stages. The source-mirrored tree
// is nested under target/ so it stays clearly separated from the finals (linked/reviewed/diagnostics
// at the out-dir root). Each file begins with a `# jazyk:` header comment recording the stage, prompt
// version, model, and cache key. The comment is ignored by the YAML parser, so the rest of the file
// is the stage payload verbatim (object.yaml is exactly the object artifact).
use crate::compile::PROMPT_VERSION;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::{Path, PathBuf};

pub struct Store {
    out_dir: PathBuf,
    model: String,
}

impl Store {
    pub fn new(out_dir: &Path, model: &str) -> Store {
        Store {
            out_dir: out_dir.to_path_buf(),
            model: model.to_string(),
        }
    }

    fn header(&self, stage: &str, key: &str) -> String {
        format!(
            "# jazyk: stage={} promptVersion={} model={} key={}\n",
            stage, PROMPT_VERSION, self.model, key
        )
    }

    // Parse the leading `# jazyk:` header comment into (model, promptVersion, key).
    fn parse_header(s: &str) -> Option<(String, String, String)> {
        let rest = s.lines().next()?.strip_prefix("# jazyk:")?.trim().to_string();
        let (mut model, mut pv, mut key) = (None, None, None);
        for tok in rest.split_whitespace() {
            if let Some(v) = tok.strip_prefix("model=") {
                model = Some(v.to_string());
            } else if let Some(v) = tok.strip_prefix("promptVersion=") {
                pv = Some(v.to_string());
            } else if let Some(v) = tok.strip_prefix("key=") {
                key = Some(v.to_string());
            }
        }
        Some((model?, pv?, key?))
    }

    fn write(&self, path: &Path, stage: &str, key: &str, body: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let mut out = self.header(stage, key);
        out.push_str(body);
        std::fs::write(path, out).ok();
    }

    // Read and deserialize only when the stored header matches this model, the current prompt
    // version, and the expected key. Otherwise the result is stale and treated as absent.
    fn read_if_match<T: DeserializeOwned>(&self, path: &Path, key: &str) -> Option<T> {
        let s = std::fs::read_to_string(path).ok()?;
        let (model, pv, stored) = Self::parse_header(&s)?;
        if model != self.model || pv != PROMPT_VERSION || stored != key {
            return None;
        }
        crate::serialize::DEFAULT.from_str(&s).ok()
    }

    fn stage_path(&self, object_id: &str, stage: &str) -> PathBuf {
        self.out_dir.join("target").join(object_id).join(format!("{}.yaml", stage))
    }

    pub fn get_stage<T: DeserializeOwned>(&self, object_id: &str, stage: &str, key: &str) -> Option<T> {
        self.read_if_match(&self.stage_path(object_id, stage), key)
    }

    pub fn put_stage<T: Serialize>(&self, object_id: &str, stage: &str, key: &str, v: &T) {
        if let Ok(body) = crate::serialize::DEFAULT.to_string(v) {
            self.write(&self.stage_path(object_id, stage), stage, key, &body);
        }
    }

    fn link_path(&self, slug: &str, stage: &str) -> PathBuf {
        self.out_dir.join("target").join("link").join(format!("{}.{}.yaml", slug, stage))
    }

    pub fn get_link<T: DeserializeOwned>(&self, slug: &str, stage: &str, key: &str) -> Option<T> {
        self.read_if_match(&self.link_path(slug, stage), key)
    }

    pub fn put_link<T: Serialize>(&self, slug: &str, stage: &str, key: &str, v: &T) {
        if let Ok(body) = crate::serialize::DEFAULT.to_string(v) {
            self.write(&self.link_path(slug, stage), stage, key, &body);
        }
    }
}

// Filesystem-safe slug for an entity global id (e.g. `ent:database` -> `ent-database`). The mapping
// is not reversible, but global ids are unique so slugs stay unique enough for the link cache.
pub fn entity_slug(global_id: &str) -> String {
    global_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

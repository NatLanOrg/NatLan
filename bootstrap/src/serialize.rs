// Serialization format for artifacts and the cache. YAML today; the enum is the seam where another
// format (or a visualization export) slots in without touching call sites. See
// docs/compiler/artifacts.md#storage-layout.
use serde::de::DeserializeOwned;
use serde::Serialize;

#[derive(Clone, Copy)]
pub enum Format {
    Yaml,
}

// The format every frontend writes today.
pub const DEFAULT: Format = Format::Yaml;

impl Format {
    #[allow(dead_code)] // part of the format seam; used when a second format or exporter is added
    pub fn ext(self) -> &'static str {
        match self {
            Format::Yaml => "yaml",
        }
    }

    // Pretty, human-readable rendering (YAML is block style by default).
    pub fn to_string<T: Serialize>(self, v: &T) -> Result<String, String> {
        match self {
            Format::Yaml => serde_norway::to_string(v).map_err(|e| e.to_string()),
        }
    }

    pub fn from_str<T: DeserializeOwned>(self, s: &str) -> Result<T, String> {
        match self {
            Format::Yaml => serde_norway::from_str(s).map_err(|e| e.to_string()),
        }
    }
}

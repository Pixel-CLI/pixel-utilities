// SPDX-FileCopyrightText: The Pixel contributors
// SPDX-License-Identifier: MIT

//! The slice of Pixel's global configuration `web-search` reads.
//!
//! The file is core's: `~/.pixel/config.yaml` (or the legacy `config.json`
//! until an install writes the YAML one), a mapping that holds
//! `web_search.searxng_url` and `remote_keys.perplexity`. Core writes it
//! (`pixel config`, the web-search setup step); this module only reads, with
//! the same rules as `crates/pixel/src/config_file.rs` and
//! `config_cmd::web_search_*` of Pixel-CLI/pixel (MIT): a missing file is an
//! empty mapping, an unreadable or invalid one yields no value, and parser
//! diagnostics are never shown because they can quote a credential.

use std::path::{Path, PathBuf};

use serde_json::Value;

const FILE_NAME: &str = "config.yaml";
const LEGACY_FILE_NAME: &str = "config.json";

/// `config.yaml` in `directory`, or the legacy `config.json` while only that
/// exists.
fn preferred_path(directory: &Path) -> PathBuf {
    let yaml = directory.join(FILE_NAME);
    let legacy = directory.join(LEGACY_FILE_NAME);
    if !yaml.exists() && legacy.exists() {
        legacy
    } else {
        yaml
    }
}

/// The global configuration file: `$HOME/.pixel/config.yaml`.
fn global_config_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| preferred_path(&PathBuf::from(home).join(".pixel")))
}

/// The configuration mapping at `path`; `None` when it is missing, unreadable
/// or not a mapping.
fn load(path: &Path) -> Option<Value> {
    let text = std::fs::read_to_string(path).ok()?;
    let value: Value = if path.extension().is_some_and(|ext| ext == "json") {
        serde_json::from_str(&text).ok()?
    } else {
        serde_saphyr::from_str(&text).ok()?
    };
    if value.is_null() {
        return Some(serde_json::json!({}));
    }
    value.is_object().then_some(value)
}

/// A non-empty string at `section.key` of `doc`.
fn string_at(doc: &Value, section: &str, key: &str) -> Option<String> {
    doc.get(section)?
        .get(key)?
        .as_str()
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn global_string(section: &str, key: &str) -> Option<String> {
    string_at(&load(&global_config_path()?)?, section, key)
}

/// The SearXNG base URL the web-search setup recorded, if any: the
/// `PIXEL_WEB_SEARCH_URL` env var wins over it in `web-search`.
pub fn web_search_searxng_url() -> Option<String> {
    global_string("web_search", "searxng_url")
}

/// The Perplexity API key the web-search setup stored, under
/// `remote_keys.perplexity`. Never echoed back.
pub fn web_search_perplexity_key() -> Option<String> {
    global_string("remote_keys", "perplexity")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, text).unwrap();
        path
    }

    #[test]
    fn yaml_is_preferred_and_the_legacy_json_is_the_fallback() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(preferred_path(dir.path()), dir.path().join("config.yaml"));
        write(dir.path(), "config.json", "{}");
        assert_eq!(preferred_path(dir.path()), dir.path().join("config.json"));
        write(dir.path(), "config.yaml", "");
        assert_eq!(preferred_path(dir.path()), dir.path().join("config.yaml"));
    }

    #[test]
    fn the_stored_values_come_from_yaml_or_json() {
        let dir = tempfile::tempdir().unwrap();
        let yaml = write(
            dir.path(),
            "config.yaml",
            "web_search:\n  searxng_url: https://sx.test\nremote_keys:\n  perplexity: pplx\n",
        );
        let doc = load(&yaml).unwrap();
        assert_eq!(
            string_at(&doc, "web_search", "searxng_url").as_deref(),
            Some("https://sx.test")
        );
        assert_eq!(
            string_at(&doc, "remote_keys", "perplexity").as_deref(),
            Some("pplx")
        );
        let json = write(
            dir.path(),
            "config.json",
            r#"{"web_search":{"searxng_url":"https://sx.test"}}"#,
        );
        let doc = load(&json).unwrap();
        assert_eq!(
            string_at(&doc, "web_search", "searxng_url").as_deref(),
            Some("https://sx.test")
        );
        assert_eq!(string_at(&doc, "remote_keys", "perplexity"), None);
    }

    #[test]
    fn a_missing_empty_or_invalid_file_gives_no_value() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load(&dir.path().join("config.yaml")).is_none());
        let empty = write(dir.path(), "config.yaml", "");
        assert_eq!(load(&empty), Some(serde_json::json!({})));
        let broken = write(dir.path(), "config.yaml", "web_search: [\n");
        assert!(load(&broken).is_none());
        let scalar = write(dir.path(), "config.yaml", "just text\n");
        assert!(load(&scalar).is_none());
        let blank = write(
            dir.path(),
            "config.yaml",
            "web_search:\n  searxng_url: \"\"\n",
        );
        let doc = load(&blank).unwrap();
        assert_eq!(string_at(&doc, "web_search", "searxng_url"), None);
    }
}

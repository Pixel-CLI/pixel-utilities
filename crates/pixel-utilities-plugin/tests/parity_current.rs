// SPDX-FileCopyrightText: The Pixel contributors
// SPDX-License-Identifier: MIT

//! `web-search`, `check-release` and `ai-cli-readify` against the built-in
//! commands of Pixel core (goldens recorded from `ad788194`, `origin/main`
//! when they were ported; `ai-cli-readify` from a build with the `readify`
//! feature).
//!
//! Nothing here reaches a real network: `web-search` talks to a loopback
//! SearXNG stub, and `ai-cli-readify` runs without a provider key, which is
//! the answer both implementations give before any request is made.

use std::io::{Read, Write};
use std::net::TcpListener;

use pixel_utilities_plugin::testing::{Setup, assert_parity, git_fixture, write};

const BIN: &str = env!("CARGO_BIN_EXE_pixel-utilities");

fn parity(command: &str, case: &str, make: &dyn Fn() -> Setup) {
    assert_parity(env!("CARGO_MANIFEST_DIR"), BIN, command, case, make);
}

// --- web-search --------------------------------------------------------------

/// Serve `body` as a `200 OK` JSON response on an ephemeral loopback port,
/// `connections` times; returns the base URL (`http://127.0.0.1:<port>`).
fn http_stub(body: &'static str, connections: usize) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let port = listener.local_addr().expect("addr").port();
    std::thread::spawn(move || {
        for _ in 0..connections {
            let Ok((mut stream, _)) = listener.accept() else {
                break;
            };
            let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf);
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });
    format!("http://127.0.0.1:{port}")
}

const TWO_HITS: &str = r#"{"results":[{"title":"JEV","url":"https://example.test/jev","content":"Joint Embedded Validator","engine":"stub"},{"title":"JEV again","url":"https://example.test/jev2","content":"","engine":"stub"},{"title":"JEV","url":"https://example.test/jev","content":"a duplicate","engine":"stub"}]}"#;

/// `args` against a SearXNG stub named by `PIXEL_WEB_SEARCH_URL`, with an
/// empty home so no stored configuration is read.
fn search(args: &[&str], body: &'static str) -> Setup {
    let mut setup = Setup::new(git_fixture(), args);
    let home = setup.scratch();
    setup
        .env("HOME", home)
        .env("PIXEL_WEB_SEARCH_URL", http_stub(body, 1))
        // The stub's port differs between the two runs.
        .scrub_pattern(r"127\.0\.0\.1:\d+", "STUB")
}

#[test]
fn web_search_json_has_hits_epistemics_and_snapshot() {
    parity("web-search", "web-search-json", &|| {
        search(&["JEV", "--json"], TWO_HITS)
    });
}

#[test]
fn web_search_text_lists_numbered_hits() {
    parity("web-search", "web-search-text", &|| {
        search(&["JEV"], TWO_HITS)
    });
}

#[test]
fn web_search_honours_the_limit() {
    parity("web-search", "web-search-limit-json", &|| {
        search(&["JEV", "--limit", "1", "--json"], TWO_HITS)
    });
}

#[test]
fn web_search_without_hits_is_unresolved() {
    parity("web-search", "web-search-unresolved", &|| {
        search(&["JEV"], r#"{"results":[]}"#)
    });
    parity("web-search", "web-search-unresolved-json", &|| {
        search(&["JEV", "--json"], r#"{"results":[]}"#)
    });
}

#[test]
fn web_search_reads_the_stored_searxng_url() {
    parity("web-search", "web-search-stored-url-json", &|| {
        let mut setup = Setup::new(git_fixture(), &["JEV", "--json"]);
        let home = setup.scratch();
        write(
            &home,
            ".pixel/config.yaml",
            &format!("web_search:\n  searxng_url: {}\n", http_stub(TWO_HITS, 1)),
        );
        setup
            .env("HOME", home)
            .env("PIXEL_WEB_SEARCH_URL", "")
            .scrub_pattern(r"127\.0\.0\.1:\d+", "STUB")
    });
}

#[test]
fn web_search_needs_a_query() {
    parity("web-search", "web-search-no-query", &|| {
        Setup::new(git_fixture(), &[])
    });
}

#[test]
fn web_search_rejects_a_non_numeric_limit() {
    parity("web-search", "web-search-bad-limit", &|| {
        Setup::new(git_fixture(), &["jev", "--limit", "abc"])
    });
}

// --- check-release -----------------------------------------------------------

/// A minimal workspace at version 0.2.3 with a matching lock and changelog.
fn release_fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    write(
        root,
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/pixel\", \"crates/pixel-ops\"]\nresolver = \"3\"\n",
    );
    write(
        root,
        "crates/pixel/Cargo.toml",
        "[package]\nname = \"pixel-cli\"\nversion = \"0.2.3\"\n\n[[bin]]\nname = \"pixel\"\n",
    );
    write(
        root,
        "crates/pixel-ops/Cargo.toml",
        "[package]\nname = \"pixel-ops\"\nversion = \"0.2.2\"\n",
    );
    write_lock(root, "0.2.3");
    write(
        root,
        "CHANGELOG.md",
        "# Changelog\n\n## [Unreleased]\n\n## [0.2.3] - 2026-09-12\n\n### Added\n- a thing\n\n## [0.2.2] - 2026-09-11\n- older\n",
    );
    dir
}

fn write_lock(root: &std::path::Path, cli_version: &str) {
    write(
        root,
        "Cargo.lock",
        &format!(
            "version = 4\n\n[[package]]\nname = \"pixel-cli\"\nversion = \"{cli_version}\"\n\n[[package]]\nname = \"pixel-ops\"\nversion = \"0.2.2\"\n"
        ),
    );
}

fn release(args: &'static [&'static str]) -> impl Fn() -> Setup {
    move || Setup::new(release_fixture(), args)
}

#[test]
fn a_consistent_release_passes() {
    parity("check-release", "check-release-ok", &release(&["v0.2.3"]));
}

#[test]
fn a_consistent_release_as_json() {
    parity(
        "check-release",
        "check-release-ok-json",
        &release(&["--json", "refs/tags/v0.2.3"]),
    );
}

#[test]
fn a_tag_that_is_not_the_cli_version_fails() {
    parity(
        "check-release",
        "check-release-wrong-version",
        &release(&["0.2.4"]),
    );
    parity(
        "check-release",
        "check-release-wrong-version-json",
        &release(&["--json", "0.2.4"]),
    );
}

#[test]
fn a_stale_lock_fails() {
    parity("check-release", "check-release-stale-lock", &|| {
        let setup = Setup::new(release_fixture(), &["0.2.3"]);
        write_lock(setup.dir.path(), "0.2.2");
        setup
    });
}

#[test]
fn a_changelog_without_the_heading_fails() {
    parity("check-release", "check-release-no-heading", &|| {
        let setup = Setup::new(release_fixture(), &["0.2.3"]);
        write(
            setup.dir.path(),
            "CHANGELOG.md",
            "# Changelog\n\n## [Unreleased]\n\n- not yet cut\n",
        );
        setup
    });
}

#[test]
fn a_malformed_version_is_rejected_before_any_file_is_read() {
    parity("check-release", "check-release-bad-version", &|| {
        Setup::new(git_fixture(), &["not-a-version"])
    });
}

#[test]
fn a_directory_without_a_workspace_is_an_error() {
    parity("check-release", "check-release-no-workspace", &|| {
        Setup::new(git_fixture(), &["1.2.3", "--repo", "."])
    });
}

// --- ai-cli-readify ----------------------------------------------------------

/// No provider key, an empty home and no agent CLI on `PATH`: the probe is
/// never sent, and the report is the same for both implementations.
fn readify(args: &[&str]) -> Setup {
    let mut setup = Setup::new(git_fixture(), args);
    let home = setup.scratch();
    setup
        .env("HOME", home)
        .env("PATH", "/usr/bin:/bin")
        .env("OLLAMA_API_KEY", "")
}

#[test]
fn readify_without_a_key_reports_not_ready() {
    parity("ai-cli-readify", "ai-cli-readify-no-key", &|| readify(&[]));
}

#[test]
fn readify_without_a_key_reports_not_ready_as_json() {
    parity("ai-cli-readify", "ai-cli-readify-no-key-json", &|| {
        readify(&["--json"])
    });
}

#[test]
fn readify_can_verify_a_subset_of_agents() {
    parity("ai-cli-readify", "ai-cli-readify-agent-json", &|| {
        readify(&["--agent", "codex", "--agent", "devin", "--json"])
    });
}

#[test]
fn readify_apply_without_an_answering_provider_rewrites_nothing() {
    parity("ai-cli-readify", "ai-cli-readify-apply", &|| {
        readify(&["--apply"])
    });
}

#[test]
fn readify_rejects_an_unknown_agent() {
    parity("ai-cli-readify", "ai-cli-readify-bad-agent", &|| {
        readify(&["--agent", "nope"])
    });
}

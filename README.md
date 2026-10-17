# pixel-utilities

The `utilities` plugin for [Pixel](https://github.com/Pixel-CLI/pixel): three
small commands that are not retrieval, in one `pixel-utilities` binary. They
used to be built into `pixel`; they live here so the core stays small and only
the people who use them install them.

| Command | What it does |
| --- | --- |
| `pixel web-search` | deterministic web lookup for terms the index cannot know (the refine step of a gated `pixel plan`); SearXNG, Perplexity or a free public chain; no LLM, no daemon |
| `pixel check-release` | check that a release tag is consistent with the tree before it is built or published: the CLI version, `Cargo.lock` freshness, the `CHANGELOG.md` heading and an empty Unreleased section; exit 1 on any failed check |
| `pixel ai-cli-readify` | probe provider readiness for Codex, Claude Code, Antigravity and Devin with one real request, and with `--apply` point their configs at the provider that answered |

Each command reproduces the built-in's arguments, output and exit status.
`pixel-utilities --help` lists them; `pixel-utilities <command> --help` shows
one's options.

## Install

With a Pixel that has the plugin host (`pixel install <source>`):

```bash
pixel install github:Pixel-CLI/pixel-utilities
pixel plugin enable utilities   # the plugin declares network access (web-search, ai-cli-readify)
```

The release assets are `pixel-utilities-<target-triple>` executables, each with
a `.sha256` file: `aarch64-apple-darwin`, `x86_64-apple-darwin`,
`x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl` (Linux is
musl: the triples `pixel install` asks for).

From source, without Pixel's plugin management:

```bash
cargo install --git https://github.com/Pixel-CLI/pixel-utilities pixel-utilities-plugin
pixel-utilities install-links   # makes `pixel web-search`, `pixel check-release`, ... find it
```

## How `pixel <command>` finds this plugin

Plugin API 1 resolves `pixel web-search` to a plugin *named* `web-search`, or to
an executable `pixel-web-search` on `PATH`. This repository is **one** plugin
named `utilities` that answers to three names, so:

- `pixel utilities <command> …` always works once the plugin is installed (the
  host runs `bin/pixel-utilities`, which is, or finds, the `pixel-utilities`
  binary).
- `pixel <command> …` works through `PATH` links. `pixel-utilities
  install-links` creates `pixel-<command>` symlinks to `pixel-utilities` next
  to it (or in `--dir <dir>`); the binary runs the command named by the name it
  was started under. `pixel-utilities install-links --remove` deletes them. It
  never replaces a file it did not make.
- [`pixel-plugin.toml`](pixel-plugin.toml) lists the three names under
  `commands`. API 1 ignores that key. A host that reads it could route every
  listed name to `pixel-utilities <name>` with no links at all;
  `tests/manifest.rs` keeps the list equal to what the binary has.

## Environment

The host passes `PIXEL_API`, `PIXEL_REPO_ROOT`, `PIXEL_GRAPH_DB` and
`PIXEL_BIN`; all are optional. None of the three commands takes a `[PATH]`:
`check-release` reads `--repo` (default: the current directory), exactly as the
built-in did.

`web-search` reads `PIXEL_WEB_SEARCH_URL` (a SearXNG base URL) and
`PERPLEXITY_API_KEY`, then the same values stored in Pixel's global
configuration (`~/.pixel/config.yaml`: `web_search.searxng_url`,
`remote_keys.perplexity`). Core writes that file (`pixel config`, the
web-search setup step); the plugin only reads it.

`ai-cli-readify` reads `OLLAMA_API_KEY`. Under `--authenticate` it replays
Claude's browser sign-in through `pixel flow`, run with the host's `PIXEL_BIN`
(else `pixel` from `PATH`).

## Differences from the built-ins

- **No metrics line, no action-log entry.** A plugin is a separate process and
  the invocation accounting belongs to the `pixel` binary. `--metrics on|off`
  is still accepted (hidden), so scripts that pass it keep parsing.
- **`ai-cli-readify` is always built.** In core it sat behind the `readify`
  cargo feature, left out of the shipped binary (issue #602); installing this
  plugin is the opt-in. It replays the browser flow through the host's `pixel`
  (`PIXEL_BIN`), where core used its own executable.
- **No old alias.** `release-check` was a clap alias of `check-release`; the
  plugin answers to canonical names only.
- **The web-search setup step stays in core.** The interactive question that
  records a SearXNG URL or a Perplexity key is part of `pixel config setup`
  and writes core's configuration; only the command that uses the answer
  moved.

## Where the code comes from

Library crates of Pixel core (`pixel-release`, `pixel-install`, `pixel-proto`,
`pixel-daemon`, `pixel-index`) come through a git dependency pinned to one
commit of [Pixel-CLI/pixel](https://github.com/Pixel-CLI/pixel), recorded once
in `[workspace.dependencies]` of [`Cargo.toml`](Cargo.toml)
(`scripts/check-pin.sh` enforces a single value). Logic that lived only in the
`pixel` binary crate is ported here, MIT, with its origin in each module's doc:

| Here | From |
| --- | --- |
| `commands/web_search.rs` | `crates/pixel/src/web_search.rs` |
| `config.rs` | `config_file::load` and `config_cmd::web_search_*` (read side only) |
| `commands/check_release.rs` | the `CheckRelease` command of `crates/pixel/src/main.rs` |
| `commands/readify.rs`, `ai_cli_readify/` | `crates/pixel/src/ai_cli_readify/` and its command in `main.rs` |
| `env.rs`, `root.rs`, `output.rs`, `run.rs`, `links.rs` | as in Pixel-CLI/pixel-git |

## Versioning

One version (`[workspace.package]`), equal to the `version` in
`pixel-plugin.toml`; a release is a `vX.Y.Z` tag on `main`, which builds the
assets above. `api = 1` changes only when Pixel changes the plugin API. A bump
of the Pixel pin is a minor version when output changes, otherwise a patch.

## License

MIT, see [LICENSE](LICENSE).

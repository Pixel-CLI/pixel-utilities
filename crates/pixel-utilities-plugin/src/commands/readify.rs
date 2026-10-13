// SPDX-FileCopyrightText: The Pixel contributors
// SPDX-License-Identifier: MIT

//! `pixel ai-cli-readify`: the command line and handler around
//! [`crate::ai_cli_readify`].
//!
//! Ported from the `AiCliReadify` command in `crates/pixel/src/main.rs` of
//! Pixel-CLI/pixel (MIT), where it sat behind the `readify` cargo feature.

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use clap::Parser;

use crate::ai_cli_readify::{self, Agent};
use crate::output::print_data;
use crate::run::{HostFlags, run};

/// Probe provider readiness for Codex, Claude Code, Antigravity, and
/// Devin by sending one real `POST /chat/completions` ("Reply exactly
/// READY.", a 1024-token reservation, 20 s timeout) to Ollama Cloud. A
/// 200 that answers with the READY it asked for means the provider is
/// ready; a 200 whose body reaches no model carries its own failure
/// rather than counting as one. With
/// `--apply`, the provider that answered is written into
/// `~/.codex/config.toml` (`model`, `model_provider`,
/// `[model_providers.recording_cloud]`),
/// `~/.claude/settings.json` (`env.ANTHROPIC_BASE_URL` and the model
/// slots), and
/// `~/.gemini/antigravity-cli/settings.json`
/// (`AGY_LLM_GATEWAY_URL`); Devin is verified-only, never rewritten.
/// Claude's gateway credential and its `x-litellm-api-key` header are
/// left to the shell — a settings file expands neither, and would
/// replace the working value — so the report names both exports instead.
/// With `--approve`, the workspace named by `--workspace` has its
/// startup gate cleared as well.
#[derive(Parser)]
#[command(name = "ai-cli-readify", bin_name = "pixel ai-cli-readify")]
struct AiCliReadifyCli {
    #[command(flatten)]
    _host: HostFlags,
    /// Also rewrite the agents' config files; without this flag the
    /// command only probes and verifies (safe to re-run).
    #[arg(long)]
    apply: bool,
    /// Probe timeout per provider, in seconds.
    #[arg(long, default_value_t = 20)]
    timeout: u64,
    /// Verify only this subset of agents (repeatable). Default: all
    /// four (codex, claude, antigravity, devin).
    #[arg(long = "agent", value_enum)]
    agents: Vec<ai_cli_readify::AgentFlag>,
    /// Emit a machine-readable JSON report on stdout.
    #[arg(long)]
    json: bool,
    /// Answer the startup prompts this command knows the key for, instead
    /// of reporting them and stopping. Off by default: a trust dialog is
    /// not this command's decision to take, and a prompt with no verified
    /// answer is reported either way.
    #[arg(long)]
    answer_prompts: bool,
    /// Clear each agent's own startup gate for `--workspace`: Codex's
    /// workspace trust and its hooks' trust at their current hashes
    /// (through Codex's own `config/batchWrite` RPC, never by editing
    /// `config.toml`), and Claude's onboarding and trust dialog in
    /// `~/.claude.json`. Off by default — a trust write outlives the run,
    /// and it means "run this folder's hooks and code without asking
    /// again". Antigravity and Devin have no approval path: their trust
    /// state is only ever read. `~/.claude.json` belongs to Claude Code,
    /// so close a running one first: the file is read, merged and
    /// rewritten, and a write Claude Code makes in between is lost.
    #[arg(long)]
    approve: bool,
    /// The folder `--approve` is about. Defaults to the working
    /// directory. Exactly this path is written, never a parent: a trust
    /// level granted to a directory covers everything below it, so a run
    /// that walked up would grant more than it was asked to.
    #[arg(long, default_value = ".")]
    workspace: PathBuf,
    /// Hand a Claude lane stuck on an auth wall to the installed
    /// `claude-code-auth-flow`: spawn `claude auth login`, replay the
    /// flow with the authorize URL it prints (which drives `agent-browser`
    /// against the user's real browser profile), wait for the login to
    /// exit, then re-probe the lane once. Off by default, and it fires
    /// only for the two auth wordings — a quota, a rate limit, a
    /// transport error or a model-not-found failure leaves the browser
    /// untouched. A login that prints no authorize URL is refused rather
    /// than opened at an empty one.
    #[arg(long)]
    authenticate: bool,
    /// The account `--authenticate` should use: passed to
    /// `claude auth login --email` and to the flow's own account
    /// shortcut. Only consulted when the chain runs.
    #[arg(long)]
    account: Option<String>,
}

/// Entry point of `pixel ai-cli-readify` (the arguments after the command name).
pub fn ai_cli_readify(args: Vec<OsString>) -> ExitCode {
    run::<AiCliReadifyCli>("ai-cli-readify", args, |cli| {
        let opts = ai_cli_readify::Options {
            apply: cli.apply,
            timeout: Duration::from_secs(cli.timeout),
            agents: cli.agents.into_iter().map(Agent::from).collect(),
            answer_prompts: cli.answer_prompts,
            approve: cli.approve,
            workspace: cli.workspace,
            authenticate: cli.authenticate,
            account: cli.account,
        };
        let report = ai_cli_readify::run(&opts)?;
        if cli.json {
            let value = serde_json::to_value(&report).map_err(|e| e.to_string())?;
            print_data(&value, true)
        } else {
            ai_cli_readify::print_report(&report);
            Ok(())
        }
    })
}

// SPDX-FileCopyrightText: The Pixel contributors
// SPDX-License-Identifier: MIT

//! The three commands of the `utilities` plugin and how a name finds its code.

use std::ffi::OsString;
use std::process::ExitCode;

mod check_release;
mod readify;
mod web_search;

/// One command: the name `pixel <name>` is typed with, a line for `--help`,
/// and the entry point, which receives the arguments after the name.
pub struct Command {
    pub name: &'static str,
    pub about: &'static str,
    run: fn(Vec<OsString>) -> ExitCode,
}

impl Command {
    /// Run the command on `args` (the arguments after its name).
    pub fn run(&self, args: &[OsString]) -> ExitCode {
        let mut argv = Vec::with_capacity(args.len() + 1);
        argv.push(OsString::from(format!("pixel {}", self.name)));
        argv.extend_from_slice(args);
        (self.run)(argv)
    }
}

const fn command(
    name: &'static str,
    about: &'static str,
    run: fn(Vec<OsString>) -> ExitCode,
) -> Command {
    Command { name, about, run }
}

/// Every command, in the order `--help` lists them. This list is also the
/// `commands` array of `pixel-plugin.toml` (`tests/manifest.rs` compares the
/// two).
pub const COMMANDS: [Command; 3] = [
    command(
        "web-search",
        "Deterministic web lookup for terms the index cannot know (no LLM, no daemon)",
        web_search::web_search,
    ),
    command(
        "check-release",
        "Check that a release tag is consistent with the tree before it is built or published",
        check_release::check_release,
    ),
    command(
        "ai-cli-readify",
        "Probe provider readiness for Codex, Claude Code, Antigravity and Devin; --apply rewrites their configs",
        readify::ai_cli_readify,
    ),
];

/// The command named `name`, if this plugin provides it.
pub fn find(name: &str) -> Option<&'static Command> {
    COMMANDS.iter().find(|c| c.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_unique_and_found() {
        for (i, c) in COMMANDS.iter().enumerate() {
            assert!(find(c.name).is_some_and(|f| f.name == c.name));
            assert!(
                COMMANDS[i + 1..].iter().all(|o| o.name != c.name),
                "{}",
                c.name
            );
        }
        assert!(find("utilities").is_none());
        assert!(find("").is_none());
    }
}

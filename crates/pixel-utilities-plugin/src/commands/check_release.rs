// SPDX-FileCopyrightText: The Pixel contributors
// SPDX-License-Identifier: MIT

//! `pixel check-release`: check that a release tag is consistent with the
//! tree before anything is built or published.
//!
//! Ported from the `CheckRelease` command line and handler in
//! `crates/pixel/src/main.rs` of Pixel-CLI/pixel (MIT). The checks themselves
//! (`pixel_release::run`) and the report are core's `pixel-release` library,
//! which Pixel's own release workflow still builds with.

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use crate::output::{print_data, write_stdout};
use crate::run::{HostFlags, run};

/// Check that a release tag is consistent with the tree before anything
/// is built or published: crates/pixel/Cargo.toml carries the version,
/// Cargo.lock is fresh for every workspace member, CHANGELOG.md has the
/// `## [x.y.z]` heading and an empty Unreleased section. Exit 1 on any
/// failed check.
#[derive(Parser)]
#[command(name = "check-release", bin_name = "pixel check-release")]
struct CheckReleaseCli {
    #[command(flatten)]
    _host: HostFlags,
    /// The version or tag: `1.2.3`, `v1.2.3` or `refs/tags/v1.2.3`.
    version: String,
    /// Repository root (default: current directory).
    #[arg(long, default_value = ".")]
    repo: PathBuf,
    /// Emit the report as one JSON document.
    #[arg(long)]
    json: bool,
}

/// Entry point of `pixel check-release` (the arguments after the command name).
pub fn check_release(args: Vec<OsString>) -> ExitCode {
    run::<CheckReleaseCli>("check-release", args, |cli| {
        let report = pixel_release::run(&cli.repo, &cli.version)?;
        if cli.json {
            print_data(&report.to_json(), true)?;
        } else {
            write_stdout(&report.render())?;
        }
        if report.ok() {
            Ok(())
        } else {
            Err("release-check failed".to_string())
        }
    })
}

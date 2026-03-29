// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! Unit tests for the CLI module

use crate::cli::{Cli, Commands};
use clap::Parser;

// --- Subcommand parsing ---

#[test]
fn test_cli_parses_run_subcommand() {
    let cli = Cli::try_parse_from(["bindcar", "run"]).unwrap();
    assert_eq!(cli.command, Some(Commands::Run));
}

#[test]
fn test_cli_parses_drone_subcommand() {
    let cli = Cli::try_parse_from(["bindcar", "drone"]).unwrap();
    assert_eq!(cli.command, Some(Commands::Drone));
}

#[test]
fn test_cli_no_subcommand_stores_none() {
    let cli = Cli::try_parse_from(["bindcar"]).unwrap();
    assert_eq!(cli.command, None);
}

#[test]
fn test_cli_unknown_subcommand_returns_error() {
    let result = Cli::try_parse_from(["bindcar", "fly"]);
    assert!(result.is_err(), "Unknown subcommand should return an error");
}

// --- resolved_command() defaulting ---

#[test]
fn test_resolved_command_is_run_when_no_subcommand_given() {
    let cli = Cli::try_parse_from(["bindcar"]).unwrap();
    assert_eq!(
        cli.resolved_command(),
        &Commands::Run,
        "Missing subcommand should default to Run"
    );
}

#[test]
fn test_resolved_command_is_run_when_run_given() {
    let cli = Cli::try_parse_from(["bindcar", "run"]).unwrap();
    assert_eq!(cli.resolved_command(), &Commands::Run);
}

#[test]
fn test_resolved_command_is_drone_when_drone_given() {
    let cli = Cli::try_parse_from(["bindcar", "drone"]).unwrap();
    assert_eq!(cli.resolved_command(), &Commands::Drone);
}

// --- Help text sanity checks ---

#[test]
fn test_run_subcommand_help_mentions_sidecar() {
    use clap::CommandFactory;
    let mut cmd = Cli::command();
    let help = format!("{}", cmd.render_long_help());
    // The top-level help should mention both subcommands
    assert!(
        help.contains("run") || help.contains("drone"),
        "Help text should mention subcommands, got:\n{}",
        help
    );
}

// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! CLI argument parsing for the bindcar binary.
//!
//! bindcar supports two operating modes selected via subcommand:
//!
//! - **`run`** (default) — sidecar mode: runs alongside a local BIND9 instance inside a
//!   Kubernetes pod, communicating over shared volumes and local RNDC.
//!
//! - **`drone`** — standalone mode: runs as an independent process on a bare-metal or VM
//!   host, managing a remote BIND9 instance. Authentication is performed via the
//!   Kubernetes TokenReview API using explicit credentials (`KUBE_API_SERVER`,
//!   `KUBE_TOKEN_PATH`, `KUBE_CA_CERT_PATH`).
//!
//! When no subcommand is given, `run` is the default, preserving backwards compatibility
//! for existing deployments and process supervisors that call `bindcar` directly.

use clap::{Parser, Subcommand};

/// bindcar — HTTP REST API for managing BIND9 zones via RNDC
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Enable debug-level logging (overrides RUST_LOG)
    #[arg(long, short = 'd', global = true)]
    pub debug: bool,

    /// Acknowledge and allow starting with weak/disabled authentication on a
    /// non-loopback interface. Without this (or a loopback bind / real auth),
    /// bindcar refuses to start to avoid silently exposing an unauthenticated API.
    #[arg(long, global = true)]
    pub i_know_this_is_insecure: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// bindcar operating modes
#[derive(Subcommand, Debug, PartialEq, Clone)]
pub enum Commands {
    /// Run as a Kubernetes sidecar alongside a local BIND9 instance (default)
    Run,
    /// Run standalone, managing a remote BIND9 instance from outside the cluster
    Drone,
}

impl Cli {
    /// Return the resolved command, defaulting to [`Commands::Run`] when no subcommand
    /// was given. This preserves backwards compatibility: `bindcar` with no args
    /// behaves identically to `bindcar run`.
    pub fn resolved_command(&self) -> &Commands {
        self.command.as_ref().unwrap_or(&Commands::Run)
    }
}

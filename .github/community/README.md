# bindcar Roadmap Index

This directory holds bindcar's roadmap documents. Each one describes a body
of work — *what* and *why*, with a task list and a definition of done.
[`../../ROADMAPS.md`](../../ROADMAPS.md) is the status board that indexes
them and carries the current completion state.

## Numbering

Numbers are **sequential** and **stable once assigned**: a new roadmap takes
the next free number, whatever its subject. Numbers are never reused and
never renumbered — a link to `04` stays valid for the life of the repo.

Filenames are `NN-lowercase-hyphenated-title.md`. No uppercase, no
underscores.

The headings below group the documents by theme for reading; the grouping
carries no meaning for the numbering.

## Index

### Reference and analysis

| # | File | What |
|---|---|---|
| 01 | [`01-dnssec-feature-summary.md`](01-dnssec-feature-summary.md) | Record of the shipped DNSSEC implementation and its API surface |

### Architecture and refactoring

| # | File | What |
|---|---|---|
| 02 | [`02-feature-gate-http-server.md`](02-feature-gate-http-server.md) | Put the HTTP server behind a cargo feature so type-only consumers shed ~30 deps |

### Features

| # | File | What |
|---|---|---|
| 03 | [`03-bind9-full-zone-config.md`](03-bind9-full-zone-config.md) | Parse and serialize the full BIND9 9.18+ zone statement option set |
| 04 | [`04-standalone-out-of-cluster.md`](04-standalone-out-of-cluster.md) | Run bindcar outside Kubernetes against pre-existing BIND9, still on SA-token identity |

## Reserved numbers

`05` is **assigned but deliberately absent**. It covers an unremediated
transport-security weakness in shipped code, and this repository is public, so
it is tracked privately until that work ships. Do not reuse the number — the
next new roadmap takes `06`.

## Consumer upgrade guides live in bindy

The bindcar → bindy upgrade guides are **not** roadmaps and are not kept here.
They tell the *bindy operator* what to change to consume a bindcar release, so
they live in that repo's own index as
[`53`–`56`](https://github.com/firestoned/bindy/tree/main/.github/community).
When a bindcar release changes something a consumer must react to, write the
delta there, not in this directory.

## How these relate to the rest of the repo

- **Roadmaps say what and why.** An architecturally significant *how*
  still goes through an ADR (`docs/adr/NNNN-title.md`) first — a roadmap
  entry does not substitute for one.
- **Task lists are the source of truth.** Check items off in the file as
  they land, in the same PR that lands them.
- **Status changes go in `ROADMAPS.md`** in that same PR. That file is a
  board, not documentation of intent.
- Code style, testing and documentation rules live in
  [`.claude/rules/`](../../.claude/rules/), not here.

## Reading a migrated doc

Everything here was migrated on 2026-09-12 from an external roadmap set, and
each carries a `> **Status:**` block under its title recording what was
verified against the tree at that point. **The body below that block is the
document as originally written** — file paths and line numbers in older docs
have drifted. Trust the status block; re-verify the body.

## Adding a roadmap

1. Take the next free number (`06` today).
2. Filename: `NN-lowercase-hyphenated-title.md`.
3. Open with a `> **Status:**` block so a reader knows where the work stands
   before reading the analysis.
4. Add a row to the table above **and** to `ROADMAPS.md`.

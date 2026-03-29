# Documentation Standards

## Before Marking Any Task Complete

ALWAYS ask: "Does documentation need to be updated?"

Applies to: code changes, API changes, configuration changes, architecture changes.

---

## Documentation Update Workflow

1. **Analyze the change**: user-facing impact? architectural implications? new APIs/config?
2. **Update in this order:**
   - `.claude/CHANGELOG.md` (see `update-changelog` skill — `**Author:**` is MANDATORY)
   - `docs/src/` — affected user guides, quickstart, config references, troubleshooting
   - `examples/` — update to match new behavior
   - Architecture diagrams (Mermaid in `docs/src/`) if structure changed
   - `README.md` if getting-started or features changed
3. **Verify:** read docs as a new user, run `build-docs` skill

---

## What to Update by Change Type

**API handler changes** (`src/zones.rs`, `src/records.rs`, etc.):
- Update API reference docs
- Update request/response examples in `docs/src/reference/`
- Update troubleshooting guides

**Authentication changes** (`src/auth.rs`):
- Update authentication guide in `docs/src/`
- Update examples and quickstart

**Core logic changes** (`src/rndc.rs`, `src/nsupdate.rs`, etc.):
- Update architecture docs
- Update troubleshooting guides

**New features:**
- Add to `docs/src/user-guide/`, update `README.md`, add examples, add troubleshooting

**Bug fixes:**
- Update troubleshooting guides with the fix

---

## Building Documentation

**ALWAYS use `make docs`, never `mkdocs build` directly.**

> Run the `build-docs` skill.

---

## Changelog Requirements

Every entry in `.claude/CHANGELOG.md` MUST have `**Author:**` — no exceptions.

Format:
```markdown
## [YYYY-MM-DD HH:MM] - Brief Title

**Author:** <Name of requester or approver>

### Changed
- `path/to/file.rs`: Description of the change

### Why
Brief explanation.

### Impact
- [ ] Breaking change
- [ ] API change
- [ ] Config change only
- [ ] Documentation only
```

---

## Code Comments

All public functions and types MUST have rustdoc comments:

```rust
/// Creates a DNS zone by writing a zone file and registering with BIND9 via RNDC.
///
/// # Arguments
/// * `config` - Server configuration
/// * `zone_name` - Fully qualified domain name
///
/// # Errors
/// Returns `AppError::ZoneExists` if the zone already exists.
/// Returns `AppError::RndcFailed` if RNDC command fails.
pub async fn create_zone(config: &Config, zone_name: &str) -> Result<Zone, AppError> {
```

---

## Validation Checklist

- [ ] `.claude/CHANGELOG.md` updated with `**Author:**`
- [ ] All affected `docs/src/` pages updated
- [ ] Architecture diagrams updated if structure changed
- [ ] `make docs` succeeds

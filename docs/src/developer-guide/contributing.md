# Contributing to bindcar

Thank you for your interest in contributing to bindcar! This guide will help you get started.

## Code of Conduct

Be respectful and professional. We welcome contributors of all experience levels.

## Ways to Contribute

### Report Bugs

Found a bug? Please [open an issue](https://github.com/firestoned/bindcar/issues/new) with:

1. **Clear title** - Summarize the issue
2. **Environment** - OS, bindcar version, deployment method
3. **Steps to reproduce** - Detailed steps to trigger the bug
4. **Expected behavior** - What should happen
5. **Actual behavior** - What actually happens
6. **Logs** - Relevant error messages or logs
7. **Configuration** - Relevant environment variables or configuration

Example:
```markdown
### Bug Report: Zone deletion fails with 502 error

**Environment:**
- bindcar: v0.1.0
- Deployment: Kubernetes 1.28
- BIND9: 9.18

**Steps to reproduce:**
1. Create zone: `POST /api/v1/zones` with example.com
2. Delete zone: `DELETE /api/v1/zones/example.com`
3. Observe 502 error

**Expected:** Zone deleted successfully (204)
**Actual:** 502 Bad Gateway

**Logs:**
```
{"level":"error","message":"RNDC command failed","command":"delzone"}
```

**Configuration:**
- BIND_ZONE_DIR=/var/cache/bind
- RUST_LOG=debug
```

### Request Features

Have an idea? [Open a feature request](https://github.com/firestoned/bindcar/issues/new) with:

1. **Use case** - What problem does this solve?
2. **Proposed solution** - How should it work?
3. **Alternatives considered** - Other approaches you've thought about
4. **Additional context** - Screenshots, examples, etc.

Example:
```markdown
### Feature Request: Support for DNSSEC signing

**Use case:**
As a DNS operator, I want zones automatically signed with DNSSEC so that clients can verify DNS responses.

**Proposed solution:**
Add DNSSEC signing options to zone configuration:
```json
{
  "zoneName": "example.com",
  "zoneConfig": {
    "dnssec": {
      "enabled": true,
      "algorithm": "RSASHA256"
    }
  }
}
```

**Alternatives:**
- Manual signing with external tools
- Separate DNSSEC signer sidecar

**Additional context:**
Many enterprise deployments require DNSSEC for security compliance.
```

### Improve Documentation

Documentation improvements are always welcome:

1. Fix typos or unclear wording
2. Add examples or use cases
3. Improve API documentation
4. Add troubleshooting tips
5. Translate documentation (future)

To edit documentation:
```bash
# Edit files in docs/src/
vim docs/src/installation/index.md

# Build and preview
make docs-serve

# Open browser to http://localhost:3000
```

### Submit Code

Contributing code? Follow these guidelines:

## Development Setup

### Prerequisites

- Rust 1.89.0 or later
- Git
- Docker (optional, for testing)
- mdBook (for documentation)

### Clone and Build

```bash
# Fork the repository on GitHub
# Clone your fork
git clone https://github.com/YOUR_USERNAME/bindcar.git
cd bindcar

# Add upstream remote
git remote add upstream https://github.com/firestoned/bindcar.git

# Build
cargo build

# Run tests
cargo test

# Run locally
mkdir -p .tmp/zones
RUST_LOG=debug BIND_ZONE_DIR=.tmp/zones cargo run
```

## Development Workflow

### 1. Create a Branch

```bash
# Update main
git checkout main
git pull upstream main

# Create feature branch
git checkout -b feature/my-feature

# Or bug fix branch
git checkout -b fix/issue-123
```

### 2. Make Changes

Write your code following these guidelines:

#### Code Style

- **Format code**: Run `cargo fmt` before committing
- **Lint code**: Run `cargo clippy -- -D warnings`
- **Follow conventions**: Match existing code style
- **Add tests**: Write tests for new functionality
- **Document public APIs**: Use rustdoc comments

Example:
```rust
/// Creates a new DNS zone in BIND9.
///
/// # Arguments
///
/// * `zone_name` - The fully qualified domain name of the zone
/// * `zone_config` - Configuration for the zone
///
/// # Returns
///
/// Returns `Ok(())` if the zone was created successfully,
/// or an `Err` with details if the operation failed.
///
/// # Example
///
/// ```
/// let result = create_zone("example.com", &config).await?;
/// ```
pub async fn create_zone(
    zone_name: &str,
    zone_config: &ZoneConfig,
) -> Result<(), Error> {
    // Implementation
}
```

#### Testing

Write tests for:
- New features
- Bug fixes
- Edge cases
- Error conditions

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zone_name_validation() {
        assert!(is_valid_zone_name("example.com"));
        assert!(is_valid_zone_name("sub.example.com"));
        assert!(!is_valid_zone_name("invalid..com"));
        assert!(!is_valid_zone_name("example.com/"));
    }

    #[tokio::test]
    async fn test_create_zone_success() {
        let config = test_zone_config();
        let result = create_zone("test.example.com", &config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_zone_duplicate_fails() {
        let config = test_zone_config();
        
        // Create zone
        create_zone("test.example.com", &config).await.unwrap();
        
        // Attempt duplicate creation
        let result = create_zone("test.example.com", &config).await;
        assert!(result.is_err());
    }
}
```

#### Error Handling

- Use `Result<T, E>` for fallible operations
- Provide clear error messages
- Log errors appropriately
- Don't panic in library code

```rust
use tracing::error;

async fn reload_zone(zone_name: &str) -> Result<(), ZoneError> {
    let output = execute_rndc(&["reload", zone_name]).await?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!(
            zone = zone_name,
            error = %stderr,
            "Failed to reload zone"
        );
        return Err(ZoneError::RndcFailed(stderr.to_string()));
    }
    
    Ok(())
}
```

#### Logging

Use structured logging with `tracing`:

```rust
use tracing::{info, debug, error, instrument};

#[instrument(skip(config))]
async fn create_zone(
    zone_name: &str,
    zone_config: &ZoneConfig,
) -> Result<(), Error> {
    info!(zone = zone_name, "Creating zone");
    
    debug!(
        zone = zone_name,
        ttl = zone_config.ttl,
        "Zone configuration"
    );
    
    // ... implementation ...
    
    info!(zone = zone_name, "Zone created successfully");
    Ok(())
}
```

### 3. Run Quality Checks

Before committing, run:

```bash
# Format code
cargo fmt

# Run clippy
cargo clippy -- -D warnings

# Run tests
cargo test

# Check compilation
cargo check

# Or run all checks
make check
```

### 4. Commit Changes

Write clear, descriptive commit messages:

```bash
# Good commit messages
git commit -m "Add DNSSEC support for zone signing"
git commit -m "Fix zone deletion 502 error when BIND9 restarts"
git commit -m "Update documentation for authentication setup"

# Bad commit messages (avoid these)
git commit -m "Fix bug"
git commit -m "Update"
git commit -m "WIP"
```

**Commit message format:**
```
Short summary (50 chars or less)

More detailed explanation if needed. Wrap at 72 characters.

- Bullet points are okay
- Use present tense: "Add feature" not "Added feature"
- Reference issues: Fixes #123

Explain why the change is needed, not just what changed.
```

### 5. Push and Create Pull Request

```bash
# Push branch to your fork
git push origin feature/my-feature

# Create pull request on GitHub
# Fill out the PR template
```

## Pull Request Guidelines

### PR Title

Use clear, descriptive titles:

- `Add DNSSEC zone signing support`
- `Fix zone deletion 502 error`
- `Update installation documentation`
- `Refactor RNDC executor for better error handling`

### PR Description

Include:

1. **Summary** - What does this PR do?
2. **Motivation** - Why is this change needed?
3. **Changes** - What specifically changed?
4. **Testing** - How was this tested?
5. **Screenshots** - If UI/documentation changes
6. **Breaking changes** - Any backwards-incompatible changes?
7. **Closes** - Link related issues (e.g., "Closes #123")

Example:
```markdown
## Summary
Adds DNSSEC signing support for zones, allowing automatic signing with configurable algorithms.

## Motivation
Many enterprise deployments require DNSSEC for security compliance. This feature enables automatic zone signing via BIND9's inline-signing feature.

## Changes
- Add `dnssec` field to `ZoneConfig` struct
- Implement DNSSEC key generation via `rndc-confgen`
- Update zone file template to include DNSSEC policy
- Add integration tests for DNSSEC zones
- Update API documentation

## Testing
- Added unit tests for DNSSEC configuration validation
- Added integration tests for creating signed zones
- Manually tested with BIND9 9.18 in Kubernetes
- Verified DNSSEC signatures with `dig +dnssec`

## Breaking Changes
None - DNSSEC is optional and disabled by default.

## Closes
Closes #45
```

### PR Checklist

Before submitting, ensure:

- [ ] Code follows project style guidelines
- [ ] `cargo fmt` has been run
- [ ] `cargo clippy` passes with no warnings
- [ ] All tests pass (`cargo test`)
- [ ] New tests added for new functionality
- [ ] Documentation updated if needed
- [ ] Commit messages are clear and descriptive
- [ ] PR description explains the changes
- [ ] No merge conflicts with main branch

## Review Process

### What to Expect

1. **Automated Checks** - CI runs tests and linters
2. **Code Review** - Maintainer reviews your code
3. **Feedback** - You may receive change requests
4. **Iteration** - Make requested changes
5. **Approval** - Once approved, PR is merged

### Responding to Feedback

- Be open to suggestions
- Ask questions if feedback is unclear
- Make requested changes promptly
- Update your branch if main has changed

```bash
# Update your branch with latest main
git checkout main
git pull upstream main
git checkout feature/my-feature
git rebase main

# Push updated branch (may need force push)
git push origin feature/my-feature --force-with-lease
```

## Release Process

Releases are managed by maintainers:

1. Version bump in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Create git tag
4. Build and publish Docker image
5. Create GitHub release

## Getting Help

Need help contributing?

- **Questions**: Open a [discussion](https://github.com/firestoned/bindcar/discussions)
- **Chat**: Join our community (TBD)
- **Email**: Contact maintainers (TBD)

## Recognition

Contributors are recognized in:
- `CONTRIBUTORS.md` file
- Release notes
- Project README

Thank you for contributing to bindcar!

## Next Steps

- [Development Setup](./dev-setup/index.md) - Set up development environment
- [Architecture](./architecture.md) - Understand the codebase
- [API Reference](./api-reference/index.md) - API documentation

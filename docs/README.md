# bindcar Documentation

This directory contains the comprehensive documentation for bindcar, the HTTP REST API server for managing BIND9 zones via rndc commands.

## Documentation Structure

- **`src/`** - mdBook source files for user and developer documentation
- **`target/`** - Build output (gitignored)

## Building Documentation

### Prerequisites

1. **mdBook** - Install with cargo:
   ```bash
   cargo install mdbook
   ```

2. **Rust toolchain** - For generating API documentation

### Build All Documentation

Build both mdBook and rustdoc:

```bash
make docs
```

This creates a combined documentation site in `docs/target/` with:
- User and developer guides (mdBook)
- API reference (rustdoc) at `/rustdoc/`

### Build and Serve Locally

Build and serve documentation at http://localhost:3000:

```bash
make docs-serve
```

Or use mdBook's built-in server with live reload:

```bash
make docs-watch
```

This serves at http://localhost:3000 and automatically rebuilds on changes.

### Build Individual Components

Build only mdBook documentation:

```bash
make docs-mdbook
```

Build only rustdoc API documentation:

```bash
make docs-rustdoc
```

## Documentation Organization

### User Documentation

Located in `src/`:

- **Getting Started**
  - Installation guides
  - Quick start tutorial
  - Configuration

- **User Guide**
  - API overview
  - Creating and managing zones
  - Zone operations

- **Operations**
  - Deployment (Docker, Kubernetes)
  - Monitoring and logging
  - Troubleshooting

### Developer Documentation

- **Development Setup**
  - Building from source
  - Running tests
  - Development workflow

- **Architecture**
  - API design
  - RNDC integration

- **Contributing**
  - Code guidelines

### API Reference

Generated from Rust source code documentation comments using rustdoc.

## Writing Documentation

### mdBook Pages

Create new pages in `src/` and add them to `src/SUMMARY.md`.

Use Markdown with these extensions:
- GitHub-flavored Markdown
- Code blocks with syntax highlighting
- Tables
- Task lists

Example:

```markdown
# Page Title

Introduction paragraph.

## Section

Content with `inline code` and:

\`\`\`yaml
# YAML code block
apiVersion: v1
kind: Service
\`\`\`

See [other page](./other-page.md) for more.
```

### API Documentation

Add documentation comments to Rust code:

```rust
/// Brief description of the function.
///
/// More detailed explanation with examples:
///
/// # Examples
///
/// \`\`\`
/// use bindcar::zones::create_zone;
/// let zone = create_zone("example.com");
/// \`\`\`
///
/// # Errors
///
/// Returns an error if...
pub fn example() -> Result<(), Error> {
    // ...
}
```

## GitHub Pages Deployment

Documentation is automatically built and deployed to GitHub Pages on every push to the `main` branch.

The live documentation will be available at: https://firestoned.github.io/bindcar/

## Troubleshooting

### mdBook not found

Install mdBook:

```bash
cargo install mdbook
```

### Python not found (for docs-serve)

The `docs-serve` target uses Python's built-in HTTP server. Install Python 3 or use:

```bash
make docs-watch
```

This uses mdBook's built-in server instead.

### Documentation not updating

Clean and rebuild:

```bash
make docs-clean
make docs
```

## Contributing to Documentation

1. Make changes to files in `src/`
2. Test locally: `make docs-watch`
3. Verify the changes look correct
4. Commit and create a pull request

Documentation follows the same contribution guidelines as code:
- Clear, concise writing
- Proper grammar and spelling
- Tested code examples
- Linked cross-references

## Resources

- [mdBook Documentation](https://rust-lang.github.io/mdBook/)
- [Rustdoc Book](https://doc.rust-lang.org/rustdoc/)
- [Markdown Guide](https://www.markdownguide.org/)

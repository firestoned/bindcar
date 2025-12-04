# API Documentation (rustdoc)

The complete Rust API documentation is generated from the source code using rustdoc.

## View API Documentation

You can access the rustdoc API documentation at:

**[rustdoc/bindcar/index.html](rustdoc/bindcar/index.html)**

## Building API Documentation

To build and view the API documentation locally:

```bash
make docs-rustdoc
```

This will build the documentation and open it in your default browser.

## What's Included

The rustdoc documentation includes:

- **Public APIs** - All public modules, structs, and functions
- **Type Definitions** - Complete type information
- **Examples** - Code examples from documentation comments
- **Source Links** - Links to source code

## Key Modules

- **`bindcar::zones`** - Zone management functionality
- **`bindcar::rndc`** - RNDC command execution
- **`bindcar::auth`** - Authentication middleware
- **`bindcar::main`** - API server and routing

## Navigation

Use the search bar to quickly find specific functions, types, or modules. The left sidebar provides a structured view of all public APIs.

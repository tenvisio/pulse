# Contributing to Pulse

Thank you for your interest in contributing to Pulse! This document provides guidelines and information for contributors.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Making Changes](#making-changes)
- [Code Style](#code-style)
- [Commit Messages](#commit-messages)
- [Pull Request Process](#pull-request-process)
- [Good First Issues](#good-first-issues)

## Code of Conduct

By participating in this project, you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md).

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/pulse.git
   cd pulse
   ```
3. **Add the upstream remote**:
   ```bash
   git remote add upstream https://github.com/tenvisio/pulse.git
   ```

## Development Setup

### Prerequisites

- Rust 1.75 or later (we use `rust-toolchain.toml`)
- Git

### Building

```bash
# Build all crates
cargo build

# Build in release mode
cargo build --release

# Build specific crate
cargo build -p pulse-server
```

### Testing

```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p pulse-core

# Run tests with output
cargo test -- --nocapture
```

### Linting

```bash
# Format code
cargo fmt

# Check formatting (CI will fail if this fails)
cargo fmt --check

# Run clippy
cargo clippy -- -D warnings
```

### Benchmarks

```bash
# Run micro-benchmarks (Criterion)
cargo bench -p pulse-bench

# Run specific benchmark file
cargo bench -p pulse-bench --bench throughput
cargo bench -p pulse-bench --bench latency

# Run protocol codec benchmarks
cargo bench -p pulse-protocol

# Run end-to-end throughput test (requires server running in another terminal)
cargo run --release -p pulse-server  # Terminal 1
cargo run --release -p pulse-bench --bin e2e_throughput  # Terminal 2
```

## Making Changes

1. **Create a branch** from `main`:
   ```bash
   git checkout -b feature/your-feature-name
   # or
   git checkout -b fix/issue-description
   ```

2. **Make your changes** following our code style guidelines

3. **Test your changes**:
   ```bash
   cargo test
   cargo clippy -- -D warnings
   cargo fmt --check
   ```

4. **Commit your changes** using conventional commits

5. **Push to your fork** and create a pull request

## Code Style

We follow standard Rust conventions with some additions:

### General Guidelines

- Use `rustfmt` for formatting (configured in `.rustfmt.toml` if present)
- All public items must have documentation
- Prefer descriptive variable names over comments
- Keep functions focused and small
- Use `#[must_use]` for functions whose return values shouldn't be ignored

### Error Handling

- Use `thiserror` for library error types
- Use `anyhow` only in binary crates
- Provide context with errors where helpful

### Documentation

- All public APIs must have doc comments
- Include examples in doc comments where appropriate
- Use `#![deny(missing_docs)]` in library crates

### Performance

- Prefer zero-copy operations with `Bytes`
- Use `#[inline]` judiciously for hot paths
- Benchmark before optimizing

## Commit Messages

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

### Types

- `feat`: A new feature
- `fix`: A bug fix
- `docs`: Documentation only changes
- `style`: Formatting, missing semi colons, etc.
- `refactor`: Code change that neither fixes a bug nor adds a feature
- `perf`: Performance improvement
- `test`: Adding missing tests
- `chore`: Maintenance tasks

### Examples

```
feat(protocol): add heartbeat frame type

fix(router): prevent panic on empty channel name

docs(readme): add benchmarks section

perf(transport): use zero-copy frame parsing
```

## Pull Request Process

1. **Ensure CI passes** - All checks must be green
2. **Update documentation** - If you changed APIs
3. **Add tests** - For new features or bug fixes
4. **Request review** - From at least one maintainer
5. **Address feedback** - Make requested changes
6. **Squash if needed** - Keep history clean

### PR Title

Use the same format as commit messages:
```
feat(core): implement presence tracking
```

### PR Description

Include:
- What changes were made
- Why the changes were needed
- How to test the changes
- Related issues (use `Fixes #123` or `Closes #123`)

## Good First Issues

New to Pulse? Look for issues labeled [`good first issue`](https://github.com/tenvisio/pulse/labels/good%20first%20issue). These are:

- Well-defined scope
- Low complexity
- Good learning opportunities

### Suggestions for First Contributions

- Add documentation or examples
- Fix typos
- Add missing tests
- Small bug fixes
- Performance improvements with benchmarks

## Questions?

- Open a [Discussion](https://github.com/tenvisio/pulse/discussions) for general questions
- Create an [Issue](https://github.com/tenvisio/pulse/issues) for bugs or feature requests

Thank you for contributing! 🚀



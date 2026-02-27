# Development Environment Setup

This guide will help you set up a complete development environment for MAPLE.

## Prerequisites

### Required

- **Rust 1.80 or higher**: [Install Rust](https://www.rust-lang.org/tools/install)
- **Git**: Version control
- **A code editor**: VS Code, IntelliJ IDEA, or similar

### Optional

- **PostgreSQL 14+**: For persistence features
- **Docker**: For running services in containers
- **Redis**: For distributed caching (future)

## Installation

### 1. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Verify installation:
```bash
rustc --version
cargo --version
```

### 2. Install Development Tools

```bash
# Rust formatter
rustup component add rustfmt

# Rust linter
rustup component add clippy

# Rust Language Server (for IDE support)
rustup component add rust-analyzer
```

### 3. Clone Repository

```bash
git clone https://github.com/mapleaiorg/maple.git
cd maple
```

### 4. Build Project

```bash
# Build entire workspace
cargo build --workspace

# Build specific crate
cargo build -p maple-runtime

# Build in release mode (optimized)
cargo build --workspace --release
```

## Development Workflow

### Building

```bash
# Build all crates
cargo build --workspace

# Build with all features
cargo build --workspace --all-features

# Check without building (faster)
cargo check --workspace
```

### Testing

```bash
# Run all tests
cargo test --workspace

# Run tests for specific crate
cargo test -p maple-runtime

# Run specific test
cargo test test_resonator_registration

# Run tests with output
cargo test -- --nocapture

# Run doc tests
cargo test --doc
```

### Linting and Formatting

```bash
# Format code
cargo fmt --all

# Check formatting
cargo fmt --all -- --check

# Run clippy
cargo clippy --workspace -- -D warnings

# Fix clippy warnings automatically (when possible)
cargo clippy --workspace --fix
```

### Documentation

```bash
# Generate and open documentation
cargo doc --workspace --no-deps --open

# Generate documentation for specific crate
cargo doc -p maple-runtime --open

# Check documentation
cargo doc --workspace --no-deps
```

### Running Examples

```bash
# List examples
ls crates/maple-runtime/examples/

# Run example
cargo run -p maple-runtime --example 01_basic_resonator

# Run example in release mode
cargo run -p maple-runtime --example 02_resonator_coupling --release
```

## IDE Setup

### Visual Studio Code

#### Recommended Extensions

- **rust-analyzer**: Language server for Rust
- **CodeLLDB**: Debugger
- **crates**: Dependency management
- **Error Lens**: Inline error display
- **Better TOML**: TOML file support

#### Configuration

Create `.vscode/settings.json`:

```json
{
  "rust-analyzer.cargo.allFeatures": true,
  "rust-analyzer.checkOnSave.command": "clippy",
  "rust-analyzer.inlayHints.enable": true,
  "editor.formatOnSave": true,
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  }
}
```

Create `.vscode/launch.json` for debugging:

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug Example",
      "cargo": {
        "args": [
          "build",
          "-p", "maple-runtime",
          "--example", "01_basic_resonator"
        ]
      },
      "program": "${workspaceFolder}/target/debug/examples/01_basic_resonator"
    }
  ]
}
```

### IntelliJ IDEA / CLion

1. Install **Rust plugin**
2. Open project directory
3. IDE will auto-detect Cargo workspace
4. Configure run configurations for examples

## Database Setup (Optional)

### PostgreSQL

For persistence features:

```bash
# Install PostgreSQL (macOS)
brew install postgresql@14

# Start PostgreSQL
brew services start postgresql@14

# Create database
createdb maple_dev

# Set environment variable
export DATABASE_URL="postgresql://localhost/maple_dev"
```

### Running with Docker

```bash
# Start PostgreSQL in Docker
docker run --name maple-postgres \
  -e POSTGRES_PASSWORD=maple \
  -e POSTGRES_DB=maple_dev \
  -p 5432:5432 \
  -d postgres:14

# Set environment variable
export DATABASE_URL="postgresql://postgres:maple@localhost:5432/maple_dev"
```

## Environment Variables

Create `.env` file in project root:

```bash
# Database
DATABASE_URL=postgresql://localhost/maple_dev

# Logging
RUST_LOG=maple_runtime=debug,maple_integration=debug

# Testing
TEST_DATABASE_URL=postgresql://localhost/maple_test
```

## Common Development Tasks

### Create New Crate

```bash
# In crates/ directory
cd crates
cargo new my-new-crate --lib

# Add to workspace Cargo.toml
# [workspace]
# members = [
#     "crates/my-new-crate",
#     # ...
# ]
```

### Add Dependency

```bash
# Add to specific crate
cargo add tokio -p maple-runtime --features full

# Add dev dependency
cargo add --dev criterion -p maple-runtime
```

### Run Benchmarks

```bash
# Run benchmarks
cargo bench -p maple-runtime

# Run specific benchmark
cargo bench -p maple-runtime coupling_establishment
```

### Generate Flamegraph

```bash
# Install cargo-flamegraph
cargo install flamegraph

# Generate flamegraph
cargo flamegraph -p maple-runtime --example 02_resonator_coupling
```

## Troubleshooting

### Build Errors

**Issue**: Dependency resolution errors

```bash
# Update dependencies
cargo update

# Clean and rebuild
cargo clean
cargo build --workspace
```

**Issue**: Compilation errors after git pull

```bash
# Clean build artifacts
cargo clean

# Rebuild
cargo build --workspace
```

### Test Failures

**Issue**: Tests pass individually but fail when run together

```bash
# Run tests serially
cargo test --workspace -- --test-threads=1
```

**Issue**: Database tests failing

```bash
# Reset test database
dropdb maple_test
createdb maple_test

# Run tests
cargo test --workspace
```

### Performance Issues

**Issue**: Slow compile times

```bash
# Use cargo check instead of build during development
cargo check --workspace

# Enable parallel compilation
export CARGO_BUILD_JOBS=8
```

**Issue**: Tests taking too long

```bash
# Run only fast tests
cargo test --workspace --lib

# Skip integration tests
cargo test --workspace --bins
```

## Best Practices

### Development Cycle

1. **Write test first** (TDD approach)
2. **Implement functionality**
3. **Run tests**: `cargo test`
4. **Check formatting**: `cargo fmt --check`
5. **Run clippy**: `cargo clippy`
6. **Update documentation**
7. **Commit changes**

### Before Committing

```bash
# Run full check
./scripts/pre-commit.sh

# Or manually:
cargo fmt --all --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
cargo doc --workspace --no-deps
```

### Before Pull Request

```bash
# Ensure everything is clean
cargo clean
cargo build --workspace --all-features
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all --check
```

## Useful Commands

```bash
# Show crate dependency tree
cargo tree -p maple-runtime

# Show outdated dependencies
cargo outdated

# Audit dependencies for security vulnerabilities
cargo audit

# Show unused dependencies
cargo +nightly udeps --workspace

# Expand macros
cargo expand -p maple-runtime

# Show assembly
cargo asm -p maple-runtime function_name
```

## Resources

- **Rust Book**: https://doc.rust-lang.org/book/
- **Rust by Example**: https://doc.rust-lang.org/rust-by-example/
- **Async Book**: https://rust-lang.github.io/async-book/
- **API Guidelines**: https://rust-lang.github.io/api-guidelines/
- **MAPLE Docs**: https://docs.mapleai.org

## Getting Help

- **Discord**: [discord.gg/maple-ai](https://discord.gg/maple-ai)
- **GitHub Discussions**: [github.com/mapleaiorg/maple/discussions](https://github.com/mapleaiorg/maple/discussions)
- **Issues**: [github.com/mapleaiorg/maple/issues](https://github.com/mapleaiorg/maple/issues)

---

**Happy coding!** 🍁

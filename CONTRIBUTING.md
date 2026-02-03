# Contributing to MAPLE

Thank you for your interest in contributing to MAPLE! This document provides guidelines and instructions for contributing to the project.

## Code of Conduct

We are committed to providing a welcoming and inclusive environment. All contributors are expected to:

- Be respectful and considerate
- Welcome newcomers and help them get started
- Provide constructive feedback
- Focus on what is best for the community
- Show empathy towards other community members

## How to Contribute

### Reporting Bugs

Before creating a bug report:
1. Check the [issue tracker](https://github.com/mapleaiorg/maple/issues) for existing reports
2. Update to the latest version to see if the issue persists
3. Collect relevant information (OS, Rust version, error messages)

**Good bug reports include:**
- Clear, descriptive title
- Steps to reproduce the issue
- Expected vs. actual behavior
- Code samples or test cases
- Error messages and stack traces
- Environment details

### Suggesting Enhancements

Enhancement suggestions are welcome! Please include:
- Clear description of the enhancement
- Use cases and motivation
- Examples of how it would work
- Any implementation ideas (optional)

### Pull Requests

1. **Fork and clone** the repository
2. **Create a branch** for your changes: `git checkout -b feature/my-feature`
3. **Make your changes** following our coding standards
4. **Add tests** for new functionality
5. **Update documentation** as needed
6. **Run tests**: `cargo test --workspace`
7. **Run clippy**: `cargo clippy --workspace -- -D warnings`
8. **Format code**: `cargo fmt --all`
9. **Commit changes** with clear messages
10. **Push** to your fork: `git push origin feature/my-feature`
11. **Open a Pull Request** with a clear description

## Development Setup

### Prerequisites

- **Rust 1.75+**: [Install Rust](https://www.rust-lang.org/tools/install)
- **Git**: Version control
- **PostgreSQL** (optional): For persistence features

### Building from Source

```bash
# Clone the repository
git clone https://github.com/mapleaiorg/maple.git
cd maple

# Build the workspace
cargo build --workspace

# Run tests
cargo test --workspace

# Build documentation
cargo doc --workspace --no-deps --open
```

### Running Examples

```bash
# Basic resonator
cargo run -p maple-runtime --example 01_basic_resonator

# Coupling dynamics
cargo run -p maple-runtime --example 02_resonator_coupling

# Platform configurations
cargo run -p maple-runtime --example 03_mapleverse_config
cargo run -p maple-runtime --example 04_finalverse_config
cargo run -p maple-runtime --example 05_ibank_config
```

## Coding Standards

### Rust Style

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Keep functions focused and concise
- Write clear, descriptive names

### Documentation

- **All public items must be documented**
- Use doc comments (`///`) for public APIs
- Include examples in documentation
- Explain *why*, not just *what*
- Link to related items

Example:
```rust
/// Creates a new coupling between two Resonators.
///
/// This establishes a stateful relationship that must strengthen gradually
/// according to architectural invariants. Attention is allocated before
/// the coupling is created.
///
/// # Example
///
/// ```
/// let coupling = resonator_a.couple_with(
///     resonator_b.id,
///     CouplingParams::default()
/// ).await?;
/// ```
///
/// # Errors
///
/// Returns `CouplingError::InsufficientAttention` if the source Resonator
/// doesn't have enough available attention.
///
/// # See Also
///
/// - [`CouplingParams`] for configuration options
/// - [`AttentionAllocator`] for resource management
pub async fn couple_with(&self, target: ResonatorId, params: CouplingParams)
    -> Result<CouplingHandle, CouplingError>
{
    // ...
}
```

### Testing

- **Write tests for all new functionality**
- **Unit tests** in the same file as the code
- **Integration tests** in `tests/` directory
- **Doc tests** for examples in documentation
- Aim for >80% code coverage

Example:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resonator_registration() {
        let config = RuntimeConfig::default();
        let runtime = MapleRuntime::bootstrap(config).await.unwrap();

        let spec = ResonatorSpec::default();
        let resonator = runtime.register_resonator(spec).await.unwrap();

        assert!(!resonator.id.to_string().is_empty());

        runtime.shutdown().await.unwrap();
    }
}
```

### Error Handling

- Use `Result<T, E>` for fallible operations
- Create specific error types
- Provide helpful error messages
- Never use `unwrap()` or `expect()` in library code

```rust
pub enum CouplingError {
    /// Insufficient attention available for coupling
    InsufficientAttention { required: f64, available: f64 },

    /// Target Resonator not found
    TargetNotFound(ResonatorId),

    /// Coupling strength exceeds maximum allowed
    StrengthTooHigh { requested: f64, maximum: f64 },

    // ...
}
```

## Project Structure

```
maple/
├── crates/
│   ├── maple-runtime/       # Core runtime
│   ├── resonator-types/     # Resonator types
│   ├── resonator-runtime/   # Resonator execution
│   ├── rcf-types/           # RCF type system
│   ├── mrp-types/           # MRP types
│   ├── aas-types/           # AAS types
│   └── ...
├── docs/
│   ├── getting-started.md   # Getting started guide
│   ├── architecture.md      # Architecture overview
│   ├── concepts/            # Core concepts
│   ├── platforms/           # Platform guides
│   └── adr/                 # Architecture Decision Records
├── examples/                # Example applications
└── tests/                   # Integration tests
```

## Commit Messages

Use clear, descriptive commit messages following this format:

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `test`: Test additions or changes
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `chore`: Build process or auxiliary tool changes

**Example:**
```
feat(coupling): Add gradual strengthening enforcement

Implement architectural invariant that coupling strength cannot
increase by more than 0.1 per strengthening operation. This
prevents aggressive coupling that could exhaust attention rapidly.

Closes #123
```

## Areas for Contribution

### High Priority

- **Cognitive Pipeline**: Meaning, Intent, Commitment engines
- **Persistence Layer**: Database integration for continuity
- **Distributed Runtime**: Multi-node federation
- **Performance Optimization**: Benchmarking and optimization
- **Documentation**: Examples, tutorials, guides

### Medium Priority

- **Web UI Dashboard**: Runtime monitoring interface
- **Additional Examples**: More use cases
- **Platform Integrations**: External system connectors
- **Testing**: Increase test coverage

### Low Priority

- **WASM Support**: Browser deployment
- **Mobile SDKs**: iOS/Android support
- **Additional Language Bindings**: Python, JavaScript, etc.

## Questions?

- **GitHub Discussions**: [discussions](https://github.com/mapleaiorg/maple/discussions)
- **Discord**: [discord.gg/maple-ai](https://discord.gg/maple-ai)
- **Email**: hello@maple.ai

## License

By contributing to MAPLE, you agree that your contributions will be licensed under either:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at the option of the licensee.

---

**Thank you for contributing to MAPLE!** 🍁

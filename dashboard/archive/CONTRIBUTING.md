# Contributing to Veilweaver: Threads of Eternity

Thank you for considering contributing to Veilweaver! This document provides guidelines and instructions for contributing to the project.

## Code of Conduct

By participating in this project, you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md).

## How Can I Contribute?

### Reporting Bugs

- **Ensure the bug was not already reported** by searching on GitHub under [Issues](https://github.com/lazyxeon/Veilweaver-Threads-of-Eternity/issues).
- If you're unable to find an open issue addressing the problem, [open a new one](https://github.com/lazyxeon/Veilweaver-Threads-of-Eternity/issues/new/choose) using the bug report template.
- **For security vulnerabilities**, please follow our [Security Policy](../../SECURITY.md) instead of opening a public issue.

### Suggesting Enhancements

- [Open a new issue](https://github.com/lazyxeon/Veilweaver-Threads-of-Eternity/issues/new/choose) using the feature request template.
- Clearly describe the enhancement, its benefits, and potential implementation approaches.

### Pull Requests

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Security Best Practices

When contributing code, please follow these security best practices:

### General Security Guidelines

1. **Keep dependencies updated** to their latest secure versions
2. **Validate all user inputs** before processing
3. **Follow the principle of least privilege** when implementing new features
4. **Use safe Rust practices** and avoid unsafe code blocks when possible
5. **Run security checks locally** before submitting pull requests

### Rust-Specific Security Guidelines

1. **Minimize use of `unsafe` code**
   - Only use `unsafe` when absolutely necessary
   - Document why the unsafe code is needed and why it's safe
   - Consider alternatives before using unsafe code

2. **Handle errors properly**
   - Use `Result` and `Option` types appropriately
   - Don't use `.unwrap()` or `.expect()` in production code
   - Implement proper error handling and propagation

3. **Prevent memory safety issues**
   - Avoid raw pointers when possible
   - Use Rust's ownership system correctly
   - Be careful with lifetime annotations

4. **Secure resource management**
   - Ensure resources are properly cleaned up
   - Use RAII patterns with `Drop` trait
   - Be cautious with manual resource management

5. **Secure concurrency**
   - Use Rust's thread safety mechanisms
   - Avoid data races with proper synchronization
   - Be careful with shared mutable state

### Before Submitting a Pull Request

Run these security checks locally:

```bash
# Update dependencies
cargo update

# Check for security vulnerabilities
cargo audit

# Check for license compliance
cargo deny check

# Run the static analyzer
cargo clippy --all-targets --all-features -- -D warnings

# Run tests
cargo test --all-features
```

## Development Environment Setup

1. Install Rust (nightly toolchain recommended):
   ```bash
   rustup toolchain install nightly
   rustup default nightly
   ```

2. Install development tools:
   ```bash
   cargo install cargo-audit cargo-deny cargo-criterion cargo-llvm-cov
   ```

3. Clone the repository:
   ```bash
   git clone https://github.com/lazyxeon/Veilweaver-Threads-of-Eternity.git
   cd Veilweaver-Threads-of-Eternity
   ```

4. Build the project:
   ```bash
   cargo build
   ```

5. Run tests:
   ```bash
   cargo test
   ```

## Style Guidelines

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `rustfmt` to format your code
- Use `clippy` to catch common mistakes and improve your code
- Write documentation for public APIs
- Include tests for new functionality

## Commit Messages

- Use the present tense ("Add feature" not "Added feature")
- Use the imperative mood ("Move cursor to..." not "Moves cursor to...")
- Limit the first line to 72 characters or less
- Reference issues and pull requests liberally after the first line

## Additional Resources

- [Rust Documentation](https://doc.rust-lang.org/book/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Rust Security](https://www.rust-lang.org/security.html)

Thank you for contributing to Veilweaver: Threads of Eternity!
# Contributing to Kidur

Thank you for your interest in contributing.

## Before you start

By submitting a pull request, you agree to the contributor license terms
described in [`LICENSE-ENTERPRISE`](LICENSE-ENTERPRISE): you grant Evobiosys
a license to include your contribution in both the AGPL and commercial
versions of the software. If you cannot agree to this, please open an issue
to discuss before writing code.

## Development

```bash
# Run all tests
cargo test --workspace

# Check formatting
cargo fmt --check

# Lint
cargo clippy --workspace
```

## Pull requests

- One logical change per PR
- Tests required for new behavior
- Keep commits clean: `feat:`, `fix:`, `refactor:`, `docs:` prefixes

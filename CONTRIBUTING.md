# Contributing

Thanks for contributing to `cryptoprice`.

## Development Setup

1. Install stable Rust (edition 2024 compatible).
2. Clone the repository.
3. Build once to confirm your environment:

```sh
cargo build
```

## Local Quality Checks

Run the project CI script before committing:

```sh
bash ./scripts/ci.sh
```

This runs:

- `cargo fmt --all --check`
- `cargo clippy --locked --all-targets --all-features -- -D warnings`
- `cargo test --locked`

## Pre-commit Hook (Recommended)

Enable the repository-managed hook so checks run automatically on commit:

```sh
git config core.hooksPath .githooks
```

## Code Guidelines

- Keep network I/O async; do not use blocking HTTP clients.
- Use the project error type and avoid `unwrap()` in non-test code.
- Keep modules focused and aligned with current architecture.
- Add or update tests with behavior changes.

## Pull Requests

- Keep PRs focused and small when possible.
- Include a clear description of what changed and why.
- Ensure CI passes before requesting review.

## Release Notes

Tag releases with a `v*` tag (for example `v0.2.0`).

Tag pushes trigger automated release workflows that:

- run tests,
- publish platform binaries,
- publish Docker images to GHCR.

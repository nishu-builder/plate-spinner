# Contributing to Plate-Spinner

## Feature Requests

Feature requests can be submitted as PRs that add a spec to `docs/desired-features/`. See existing files there for the format. This approach encourages thinking through the problem and solution before implementation.

## Bug Reports

Open an issue with:
- Steps to reproduce
- Expected vs actual behavior
- OS and plate-spinner version (`sp --version`)

## Pull Requests

1. Fork the repo and create a branch
2. Make your changes
3. Run tests and linting:
   ```bash
   cargo test
   cargo fmt --all -- --check
   cargo clippy --all-targets --all-features -- -D warnings
   ```
4. Submit a PR with a clear description of the change

## Development Setup

```bash
git clone https://github.com/nishu-builder/plate-spinner
cd plate-spinner
cargo build
```

The daemon auto-restarts when the binary changes, so `cargo build && sp` picks up your changes.

## Code Style

- Run `cargo fmt` before committing
- No warnings from `cargo clippy`
- Keep commits atomic and focused

## Git Hooks

Install pre-commit hooks to catch issues early:

```bash
./scripts/install-hooks.sh
```

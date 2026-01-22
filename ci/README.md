# Release Process

This project uses [cargo-dist](https://opensource.axo.dev/cargo-dist/) for automated releases.

## How It Works

When you push a version tag (e.g., `v0.2.0`), GitHub Actions automatically:

1. Builds binaries for all platforms (macOS, Linux, Windows)
2. Creates installer scripts (shell for Unix, PowerShell for Windows)
3. Publishes a GitHub Release with all artifacts

## Creating a Release

### Option 1: Use the release script

```bash
./ci/release.sh 0.2.0
git push origin main
git push origin v0.2.0
```

### Option 2: Manual steps

1. Update `version` in `Cargo.toml`
2. Commit: `git commit -am "Release 0.2.0"`
3. Tag: `git tag v0.2.0`
4. Push: `git push origin main && git push origin v0.2.0`

## Installation Commands

After a release, users can install with:

```bash
# macOS/Linux
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/nishu-builder/plate-spinner/releases/latest/download/plate-spinner-installer.sh | sh

# Windows PowerShell
irm https://github.com/nishu-builder/plate-spinner/releases/latest/download/plate-spinner-installer.ps1 | iex
```

Or download binaries directly from the [releases page](https://github.com/nishu-builder/plate-spinner/releases).

## Configuration

Release configuration lives in:

- `dist-workspace.toml` - cargo-dist settings (targets, installers)
- `.github/workflows/release.yml` - CI workflow (auto-generated, don't edit manually)

To modify settings, edit `dist-workspace.toml` and run `dist generate` to regenerate the workflow.

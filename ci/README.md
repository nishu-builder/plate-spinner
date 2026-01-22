# Release Process

This project uses [cargo-dist](https://opensource.axo.dev/cargo-dist/) for automated releases.

## Creating a Release

```bash
./ci/release.sh        # auto-increment patch (0.1.0 -> 0.1.1)
./ci/release.sh 0.2.0  # or specify a version
```

The script updates `Cargo.toml`, commits, tags, and pushes.

When a tag is pushed, a github action will:
1. Build binaries for all platforms (macOS, Linux, Windows)
2. Create installer scripts (shell for Unix, PowerShell for Windows)
3. Publish a GitHub Release with all artifacts

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

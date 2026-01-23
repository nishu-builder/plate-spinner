#!/usr/bin/env bash
set -euo pipefail

CURRENT_VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')

if [ $# -eq 0 ]; then
    IFS='.' read -r major minor patch <<< "$CURRENT_VERSION"
    patch=$((patch + 1))
    VERSION="$major.$minor.$patch"
    echo "Auto-incrementing patch version: $CURRENT_VERSION -> $VERSION"
elif [ $# -eq 1 ]; then
    VERSION="$1"
else
    echo "Usage: $0 [version]"
    echo "Example: $0 0.2.0"
    echo "If no version provided, patch version is auto-incremented"
    exit 1
fi

TAG="v$VERSION"

if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
    echo "Error: Version must be semver (e.g., 0.2.0 or 0.2.0-beta.1)"
    exit 1
fi

if [ -n "$(git status --porcelain)" ]; then
    echo "Error: Working directory is not clean. Commit or stash changes first."
    exit 1
fi

COMMIT=$(git rev-parse HEAD)
CI_STATUS=$(gh run list --commit "$COMMIT" --status completed --json conclusion --jq '.[0].conclusion // "none"' 2>/dev/null || echo "error")

if [ "$CI_STATUS" = "error" ]; then
    echo "Error: Could not check CI status. Is 'gh' installed and authenticated?"
    exit 1
elif [ "$CI_STATUS" = "none" ]; then
    echo "Error: No completed CI run found for commit $COMMIT"
    echo "Push and wait for CI to pass before releasing."
    exit 1
elif [ "$CI_STATUS" != "success" ]; then
    echo "Error: CI status for commit $COMMIT is '$CI_STATUS', not 'success'"
    exit 1
fi

if git rev-parse "$TAG" >/dev/null 2>&1; then
    echo "Error: Tag $TAG already exists"
    exit 1
fi

echo "Current version: $CURRENT_VERSION"
echo "New version: $VERSION"
echo ""

read -p "Continue? [y/N] " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 1
fi

sed -i '' "s/^version = \"$CURRENT_VERSION\"/version = \"$VERSION\"/" Cargo.toml

cargo check

git add Cargo.toml
git commit -m "Release $VERSION"
git tag "$TAG"

git push origin main
git push origin "$TAG"

echo ""
echo "Release $VERSION pushed. GitHub Actions will build and publish the release."
echo "https://github.com/nishu-builder/plate-spinner/actions"

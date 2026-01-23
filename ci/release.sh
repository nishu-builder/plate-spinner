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
MAX_WAIT=360  # 6 minutes (2x typical CI time of ~3 min)
POLL_INTERVAL=10

check_ci_status() {
    gh run list --commit "$COMMIT" --status completed --json conclusion --jq '.[0].conclusion // "none"' 2>/dev/null || echo "error"
}

wait_for_ci() {
    local elapsed=0
    echo "Waiting for CI to complete (polling every ${POLL_INTERVAL}s, max ${MAX_WAIT}s)..."
    while [ $elapsed -lt $MAX_WAIT ]; do
        sleep $POLL_INTERVAL
        elapsed=$((elapsed + POLL_INTERVAL))
        CI_STATUS=$(check_ci_status)
        if [ "$CI_STATUS" = "success" ]; then
            echo "CI passed!"
            return 0
        elif [ "$CI_STATUS" != "none" ] && [ "$CI_STATUS" != "error" ]; then
            echo "CI finished with status: $CI_STATUS"
            return 1
        fi
        printf "\r  %ds elapsed..." $elapsed
    done
    echo ""
    echo "Timeout waiting for CI"
    return 1
}

CI_STATUS=$(check_ci_status)

if [ "$CI_STATUS" = "error" ]; then
    echo "Error: Could not check CI status. Is 'gh' installed and authenticated?"
    exit 1
elif [ "$CI_STATUS" = "none" ]; then
    echo "No completed CI run found for commit $COMMIT"
    read -p "Wait for CI to complete? [y/N] " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        if ! wait_for_ci; then
            exit 1
        fi
    else
        echo "Aborted."
        exit 1
    fi
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

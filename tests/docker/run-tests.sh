#!/bin/bash
set -e

cd "$(dirname "$0")/../.."

echo "Building Docker image..."
docker build -t plate-spinner-test -f tests/docker/Dockerfile .

echo ""
echo "=== Running unit tests ==="
docker run --rm plate-spinner-test

echo ""
echo "=== Running E2E tests ==="
docker run --rm plate-spinner-test bash ./tests/docker/test_e2e.sh

echo ""
echo "All Docker tests passed!"

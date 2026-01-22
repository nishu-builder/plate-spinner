FROM python:3.12-slim

RUN apt-get update && apt-get install -y \
    curl \
    jq \
    && rm -rf /var/lib/apt/lists/*

# Install uv
RUN curl -LsSf https://astral.sh/uv/install.sh | sh
ENV PATH="/root/.local/bin:$PATH"

WORKDIR /app

# Copy project files
COPY pyproject.toml uv.lock README.md ./
COPY src/ ./src/
COPY plugin/ ./plugin/

# Install the package
RUN uv sync --no-dev
RUN uv pip install pytest httpx

# Copy tests
COPY tests/ ./tests/

# Run install to set up hooks
RUN uv run sp install

# Copy e2e test script
COPY tests/docker/test_e2e.sh ./tests/docker/
RUN chmod +x ./tests/docker/test_e2e.sh

# Default: run unit tests, can override to run e2e
CMD ["uv", "run", "pytest", "tests/", "-v", "--ignore=tests/docker"]

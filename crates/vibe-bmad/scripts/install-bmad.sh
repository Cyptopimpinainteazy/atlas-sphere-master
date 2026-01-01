#!/bin/sh
set -e

# Simple installer script for bmad-method (alpha by default)
# Usage: ./scripts/install-bmad.sh [stable|alpha]

TAG=${1:-alpha}

run_npx() {
  if command -v node >/dev/null 2>&1; then
    echo "Using host node to run bmad-method"
    if [ "$1" = "stable" ]; then
      npx bmad-method install
    else
      npx bmad-method@alpha install
    fi
    return 0
  fi
  if command -v docker >/dev/null 2>&1; then
    echo "Node not found; using Docker fallback to run bmad-method"
    docker run --rm -v "${PWD}":/workspace -w /workspace node:20 /bin/sh -c "npx bmad-method@${TAG} install"
    return 0
  fi
  echo "Error: Node.js or Docker is required to install BMAD. Please install Node >= 20 or Docker to proceed." >&2
  exit 1
}

echo "Installing bmad-method@${TAG}..."
run_npx ${TAG}

echo "BMAD Method installation completed or started. Run 'npm run status' to check installation status."

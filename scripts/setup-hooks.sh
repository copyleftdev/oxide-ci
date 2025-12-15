#!/usr/bin/env bash
# Setup pre-commit hooks for Oxide CI
set -euo pipefail

echo "Setting up pre-commit hooks..."

# Check if pre-commit is installed
if ! command -v pre-commit &> /dev/null; then
    echo "Installing pre-commit..."
    pip install pre-commit
fi

# Install hooks
pre-commit install

echo "Pre-commit hooks installed successfully!"
echo ""
echo "Hooks will run automatically on git commit."
echo "To run manually: pre-commit run --all-files"

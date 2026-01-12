#!/bin/bash
set -e

# Usage: ./scripts/bump-version.sh <new_version>
# Example: ./scripts/bump-version.sh 0.1.3

if [ -z "$1" ]; then
  echo "Error: Version number required"
  echo "Usage: ./scripts/bump-version.sh <version>"
  echo "Example: ./scripts/bump-version.sh 0.1.3"
  exit 1
fi

NEW_VERSION=$1

echo "🔄 Updating version to $NEW_VERSION..."

# Update Cargo workspace version (single source of truth for Rust)
echo "  📦 Updating Cargo.toml (workspace)"
sed -i.bak "s/^version = \".*\"/version = \"$NEW_VERSION\"/" Cargo.toml && rm Cargo.toml.bak

# Update package.json
echo "  📦 Updating package.json"
sed -i.bak "s/\"version\": \".*\"/\"version\": \"$NEW_VERSION\"/" package.json && rm package.json.bak

# Update tauri.conf.json
echo "  📦 Updating src-tauri/tauri.conf.json"
sed -i.bak "s/\"version\": \".*\"/\"version\": \"$NEW_VERSION\"/" src-tauri/tauri.conf.json && rm src-tauri/tauri.conf.json.bak

echo "✅ Version updated to $NEW_VERSION in all files!"
echo ""
echo "Files updated:"
echo "  - Cargo.toml (workspace)"
echo "  - package.json"
echo "  - src-tauri/tauri.conf.json"
echo ""
echo "Run 'cargo check' to verify Rust crates inherit the new version."

#!/bin/bash

# Script to copy bundle resources to target directory for CLI binaries
# Usage: ./copy-bundles.sh [profile]
# profile defaults to "debug" if not specified

PROFILE=${1:-debug}
TARGET_DIR="target/$PROFILE"

echo "Copying bundles to $TARGET_DIR..."

# Copy fetch-core bundle if it exists
if [ -d "fetch-core/bundle" ]; then
    echo "Copying fetch-core/bundle contents..."
    cp -r fetch-core/bundle/* "$TARGET_DIR/" 2>/dev/null || echo "  (no files in fetch-core/bundle or already exists)"
fi

# Copy fetch-gui bundle if it exists
if [ -d "fetch-gui/bundle" ]; then
    echo "Copying fetch-gui/bundle contents..."
    cp -r fetch-gui/bundle/* "$TARGET_DIR/" 2>/dev/null || echo "  (no files in fetch-gui/bundle or already exists)"
fi

echo "Done! Bundle contents copied to $TARGET_DIR"
echo ""
echo "You can now run the CLI binaries from $TARGET_DIR:"
echo "  $TARGET_DIR/fetch-index --help"
echo "  $TARGET_DIR/fetch-query --help"
echo "  $TARGET_DIR/fetch-daemon --help"

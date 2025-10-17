#!/bin/bash

# Script to copy ONNX runtime libraries and models to target directory
# Usage: ./copy_dylibs.sh <profile> <use_symlinks>
# Arguments:
#   profile: "debug" or "release"
#   use_symlinks: "true" to create symlinks, "false" to copy files

set -e

# Check arguments
if [ $# -ne 2 ]; then
    echo "Usage: $0 <profile> <use_symlinks>"
    echo "  profile: debug or release"
    echo "  use_symlinks: true or false"
    exit 1
fi

PROFILE="$1"
USE_SYMLINKS="$2"

# Validate profile
if [[ "$PROFILE" != "debug" && "$PROFILE" != "release" ]]; then
    echo "Error: Profile must be 'debug' or 'release'"
    exit 1
fi

# Validate symlinks flag
if [[ "$USE_SYMLINKS" != "true" && "$USE_SYMLINKS" != "false" ]]; then
    echo "Error: use_symlinks must be 'true' or 'false'"
    exit 1
fi

# Get script directory and workspace root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(dirname "$SCRIPT_DIR")"

# Determine target directory
TARGET_DIR="$WORKSPACE_ROOT/target/$PROFILE"
echo "Target directory: $TARGET_DIR"

# Create target directory if it doesn't exist
mkdir -p "$TARGET_DIR"

# Function to copy or symlink file/directory
copy_or_link() {
    local src="$1"
    local dst="$2"
    local name="$(basename "$src")"
    
    if [ ! -e "$src" ]; then
        echo "Warning: $name does not exist: $src"
        return
    fi
    
    local dst_path="$dst/$name"
    
    # Remove existing file/symlink/directory
    if [ -e "$dst_path" ] || [ -L "$dst_path" ]; then
        rm -rf "$dst_path"
    fi
    
    if [ "$USE_SYMLINKS" = "true" ]; then
        # Create symlink (use absolute path)
        local abs_src="$(cd "$(dirname "$src")" && pwd)/$(basename "$src")"
        ln -s "$abs_src" "$dst_path"
        echo "Created symlink: $dst_path -> $abs_src"
    else
        # Copy file/directory
        if [ -d "$src" ]; then
            cp -r "$src" "$dst_path"
            echo "Copied directory: $src -> $dst_path"
        else
            cp "$src" "$dst_path"
            echo "Copied file: $src -> $dst_path"
        fi
    fi
}

# Copy models from fetch-core/artifacts
MODELS_SRC="$WORKSPACE_ROOT/fetch-core/bundle/models"
if [ -d "$MODELS_SRC" ]; then
    copy_or_link "$MODELS_SRC" "$TARGET_DIR"
else
    echo "Warning: Models directory not found: $MODELS_SRC"
fi

# Copy ONNX runtime libraries
if [ -n "$ONNX_BUILD_PATH" ]; then
    echo "ONNX build path: $ONNX_BUILD_PATH"
    
    if [ ! -d "$ONNX_BUILD_PATH" ]; then
        echo "Error: ONNX build path does not exist: $ONNX_BUILD_PATH"
        exit 1
    fi
    
    # Define DLLs based on platform
    if [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "win32" ]]; then
        # Windows DLLs
        ONNX_LIBS=(
            "onnxruntime.dll"
            "onnxruntime_providers_shared.dll"
            "onnxruntime_providers_qnn.dll"
            "QnnHtp.dll"
            "QnnSystem.dll"
            "onnxruntime_providers_cuda.dll"
            "cudart64_12.dll"
            "cublasLt64_12.dll"
            "cublas64_12.dll"
        )
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS dylibs
        ONNX_LIBS=(
            "libonnxruntime.dylib"
            "libonnxruntime_providers_shared.dylib"
            "libonnxruntime_providers_qnn.dylib"
            "libQnnHtp.dylib"
            "libQnnSystem.dylib"
            "libonnxruntime_providers_cuda.dylib"
        )
    else
        # Linux .so files
        ONNX_LIBS=(
            "libonnxruntime.so"
            "libonnxruntime_providers_shared.so"
            "libonnxruntime_providers_qnn.so"
            "libQnnHtp.so"
            "libQnnSystem.so"
            "libonnxruntime_providers_cuda.so"
        )
    fi
    
    # Copy each library
    for lib in "${ONNX_LIBS[@]}"; do
        lib_path="$ONNX_BUILD_PATH/$lib"
        if [ -f "$lib_path" ]; then
            copy_or_link "$lib_path" "$TARGET_DIR"
        fi
    done
else
    echo "Warning: ONNX_BUILD_PATH environment variable not set, skipping ONNX library copy"
fi

echo "Done copying libraries and models to $TARGET_DIR"
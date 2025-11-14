# Fetch

A file search application that utilizes locally generated semantic vector embeddings to supplement searches with the actual content of the files themselves.

## Quick Start

To get started with Fetch, download the appropriate installer for your operating system in the latest release on the right and run. The installer will guide you through necessary steps.

If you have previously installed an earlier version of Fetch, you may need to 

### Windows Installation

### Early Alpha Software Warning

## Features

## Semantic Search

### Available Models and File Types

#### Supported Hardware

### Other Search Heuristics

## Settings and Application Data

## CLI

## Building from source

No distributable binaries setup as of yet. Gotta build from source.

1. Install rust, cargo and other rust dependencies: `https://rustup.rs/`
2. Clone the repository: `git clone https://github.com/august99us/fetch.git`
3. `cd fetch`
4. `cargo update` just in case
5. `cargo build` (will probably have build issues to fix)
6. binaries should be ready, either in target/debug or by running `cargo run --bin <binary_name> -- <args>`

<ins>Currently Available Binaries</ins>

All binaries should have `--help` dialogues for more info.

| Binary | Description |
| --- | --- |
| index | Indexes files or entire directories into the default data directory. |
| query | Queries the default data directory for files that fit the query. Supports pagination with --page and --num-results flags. |
| file_daemon | Starts daemon service to track file changes in a directory. |
| drop | Development binary. Drops an entire data directory. |

**Default Settings**

Most of these settings can be found in the default application folder:
| Platform | Folder |
| --- | --- |
| Linux | `/home/<user>/.local/share/fetch` |
| macOS | `/Users/<user>/Library/Application Support/fetch` |
| Windows | `C:\Users\<user>\AppData\Local\fetch` |

After the first run of any binary (except drop) there should be a daemon.toml and data.toml 
file in that folder, which contains more default settings.

The default data directory currently is `<default application folder>/data/default`

## Build Configuration

### ONNX Runtime Setup

Fetch uses ONNX Runtime for neural network inference. The build system will automatically copy the required ONNX Runtime DLLs to the output directory.

**Environment Variables:**

- `ORT_BUILD_PATH`: Path to the ONNX Runtime build directory (e.g., `C:\path\to\onnxruntime\build\Windows\Release\Release`)

**Required DLLs:**

- `onnxruntime.dll` (always required)
- `onnxruntime_providers_shared.dll` (always required)  
- `onnxruntime_providers_qnn.dll` (only with `qnn` feature)
- `QnnHtp.dll` (only with `qnn` feature)

### Features

- `cuda`: Enable CUDA execution provider support
- `qnn`: Enable Qualcomm QNN execution provider support  
- `tracing`: Enable detailed tracing output

### Build Examples

```bash
# Basic build
cargo build

# Build with CUDA support
cargo build --features cuda

# Build with QNN support (requires ORT_BUILD_PATH)
ORT_BUILD_PATH=/path/to/onnxruntime/build/output cargo build --features qnn

# Build with all features
ORT_BUILD_PATH=/path/to/onnxruntime/build/output cargo build --features cuda,qnn,tracing
```

### Model Files

The build system automatically copies model files from `artifacts/models/` to the output directory. Currently supported models:

- SigLIP-2 B/16-512 (image and text embedding models)

Model files are loaded at runtime from relative paths, so the binary must be run from a location where it can access the `models/` directory.
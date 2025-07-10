# Fetch

A human application interface aiming to provide the ability to interact with machine vector interfaces.

## Quick Start

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
| query | Queries the default data directory for files that fit the query. |
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
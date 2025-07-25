use std::{fs, sync::LazyLock};

use camino::{Utf8Path, Utf8PathBuf};
use config::{Config, ConfigError, File};

static APP_FOLDER: LazyLock<Utf8PathBuf> = LazyLock::new(|| Utf8PathBuf::from_path_buf(dirs::data_local_dir()
            .expect("Failed to get local data directory"))
            .expect("Local data directory is not a valid UTF-8 path")
            .join("fetch"));

pub fn get_default_index_directory() -> Utf8PathBuf {
    let data_config = get_data_config().expect("Failed to load data config");

    let folder = Utf8PathBuf::from(data_config.get_string("default_index_directory")
        .expect("Failed to get default table directory from data config")
        .replace("%%AppDataDirectory%%", get_app_folder().as_str()));
    // Create if it doesnt exist
    if !fs::exists(&folder).expect("Error while determining if index directory exists") {
            fs::create_dir_all(&folder).expect("Failed to create default index directory");
    }

    folder
}

pub fn get_default_preview_directory() -> Utf8PathBuf {
    let data_config = get_data_config().expect("Failed to load data config");

    let folder = Utf8PathBuf::from(data_config.get_string("default_preview_directory")
        .expect("Failed to get default preview directory from data config")
        .replace("%%AppDataDirectory%%", get_app_folder().as_str()));
    // create if doesn't exist
    if !fs::exists(&folder).expect("Error while determining if preview directory exists") {
            fs::create_dir_all(&folder).expect("Failed to create default preview directory");
    }

    folder
}

pub fn get_watchlist_file_path() -> Utf8PathBuf {
    let daemon_config = get_daemon_config().expect("Failed to load daemon config");
    println!("{}", daemon_config.get_string("watchlist_file")
        .expect("Failed to get watchlist file from daemon config"));

    Utf8PathBuf::from(daemon_config.get_string("watchlist_file")
        .expect("Failed to get watchlist file from daemon config")
        .replace("%%AppDataDirectory%%", get_app_folder().as_str()))
}

fn get_daemon_config() -> Result<Config, ConfigError> {
    let config_file_path = get_app_folder().join("daemon.toml");
    if !fs::exists(&config_file_path).expect("Error while checking if data config file exists") {
        #[cfg(target_family = "unix")]
        let bytes = include_bytes!("../artifacts/defaults/daemon.toml");
        #[cfg(target_family = "windows")]
        let bytes = include_bytes!("../artifacts/defaults/windows/daemon.toml");
        // If the daemon.toml file does not exist, create it with default values
        fs::write(&config_file_path, bytes).expect("Failed to create default daemon.toml");
    }

    Config::builder()
        .add_source(File::with_name(config_file_path.as_str()))
        .build()
}

fn get_data_config() -> Result<Config, ConfigError> {
    let config_file_path = get_app_folder().join("data.toml");
    if !fs::exists(&config_file_path).expect("Error while checking if data config file exists") {
        #[cfg(target_family = "unix")]
        let bytes = include_bytes!("../artifacts/defaults/data.toml");
        #[cfg(target_family = "windows")]
        let bytes = include_bytes!("../artifacts/defaults/windows/data.toml");
        // If the data.toml file does not exist, create it with default values
        fs::write(&config_file_path, bytes).expect("Failed to create default data.toml");
    }

    Config::builder()
        .add_source(File::with_name(config_file_path.as_str()))
        .build()
}

fn get_app_folder() -> &'static Utf8Path {
    let folder: &'static Utf8PathBuf = &APP_FOLDER;
    if !fs::exists(folder).expect("Error while determining if app data directory exists") {
            fs::create_dir_all(folder).expect("Failed to create local data directory");
    }
    folder.as_path()
}
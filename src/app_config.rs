use camino::Utf8PathBuf;
use config::{Config, ConfigError, File};

pub fn get_daemon_config() -> Result<Config, ConfigError> {
    Config::builder()
        .add_source(File::with_name(get_app_config_folder().join("daemon.toml").as_str()))
        .build()
}

fn get_app_config_folder() -> Utf8PathBuf {
    Utf8PathBuf::from("~/.fetch_config")
}
// NB: Most of this code originated from: https://github.com/gibfahn/up-rs, MIT & Apache 2.0 licensed.

use camino::Utf8PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DefaultsError {
    #[error("Unable to create dir at: {path}.")]
    DirCreation { path: Utf8PathBuf, source: std::io::Error },

    #[error("Unable to copy file. From: {from_path} To: {to_path}")]
    FileCopy {
        from_path: Utf8PathBuf,
        to_path: Utf8PathBuf,
        source: std::io::Error,
    },

    #[error("Failed to read bytes from path {path}")]
    FileRead { path: Utf8PathBuf, source: std::io::Error },

    #[error("Expected to find a plist dictionary, but found a {plist_type} instead.\nDomain: {domain:?}\nKey: {key:?}")]
    NotADictionary { domain: String, key: String, plist_type: &'static str },

    #[error("Failed to read Plist file {path}.")]
    PlistRead { path: Utf8PathBuf, source: plist::Error },

    #[error("Failed to write value to plist file {path}")]
    PlistWrite { path: Utf8PathBuf, source: plist::Error },

    #[error("Failed to write a value to plist file {path} as sudo.")]
    PlistSudoWrite { path: Utf8PathBuf, source: std::io::Error },

    #[error("Invalid YAML at '{path}'")]
    InvalidYaml { path: Utf8PathBuf, source: serde_yaml::Error },

    #[error("Failed to serialize plist to YAML. Domain: {domain:?}")]
    SerializationFailed { domain: String, source: serde_yaml::Error },

    #[error("Failed to deserialize the YAML file or string.")]
    DeserializationFailed { source: serde_yaml::Error },

    #[error("Expected a domain, but didn't find one.")]
    MissingDomain {},

    #[error("Unexpectedly empty option found.")]
    UnexpectedNone,

    #[error("Eyre error.")]
    EyreError { source: color_eyre::Report },
}

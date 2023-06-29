// NB: Most of this code originated from: https://github.com/gibfahn/up-rs, MIT & Apache 2.0 licensed.

use camino::Utf8PathBuf;
use displaydoc::Display;
use thiserror::Error;

#[derive(Error, Debug, Display)]
/// Errors thrown by this file.
pub enum DefaultsError {
    /// Unable to create dir at: {path}.
    DirCreation {
        /// Dir we failed to create.
        path: Utf8PathBuf,
        /// Source error.
        source: std::io::Error,
    },

    /**
    Unable to copy file.

    From: {from_path}
    To: {to_path}
    */
    FileCopy {
        /// Path we tried to copy from.
        from_path: Utf8PathBuf,
        /// Path we tried to copy to.
        to_path: Utf8PathBuf,
        /// Source error.
        source: std::io::Error,
    },

    /// Failed to read bytes from path {path}.
    FileRead {
        /// File we tried to read.
        path: Utf8PathBuf,
        /// Source error.
        source: std::io::Error,
    },

    // /// Failed to write bytes to path {path}.
    // FileWrite {
    //     /// File we tried to read.
    //     path: Utf8PathBuf,
    // },
    /**
    Expected to find a plist dictionary, but found a {plist_type} instead.
    Domain: {domain:?}
    Key: {key:?}
    */
    NotADictionary {
        /// Plist domain.
        domain: String,
        /// Plist key.
        key: String,
        /// Type found instead of a dictionary.
        plist_type: &'static str,
    },

    /// Failed to read Plist file {path}.
    PlistRead {
        /// Path to plist file we failed to read.
        path: Utf8PathBuf,
        /// Source error.
        source: plist::Error,
    },

    /// Failed to write value to plist file {path}
    PlistWrite {
        /// Path to plist file we failed to write.
        path: Utf8PathBuf,
        /// Source error.
        source: plist::Error,
    },

    /// Failed to write a value to plist file {path} as sudo.
    PlistSudoWrite {
        /// Path to plist file we failed to write.
        path: Utf8PathBuf,
        /// Source error.
        source: std::io::Error,
    },

    /// Invalid YAML at '{path}':
    InvalidYaml {
        /// Path that contained invalid YAML.
        path: Utf8PathBuf,
        /// Source error.
        source: serde_yaml::Error,
    },

    /**
    Failed to serialize plist to YAML.
    Domain: {domain:?}
    */
    SerializationFailed {
        /// Plist domain we failed to serialize.
        domain: String,
        /// Source error.
        source: serde_yaml::Error,
    },

    /// Failed to deserialize the YAML file or string.
    DeserializationFailed {
        /// Source error.
        source: serde_yaml::Error,
    },

    /**
    Expected a domain, but didn't find one.
    */
    MissingDomain {},

    /// Unexpectedly empty option found.
    UnexpectedNone,

    /// Eyre error.
    EyreError {
        /// Source error.
        source: color_eyre::Report,
    },
}

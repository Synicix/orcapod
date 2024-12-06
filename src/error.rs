use colored::Colorize;
use glob;
use merkle_hash::error::IndexingError;
use regex;
use serde_yaml::{self, Value};
use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    io,
    path::PathBuf,
    result,
    string::FromUtf8Error,
};
/// Shorthand for a Result that returns an `OrcaError`.
pub type Result<T> = result::Result<T, OrcaError>;

/// Possible errors you may encounter.
#[derive(Debug)]
pub(crate) enum Kind {
    /// Returned if a file is not expected to exist.
    FileExists(PathBuf),
    /// Returned if an annotation was expected to exist.
    NoAnnotationFound(String, String, String),
    /// Returned if a regular expression was expected to match.
    NoRegexMatch,
    /// Wrapper around `glob::GlobError`
    GlobError(glob::GlobError),
    /// Wrapper around `glob::PatternError`
    GlobPatternError(glob::PatternError),
    /// Wrapper around `regex::Error`
    RegexError(regex::Error),
    /// Wrapper around `serde_yaml::Error`
    SerdeYamlError(serde_yaml::Error),
    /// Wrapper around `io::Error`
    IoError(io::Error),
    /// Wrapper around index error thrown by
    IndexingError(IndexingError),
    /// Wrapper around utf8 encoding error
    FromUtf8Error(FromUtf8Error),
    UnsupportedFileStorage(String),
    InvalidURIForFileStore(String, String),
    InvalidStoreName(String),
    NoStorePointersFound,
    MissingPodHashFromPodJobYaml(String),
    FailedToCovertValueToString(Value),
}

/// A stable error API interface.
#[derive(Debug)]
pub struct OrcaError(Kind);
impl Error for OrcaError {}

impl Display for OrcaError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match &self.0 {
            Kind::FileExists(path) => {
                write!(
                    f,
                    "File `{}` already exists.",
                    path.to_string_lossy().bright_cyan()
                )
            }
            Kind::NoAnnotationFound(class, name, version) => {
                write!(f, "No annotation found for `{name}:{version}` {class}.")
            }
            Kind::NoRegexMatch => {
                write!(f, "No match for regex.")
            }
            Kind::GlobError(error) => write!(f, "{error}"),
            Kind::GlobPatternError(error) => write!(f, "{error}"),
            Kind::SerdeYamlError(error) => write!(f, "{error}"),
            Kind::RegexError(error) => write!(f, "{error}"),
            Kind::IoError(error) => write!(f, "{error}"),
            Kind::IndexingError(error) => write!(f, "{error}"),
            Kind::FromUtf8Error(error) => write!(f, "{error}"),
            Kind::UnsupportedFileStorage(file_store_type) => {
                write!(f, "Unsupported file store: {file_store_type}")
            }
            Kind::InvalidURIForFileStore(error, uri) => {
                write!(
                    f,
                    "Fail to initialize file store with uri {uri} with error {error}"
                )
            }
            Kind::InvalidStoreName(store_name) => {
                write!(f, "Invalid store name: {store_name}")
            }
            Kind::NoStorePointersFound => write!(
                f,
                "No Store Pointers available. Perhaps define one first as save it"
            ),

            Kind::MissingPodHashFromPodJobYaml(yaml) => {
                write!(f, "Missing pod_hash from Pod Job yaml: {yaml}")
            }
            Kind::FailedToCovertValueToString(value) => {
                write!(f, "Failed to covert value {value:?} to string")
            }
        }
    }
}
impl From<glob::GlobError> for OrcaError {
    fn from(error: glob::GlobError) -> Self {
        Self(Kind::GlobError(error))
    }
}
impl From<glob::PatternError> for OrcaError {
    fn from(error: glob::PatternError) -> Self {
        Self(Kind::GlobPatternError(error))
    }
}
impl From<serde_yaml::Error> for OrcaError {
    fn from(error: serde_yaml::Error) -> Self {
        Self(Kind::SerdeYamlError(error))
    }
}
impl From<regex::Error> for OrcaError {
    fn from(error: regex::Error) -> Self {
        Self(Kind::RegexError(error))
    }
}
impl From<io::Error> for OrcaError {
    fn from(error: io::Error) -> Self {
        Self(Kind::IoError(error))
    }
}

impl From<FromUtf8Error> for OrcaError {
    fn from(error: FromUtf8Error) -> Self {
        Self(Kind::FromUtf8Error(error))
    }
}

impl From<IndexingError> for OrcaError {
    fn from(error: IndexingError) -> Self {
        Self(Kind::IndexingError(error))
    }
}

impl From<Kind> for OrcaError {
    fn from(error: Kind) -> Self {
        Self(error)
    }
}

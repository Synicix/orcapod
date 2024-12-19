use colored::Colorize;
use glob;
use regex;
use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    io,
    path::PathBuf,
    result,
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
    /// Wrapper around utf8 encoding error
    UnsupportedFileStorage(String),
    InvalidURIForFileStore(String, String),
    MissingPodHashFromPodJobYaml(String),
    FailedToCovertValueToString,
    MultipleHashesForAnnotation(String, String),
    InvalidIndex(usize),
    DeletingAnnotationForStorePointerNotAllowed,
    UnsupportedPath(PathBuf),
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
            Kind::UnsupportedFileStorage(file_store_type) => {
                write!(f, "Unsupported file store: {file_store_type}")
            }
            Kind::InvalidURIForFileStore(error, uri) => {
                write!(
                    f,
                    "Fail to initialize file store with uri {uri} with error {error}"
                )
            }
            Kind::MissingPodHashFromPodJobYaml(yaml) => {
                write!(f, "Missing pod_hash from Pod Job yaml: {yaml}")
            }
            Kind::FailedToCovertValueToString => {
                write!(f, "Failed to covert value to string")
            }
            Kind::MultipleHashesForAnnotation(name, version) => write!(f, "Found mutiple hashes when searching by annotation(name: {name}, version: {version}"),
            Kind::InvalidIndex(idx) => write!(f, "Invalid idx {idx} while trying to access vector"),
            Kind::DeletingAnnotationForStorePointerNotAllowed => write!(f, "Deletion store pointer annotation is not allowed"),
            Kind::UnsupportedPath(path) => write!(f, "Unsupported path type for path {}. Currently only support dir and file", path.to_string_lossy()),
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

impl From<Kind> for OrcaError {
    fn from(error: Kind) -> Self {
        Self(error)
    }
}

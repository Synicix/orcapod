use colored::Colorize;
use glob;
use merkle_hash::error::IndexingError;
use regex;
use serde_yaml;
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
    /// Returned if an annotation delete was attempted on a model's last annotation.
    DeletingLastAnnotation(String, String, String),
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
}

/// A stable error API interface.
#[derive(Debug)]
pub struct OrcaError(Kind);
impl Error for OrcaError {}
impl OrcaError {
    /// Returns `true` if the error was caused by an attempt to delete a model's last annotation.
    pub const fn is_deleting_last_annotation(&self) -> bool {
        matches!(self.0, Kind::DeletingLastAnnotation(_, _, _))
    }
}
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
            Kind::DeletingLastAnnotation(class, name, version) => {
                write!(
                    f,
                    "Attempted to delete the last annotation for `{name}:{version}` {class}."
                )
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

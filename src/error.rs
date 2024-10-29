use anyhow;
use colored::Colorize;
use glob;
use regex;
use serde_yaml;
use std::{
    error::Error,
    fmt,
    fmt::{Display, Formatter},
    io,
    path::PathBuf,
};
/// Shorthand for a Result that returns an `OrcaError`.
pub type Result<T> = anyhow::Result<T, OrcaError>;

/// Possible errors you may encounter.
#[derive(Debug)]
pub(crate) enum Kind {
    /// Returned if a file is not expected to exist.
    FileExists(PathBuf),
    /// Returned if a file is expected to have a parent.
    FileHasNoParent(PathBuf),
    /// Returned if an annotation was expected to exist.
    NoAnnotationFound(String, String, String),
    /// Wrapper around `glob::GlobError`
    GlobError(glob::GlobError),
    /// Wrapper around `glob::PatternError`
    GlobPaternError(glob::PatternError),
    /// Wrapper around `regex::Error`
    RegexError(regex::Error),
    /// Wrapper around `serde_yaml::Error`
    SerdeYamlError(serde_yaml::Error),
    /// Split result error
    SplitResultError(String, String),
    /// Wrapper around `io::Error`
    IoError(io::Error),
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
            Kind::FileHasNoParent(path) => {
                write!(
                    f,
                    "File `{}` has no parent.",
                    path.to_string_lossy().bright_red()
                )
            }
            Kind::NoAnnotationFound(class, name, version) => {
                write!(f, "No annotation found for `{name}:{version}` {class}.")
            }
            Kind::GlobError(error) => write!(f, "{error}"),
            Kind::GlobPaternError(error) => write!(f, "{error}"),
            Kind::SerdeYamlError(error) => write!(f, "{error}"),
            Kind::SplitResultError(string, delimiter) => write!(
                f,
                "{}{}{}{}",
                "Failed to split string: ".bright_red(),
                string.bright_cyan(),
                " with delimiter ".bright_red(),
                delimiter.bright_cyan()
            ),
            Kind::RegexError(error) => write!(f, "{error}"),
            Kind::IoError(error) => write!(f, "{error}"),
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
        Self(Kind::GlobPaternError(error))
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

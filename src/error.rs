use colored::Colorize;
use serde_yaml;
use std::{
    error::Error,
    fmt,
    fmt::{Display, Formatter},
    io,
    path::PathBuf,
};

/// Wrapper around getting None when trying to find struct_name
#[derive(Debug)]
pub struct OutOfBounds {}
impl Error for OutOfBounds {}
impl Display for OutOfBounds {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Index is out of bounds.")
    }
}

/// Wrapper around serde_yaml::from_str
#[derive(Debug)]
pub struct DeserializeError {
    pub path: PathBuf,
    pub error: serde_yaml::Error,
}
impl Error for DeserializeError {}
impl Display for DeserializeError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{}{}{}{}",
            "Failed to deserialize with error ".bright_red(),
            self.error.to_string().bright_red(),
            " for ".bright_red(),
            self.path.to_string_lossy().bright_cyan()
        )
    }
}

/// Wrapper around getting None when trying to find parent
#[derive(Debug)]
pub struct FileHasNoParent {
    pub path: PathBuf,
}
impl Error for FileHasNoParent {}
impl Display for FileHasNoParent {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "File `{}` has no parent.",
            self.path.display().to_string().bright_red()
        )
    }
}

/// Wrapper around serde_yaml::to_string
#[derive(Debug)]
pub struct SerializeError {
    pub item_debug_string: String,
    pub error: serde_yaml::Error,
}
impl Error for SerializeError {}
impl Display for SerializeError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{}{}{}{}",
            "Failed to seralize ".bright_red(),
            self.item_debug_string.bright_cyan(),
            " with error  ".bright_red(),
            self.error.to_string().bright_red(),
        )
    }
}

/// Wrapper around fs::read_to_string and fs::write
#[derive(Debug)]
pub struct IOError {
    pub path: PathBuf,
    pub error: io::Error,
}
impl Error for IOError {}
impl Display for IOError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{}{}{}{}",
            "IO Error: ".bright_red(),
            &self.error.to_string().bright_red(),
            " at ".bright_red(),
            &self.path.to_string_lossy().cyan(),
        )
    }
}

/// Raise error when file exists but unexpected
#[derive(Debug)]
pub struct FileExists {
    pub path: PathBuf,
}
impl Error for FileExists {}
impl Display for FileExists {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "File `{}` already exists.",
            self.path.display().to_string().bright_red()
        )
    }
}

/// Raise error when glob doesn't match on an annotation
#[derive(Debug)]
pub struct NoAnnotationFound {
    pub class: String,
    pub name: String,
    pub version: String,
}
impl Error for NoAnnotationFound {}
impl Display for NoAnnotationFound {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "No annotation found for `{}:{}` {}.",
            self.name, self.version, self.class
        )
    }
}

/// Raise error when regex doesn't match
#[derive(Debug)]
pub struct NoRegexMatch {}
impl Error for NoRegexMatch {}
impl Display for NoRegexMatch {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "No match for regex.")
    }
}

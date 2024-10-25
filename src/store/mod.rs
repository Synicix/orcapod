use crate::{error::Result, model::Pod};
use std::collections::BTreeMap;

/// Standard behavior of any store backend supported.
pub trait Store {
    /// How a pod is stored.
    ///
    /// # Errors
    ///
    /// Will return `Err` if there is an issue storing `pod`.
    fn save_pod(&self, pod: &Pod) -> Result<()>;
    /// How to load a stored pod into a model instance.
    ///
    /// # Errors
    ///
    /// Will return `Err` if there is an issue loading a pod from the store using `name` and
    /// `version`.
    fn load_pod(&self, name: &str, version: &str) -> Result<Pod>;
    /// How to query stored pods.
    ///
    /// # Errors
    ///
    /// Will return `Err` if there is an issue querying metadata from existing pods in the store.
    fn list_pod(&self) -> Result<BTreeMap<String, Vec<String>>>;
    /// How to delete a stored pod (does not propagate).
    ///
    /// # Errors
    ///
    /// Will return `Err` if there is an issue deleting a pod from the store using `name` and
    /// `version`.
    fn delete_pod(&self, name: &str, version: &str) -> Result<()>;
}
/// Store implementation on a local filesystem.
pub mod filestore;

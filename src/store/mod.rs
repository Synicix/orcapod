use crate::{error::Result, model::Pod};

/// Enum for identification to
pub enum ModelID {
    /// Identification via name and version
    NameVer(String, String),
    /// Identification by hash
    Hash(String),
}

/// Struct for list functios
pub struct ModelInfo {
    /// Name from annotation of the model struct
    pub name: String,
    /// Version from annotation from model struct
    pub version: String,
    /// Hash of the model struct
    pub hash: String,
}

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
    fn load_pod(&self, item_key: &ModelID) -> Result<Pod>;
    /// How to query stored pods.
    ///
    /// # Errors
    ///
    /// Will return `Err` if there is an issue querying metadata from existing pods in the store.
    fn list_pod(&self) -> Result<Vec<ModelInfo>>;
    /// How to delete a stored pod (does not propagate).
    ///
    /// # Errors
    ///
    /// Will return `Err` if there is an issue deleting a pod from the store using `name` and
    /// `version`.
    fn delete_pod(&self, item_key: &ModelID) -> Result<()>;

    /// How to delete only annotation, which will leave the item untouched
    ///
    /// # Errors
    /// Will return `Err` if there is an issue of finding the annotation and deleting it
    fn delete_annotation<T>(&self, name: &str, version: &str) -> Result<()>;
}
/// Store implementation on a local filesystem.
pub mod filestore;

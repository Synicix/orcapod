use crate::model::Pod;
use anyhow::Result;
use std::collections::BTreeMap;

pub enum ItemKey {
    NameVer(String, String),
    Hash(String),
}

pub trait Store {
    fn save_pod(&self, pod: &Pod) -> Result<()>;
    fn load_pod(&self, item_key: &ItemKey) -> Result<Pod>;
    fn list_pod(&self) -> Result<BTreeMap<String, String>>;
    fn delete_pod(&self, item_key: &ItemKey) -> Result<()>;
    fn delete_annotation<T>(&self, name: &str, version: &str) -> Result<()>;
}

pub mod filestore;

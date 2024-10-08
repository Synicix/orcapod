use std::error::Error;

use crate::{
    filestore::FileStore,
    model::{Pod, PodNewConfig},
    store::Store,
};

pub enum StorageBackend {
    FileStore(String),
}

pub fn create_pod(
    config: PodNewConfig, // Defaults to None
    storage_backend: &StorageBackend,
) -> Result<Pod, Box<dyn Error>> {
    let pod = Pod::new(config);

    match storage_backend {
        StorageBackend::FileStore(data_storage_path) => {
            let store = FileStore::new(data_storage_path);
            store.store_pod(&pod)?;
        }
    }

    Ok(pod)
}

pub fn load_pod(hash: &str, storage_backend: &StorageBackend) -> Result<Pod, Box<dyn Error>> {
    match storage_backend {
        StorageBackend::FileStore(data_storage_path) => {
            let store = FileStore::new(data_storage_path);
            match store.load_pod(hash) {
                Ok(value) => Ok(value),
                Err(e) => return Err(e),
            }
        }
    }
}

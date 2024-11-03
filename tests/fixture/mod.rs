#![expect(clippy::expect_used, reason = "Expect OK in tests.")]
#![expect(
    clippy::missing_errors_doc,
    reason = "Integration tests won't be included in documentation."
)]

use orcapod::{
    error::Result,
    model::{Annotation, Pod, StreamInfo},
    store::{filestore::LocalFileStore, ModelID, Store},
};
use std::{collections::BTreeMap, fs, ops::Deref, path::PathBuf};
use tempfile::tempdir;

// --- fixtures ---

pub fn pod_style() -> Result<Pod> {
    Pod::new(
        "https://github.com/zenml-io/zenml/tree/0.67.0".to_owned(),
        "zenmldocker/zenml-server:0.67.0".to_owned(),
        "tail -f /dev/null".to_owned(),
        BTreeMap::from([
            (
                "painting".to_owned(),
                StreamInfo {
                    path: PathBuf::from("/input/painting.png"),
                    match_pattern: "/input/painting.png".to_owned(),
                },
            ),
            (
                "image".to_owned(),
                StreamInfo {
                    path: PathBuf::from("/input/image.png"),
                    match_pattern: "/input/image.png".to_owned(),
                },
            ),
        ]),
        PathBuf::from("/output"),
        BTreeMap::from([(
            "styled".to_owned(),
            StreamInfo {
                path: PathBuf::from("./styled.png"),
                match_pattern: "./styled.png".to_owned(),
            },
        )]),
        0.25,                // 250 millicores as frac cores
        (2_u64) * (1 << 30), // 2GiB in bytes
        Some(Annotation {
            name: "style-transfer".to_owned(),
            description: "This is an example pod.".to_owned(),
            version: "0.67.0".to_owned(),
        }),
        None,
    )
}

pub fn store_test(store_directory: Option<&str>) -> Result<TestLocalStore> {
    impl Deref for TestLocalStore {
        type Target = LocalFileStore;
        fn deref(&self) -> &Self::Target {
            &self.store
        }
    }
    impl Drop for TestLocalStore {
        fn drop(&mut self) {
            fs::remove_dir_all(self.store.get_directory()).expect("Failed to teardown store.");
        }
    }
    let tmp_directory = String::from(tempdir()?.path().to_string_lossy());
    let store =
        store_directory.map_or_else(|| LocalFileStore::new(tmp_directory), LocalFileStore::new);
    fs::create_dir_all(store.get_directory())?;
    Ok(TestLocalStore { store })
}

// --- helper functions ---

pub fn add_storage<T: TestSetup>(
    model: T,
    store: &TestLocalStore,
) -> Result<TestLocallyStoredModel<T>> {
    impl<'base, T: TestSetup> Deref for TestLocallyStoredModel<'base, T> {
        type Target = T;
        fn deref(&self) -> &Self::Target {
            &self.model
        }
    }
    impl<'base, T: TestSetup> Drop for TestLocallyStoredModel<'base, T> {
        fn drop(&mut self) {
            self.model
                .delete(self.store)
                .expect("Failed to teardown model.");
        }
    }
    model.save(store)?;
    let model_with_storage = TestLocallyStoredModel { store, model };
    Ok(model_with_storage)
}

// --- util ---

#[derive(Debug)]
pub struct TestLocalStore {
    store: LocalFileStore,
}

#[derive(Debug)]
pub struct TestLocallyStoredModel<'base, T: TestSetup> {
    pub store: &'base TestLocalStore,
    pub model: T,
}

pub trait TestSetup {
    fn save(&self, store: &LocalFileStore) -> Result<()>;
    fn delete(&self, store: &LocalFileStore) -> Result<()>;
}

impl TestSetup for Pod {
    fn save(&self, store: &LocalFileStore) -> Result<()> {
        store.save_pod(self)
    }
    fn delete(&self, store: &LocalFileStore) -> Result<()> {
        store.delete_pod(&ModelID::Hash(self.hash.clone()))
    }
}

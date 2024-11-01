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

#[derive(Debug)]
pub struct TestLocalStore {
    store: LocalFileStore,
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

#[derive(Debug)]
pub struct TestLocallyStoredPod<'base> {
    pub store: &'base TestLocalStore,
    pub pod: Pod,
}

pub fn add_pod_storage(pod: Pod, store: &TestLocalStore) -> Result<TestLocallyStoredPod> {
    impl<'base> Deref for TestLocallyStoredPod<'base> {
        type Target = Pod;
        fn deref(&self) -> &Self::Target {
            &self.pod
        }
    }
    impl<'base> Drop for TestLocallyStoredPod<'base> {
        fn drop(&mut self) {
            self.store
                .delete_pod(&ModelID::Hash(self.pod.hash.clone()))
                .expect("Failed to teardown pod.");
        }
    }
    let pod_with_storage = TestLocallyStoredPod { store, pod };
    pod_with_storage.store.save_pod(&pod_with_storage)?;
    Ok(pod_with_storage)
}

use orcapod::error::Result;
use orcapod::model::{PodJob, PodJobRetryPolicy};
use orcapod::store::ModelInfo;
use orcapod::{
    model::{to_yaml, Annotation, Pod, StreamInfo},
    store::{filestore::LocalFileStore, ModelID, Store},
};
use std::{
    collections::BTreeMap,
    fs,
    ops::{Deref, DerefMut},
    path::PathBuf,
};
use tempfile::tempdir;

#[derive(PartialEq, Clone)]
pub enum Model {
    Pod(Pod),
    PodJob(PodJob),
}

pub enum ModelType {
    Pod,
    PodJob,
}

impl Model {
    #[expect(clippy::unwrap_used, reason = "test")]
    ///
    /// # Panics
    /// Panic if annotation is none
    pub fn get_name(&self) -> &str {
        match self {
            Self::Pod(pod) => &pod.annotation.as_ref().unwrap().name,
            Self::PodJob(pod_job) => &pod_job.annotation.as_ref().unwrap().name,
        }
    }

    pub fn get_hash(&self) -> &str {
        match self {
            Self::Pod(pod) => &pod.hash,
            Self::PodJob(pod_job) => &pod_job.hash,
        }
    }

    #[expect(clippy::unwrap_used, reason = "test")]
    /// # Panics
    /// Panic if annotation is none
    pub fn get_version(&self) -> &str {
        match self {
            Self::Pod(pod) => &pod.annotation.as_ref().unwrap().version,
            Self::PodJob(pod_job) => &pod_job.annotation.as_ref().unwrap().version,
        }
    }

    pub const fn is_annotation_none(&self) -> bool {
        match self {
            Self::Pod(pod) => pod.annotation.is_none(),
            Self::PodJob(pod_job) => pod_job.annotation.is_none(),
        }
    }

    /// # Errors
    /// Errors if ``to_yaml`` fails
    pub fn to_yaml(&self) -> Result<String> {
        match self {
            Self::Pod(pod) => to_yaml(pod),
            Self::PodJob(pod_job) => to_yaml(pod_job),
        }
    }

    #[expect(clippy::unwrap_used, reason = "test")]
    ///
    /// # Panics
    /// panics if annotation is none
    pub fn set_name(&mut self, name: &str) {
        match self {
            Self::Pod(pod) => name.clone_into(&mut pod.annotation.as_mut().unwrap().name),
            Self::PodJob(pod_job) => {
                name.clone_into(&mut pod_job.annotation.as_mut().unwrap().name);
            }
        }
    }
}

///
/// # Errors
/// Forward get fuction errors
pub fn get_test_item(item_type: &ModelType) -> Result<Model> {
    match item_type {
        ModelType::Pod => Ok(Model::Pod(get_test_pod()?)),
        ModelType::PodJob => Ok(Model::PodJob(get_test_pod_job()?)),
    }
}

/// # Errors
/// Forwards ``pod::new()`` errors
pub fn get_test_pod() -> Result<Pod> {
    Pod::new(
        Some(Annotation {
            name: "style-transfer".to_owned(),
            description: "This is an example pod.".to_owned(),
            version: "0.67.0".to_owned(),
        }),
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
        None,
    )
}

/// # Errors
/// Forward ``pod_job::new()`` errors
pub fn get_test_pod_job() -> Result<PodJob> {
    let mut input_volume_map = BTreeMap::new();
    input_volume_map.insert(PathBuf::from("style"), PathBuf::from("style.png"));
    input_volume_map.insert(PathBuf::from("image"), PathBuf::from("image.png"));

    let mut output_volume_map = BTreeMap::new();
    output_volume_map.insert(PathBuf::from("stylized_image"), PathBuf::from("image.png"));

    PodJob::new(
        Some(Annotation {
            name: "Test Pod Job".to_owned(),
            version: "0.0.0".to_owned(),
            description: "example_description".to_owned(),
        }),
        get_test_pod()?,
        input_volume_map,
        output_volume_map,
        2.0_f32,
        (2_u64) * (1 << 30),
        PodJobRetryPolicy::NoRetry,
    )
}

#[derive(Debug)]
pub struct TestLocalStore {
    store: LocalFileStore,
}

impl Deref for TestLocalStore {
    type Target = LocalFileStore;
    fn deref(&self) -> &Self::Target {
        &self.store
    }
}

impl DerefMut for TestLocalStore {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.store
    }
}

#[expect(clippy::expect_used, reason = "test")]
impl Drop for TestLocalStore {
    fn drop(&mut self) {
        fs::remove_dir_all(self.store.get_directory()).expect("Failed to teardown store.");
    }
}

#[expect(clippy::unwrap_used, reason = "test")]
impl TestLocalStore {
    /// # Panics
    /// Will panic if fail to fetch stuff related to annotation
    pub fn make_annotation_path(&self, item: &Model) -> PathBuf {
        match item {
            Model::Pod(pod) => self.store.make_annotation_path::<Pod>(
                &pod.hash,
                &pod.annotation.as_ref().unwrap().name,
                &pod.annotation.as_ref().unwrap().version,
            ),
            Model::PodJob(pod_job) => self.store.make_annotation_path::<PodJob>(
                &pod_job.hash,
                &pod_job.annotation.as_ref().unwrap().name,
                &pod_job.annotation.as_ref().unwrap().version,
            ),
        }
    }

    pub fn make_path(&self, model_type: &ModelType, hash: &str, file_name: &str) -> PathBuf {
        match model_type {
            ModelType::Pod => self.store.make_path::<Pod>(hash, file_name),
            ModelType::PodJob => self.store.make_path::<PodJob>(hash, file_name),
        }
    }

    /// # Errors
    /// Forward save model errors
    pub fn save_model(&self, model: &Model) -> Result<()> {
        match model {
            Model::Pod(pod) => self.store.save_pod(pod),
            Model::PodJob(pod_job) => self.store.save_pod_job(pod_job),
        }
    }

    /// # Errors
    /// Forward load model errors
    pub fn load_model(&self, model_type: &ModelType, item_key: &ModelID) -> Result<Model> {
        match model_type {
            ModelType::Pod => Ok(Model::Pod(self.store.load_pod(item_key)?)),
            ModelType::PodJob => Ok(Model::PodJob(self.store.load_pod_job(item_key)?)),
        }
    }

    /// # Errors
    /// Forward list model errors
    pub fn list_model(&self, model_type: &ModelType) -> Result<Vec<ModelInfo>> {
        match model_type {
            ModelType::Pod => self.store.list_pod(),
            ModelType::PodJob => self.store.list_pod_job(),
        }
    }

    /// # Errors
    /// Forward delete model errors
    pub fn delete_model(&self, model_type: &ModelType, item_key: &ModelID) -> Result<()> {
        match model_type {
            ModelType::Pod => self.store.delete_pod(item_key),
            ModelType::PodJob => self.store.delete_pod_job(item_key),
        }
    }

    /// # Errors
    /// Forward delete item annotation errors
    pub fn delete_model_annotation(
        &mut self,
        item_type: &ModelType,
        name: &str,
        version: &str,
    ) -> Result<()> {
        match item_type {
            ModelType::Pod => Ok(self.store.delete_annotation::<Pod>(name, version)?),
            ModelType::PodJob => Ok(self.store.delete_annotation::<PodJob>(name, version)?),
        }
    }
}

/// # Errors
/// Will error if creating new store fails
pub fn store_test(store_directory: Option<&str>) -> Result<TestLocalStore> {
    let tmp_directory = tempdir()?.path().to_owned();
    let store =
        store_directory.map_or_else(|| LocalFileStore::new(&tmp_directory), LocalFileStore::new);
    Ok(TestLocalStore { store })
}

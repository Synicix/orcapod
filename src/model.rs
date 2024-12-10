use crate::{
    error::{Kind, OrcaError, Result},
    store::{localstore::LocalStore, FileStore},
    util::{get_type_name, hash},
};
use serde::{ser::SerializeStruct, Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    result,
};

///
/// # Errors
/// Error out with ``serde_yaml`` seralization error if something goes wrong
pub fn to_yaml<T: Serialize>(item: &T) -> Result<String> {
    let mut yaml = serde_yaml::to_string(item)?;
    yaml.insert_str(0, &format!("class: {}\n", get_type_name::<T>())); // replace class at top
    Ok(yaml)
}

/// Model object that contains a ``BTreeMap``that maps store names to the actual URI use to reconstruct the stores
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct StorePointer {
    /// Version tag to uniquely identify
    #[serde(skip)]
    pub annotation: Annotation,
    #[serde(skip)]
    /// hash identity, for now it is just the uri
    pub hash: String,
    /// Uri path to the store
    pub uri: String,
}

impl StorePointer {
    /// Function to create new store pointer and compute the hash
    ///
    /// # Errors
    /// Return serialization error if something went wrong.
    pub fn new(annotation: Annotation, uri: String) -> Result<Self> {
        let mut store_pointer = Self {
            annotation,
            uri,
            hash: String::new(),
        };

        store_pointer.hash = hash(&to_yaml(&store_pointer)?);
        Ok(store_pointer)
    }

    /// Function to rebuild the store based on self
    ///
    /// # Errors
    /// Will fail if rebuilding of the store access struct fails
    pub fn get_store(&self) -> Result<impl FileStore> {
        // Load the yaml into a Btreemap, pull out the class, then build the store

        let storage_class_name = self.uri.split("::").collect::<Vec<&str>>()[0];

        match storage_class_name {
            "LocalStore" => Ok(LocalStore::from_uri(&self.uri)?),
            _ => Err(OrcaError::from(Kind::UnsupportedFileStorage(
                storage_class_name.to_owned(),
            ))),
        }
    }
}

/// A reusable, containerized computational unit.
#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct Pod {
    /// Metadata that doesn't affect reproducibility.
    #[serde(skip)]
    pub annotation: Option<Annotation>,
    /// Unique id based on reproducibility.
    #[serde(skip)]
    pub hash: String,
    command: String,
    image: String,
    input_stream_map: BTreeMap<String, StreamInfo>,
    output_dir: PathBuf,
    output_stream_map: BTreeMap<String, StreamInfo>,
    recommended_cpus: f32,
    recommended_memory: u64,
    required_gpu: Option<GPURequirement>,
    source_commit_url: String,
}

impl Pod {
    /// Construct a new pod instance.
    ///
    /// # Errors
    ///
    /// Will return `Err` if there is an issue initializing a `Pod` instance.
    pub fn new(
        annotation: Option<Annotation>,
        source_commit_url: String,
        image: String,
        command: String,
        // Defined as Key and (Pathbuf, and Regex restriction)
        input_stream_map: BTreeMap<String, StreamInfo>,
        output_dir: PathBuf,
        output_stream_map: BTreeMap<String, StreamInfo>,
        recommended_cpus: f32,
        recommended_memory: u64,
        required_gpu: Option<GPURequirement>,
    ) -> Result<Self> {
        let pod_no_hash = Self {
            annotation,
            hash: String::new(),
            source_commit_url,
            image,
            command,
            input_stream_map,
            output_dir,
            output_stream_map,
            recommended_cpus,
            recommended_memory,
            required_gpu,
        };
        Ok(Self {
            hash: hash(to_yaml(&pod_no_hash)?),
            ..pod_no_hash
        })
    }
}

/// Struct to store the path of where to look in the storage along with the checksum for hashing
/// and/or file integrity check
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct InputStoreMapping {
    path: PathBuf,
    store_name: Option<String>,
    content_check_sum: String,
}

impl InputStoreMapping {
    /// Construct a new ``InputStoreMapping`` with empty content check sum
    pub fn new(path: impl AsRef<Path>, store_name: Option<String>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            store_name,
            content_check_sum: String::new(),
        }
    }
}

/// Describe the input with two current options
/// Singular file,
/// or a collection of files that will be map
///
/// NOTE: All files are to be uploaded to the apporiate store ahead of usage
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum Input {
    /// Single File to be used as input that is stored in the store
    File(InputStoreMapping),
    /// Collection of files to be used as input (assume to be same type to match against regex)
    FileCollection(Vec<InputStoreMapping>),
    /// For folder mounting
    Folder(InputStoreMapping),
}

/// Mapping for output volume mount from container -> File Store
#[derive(Serialize, Deserialize, PartialEq, Eq, Default, Debug)]
pub struct OutputStoreMapping {
    /// Where to store the results and in which store
    pub path: PathBuf,
    /// The name of the store
    pub store_name: Option<String>,
}
/// Struct to represent ``PodJob``
#[derive(Deserialize, PartialEq, Default, Debug)]
pub struct PodJob {
    /// Optional annotation for pod job
    #[serde(skip)]
    pub annotation: Option<Annotation>,
    /// Computed by coverting it to yaml then hash
    #[serde(skip)]
    pub pod: Pod,
    /// Hash of the yaml seraliziation of pod job
    #[serde(skip)]
    pub hash: String,
    /// String is the key, variable, input is the actual path to look up
    input_store_mapping: BTreeMap<String, Input>,
    output_store_mapping: OutputStoreMapping,
    cpu_limit: f32, // Num of cpu to limit the pod from
    mem_limit: u64, // Bytes to limit memory
    retry_policy: RetryPolicy,
}

impl PodJob {
    /// Function to create a new pod job
    ///
    /// # Errors
    /// Will error out if fail to cover to yaml and hash
    pub fn new(
        annotation: Option<Annotation>,
        pod: Pod,
        input_store_mapping: BTreeMap<String, Input>,
        output_store_mapping: OutputStoreMapping,
        cpu_limit: f32,
        mem_limit: u64,
        retry_policy: RetryPolicy,
    ) -> Result<Self> {
        let pod_job_no_hash = Self {
            annotation,
            hash: String::new(),
            pod,
            input_store_mapping,
            output_store_mapping,
            cpu_limit,
            mem_limit,
            retry_policy,
        };

        Ok(Self {
            hash: hash(&to_yaml(&pod_job_no_hash)?),
            ..pod_job_no_hash
        })
    }
}

impl Serialize for PodJob {
    fn serialize<S>(&self, serializer: S) -> result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("PodJob", 6)?;
        state.serialize_field("cpu_limit", &self.cpu_limit)?;
        state.serialize_field("input_store_mapping", &self.input_store_mapping)?;
        state.serialize_field("mem_limit", &self.mem_limit)?;
        state.serialize_field("output_store_mapping", &self.output_store_mapping)?;
        state.serialize_field("pod_hash", &self.pod.hash)?;
        state.serialize_field("retry_policy", &self.retry_policy)?;

        state.end()
    }
}

// --- util types ---

/// Standard metadata structure for all model instances.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Default)]
pub struct Annotation {
    /// A unique name.
    pub name: String,
    /// A unique semantic version.
    pub version: String,
    /// A long form description.
    pub description: String,
}

/// Specification for GPU requirements in computation.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct GPURequirement {
    /// GPU model specification.
    pub model: GPUModel,
    /// Manufacturer recommended memory.
    pub recommended_memory: u64,
    /// Number of GPU cards required.
    pub count: u16,
}

/// GPU model specification.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum GPUModel {
    /// NVIDIA-manufactured card where `String` is the specific model e.g. ???
    NVIDIA(String),
    /// AMD-manufactured card where `String` is the specific model e.g. ???
    AMD(String),
}
/// Streams are named and represent an abstraction for the file(s) that represent some particular
/// data.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct StreamInfo {
    /// Path to stream file.
    pub path: PathBuf,
    /// Regex restriction of input file
    /// For file, it can be i.e  \N+.png
    /// For folders it must end in a slash i.e. \N+\/
    pub match_pattern: String,
}

/// Pod job retry policy
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum RetryPolicy {
    /// Will stop the job upon first failure
    NoRetry,
    /// Will allow n number of failures within a time window of t seconds
    RetryTimeWindow(u16, u64), // Where u16 is num of retries and u64 is time in seconds
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::NoRetry
    }
}

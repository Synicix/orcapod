use crate::{
    error::Result,
    store::Store,
    util::{get_type_name, hash},
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_yaml::{Mapping, Value};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

/// Converts a model instance into a consistent yaml.
///
/// # Errors
///
/// Will return `Err` if there is an issue converting an `instance` into YAML (w/o annotation).
pub fn to_yaml<T: Serialize>(instance: &T) -> Result<String> {
    let mapping: BTreeMap<String, Value> = serde_yaml::from_str(&serde_yaml::to_string(instance)?)?; // sort
    let mut yaml = serde_yaml::to_string(
        &mapping
            .into_iter()
            .filter(|(k, _)| k != "annotation" && k != "hash")
            .collect::<BTreeMap<_, _>>(),
    )?; // skip fields
    yaml.insert_str(0, &format!("class: {}\n", get_type_name::<T>())); // replace class at top

    Ok(yaml)
}
/// Instantiates a model from from yaml content and its unique hash.
///
/// # Errors
///
/// Will return `Err` if there is an issue converting YAML files for spec+annotation into a model
/// instance.
pub fn from_yaml<T: DeserializeOwned>(
    hash: &str,
    spec_yaml: &str,
    annotation_yaml: Option<&str>,
) -> Result<T> {
    let mut spec: BTreeMap<String, Value> = serde_yaml::from_str(spec_yaml)?;
    spec.insert("hash".to_owned(), Value::from(hash));
    if let Some(resolved_annotation_yaml) = annotation_yaml {
        let annotation: Mapping = serde_yaml::from_str(resolved_annotation_yaml)?;
        spec.insert("annotation".to_owned(), Value::from(annotation));
    }

    Ok(serde_yaml::from_str(&serde_yaml::to_string(&spec)?)?)
}

// --- core model structs ---

/// Struct to store the path of where to look in the storage along with the checksum for hashing
/// and/or file integrity check
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct InputData {
    path: PathBuf,
    content_check_sum: String,
}

impl InputData {
    /// Upon creation, verify that the file is valid in the store and compute the checksum
    ///
    /// # Errors
    /// Error out with ``std::io`` if something goes wrong
    pub fn new<T: Store>(path: impl AsRef<Path>, store: &T) -> Result<Self> {
        Ok(Self {
            path: path.as_ref().to_path_buf(),
            content_check_sum: store.compute_checksum_for_file_or_dir(path)?,
        })
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
    File(InputData),
    /// Collection of files to be used as input (assume to be same type to match against regex)
    FileCollection(Vec<InputData>),
    /// For folder mounting
    Folder(InputData),
}

/// A reusable, containerized computational unit.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Pod {
    /// Metadata that doesn't affect reproducibility.
    pub annotation: Option<Annotation>,
    /// Unique id based on reproducibility.
    pub hash: String,
    source_commit_url: String,
    image: String,
    command: String,
    input_stream_map: BTreeMap<String, StreamInfo>,
    output_dir: PathBuf,
    output_stream_map: BTreeMap<String, StreamInfo>,
    recommended_cpus: f32,
    recommended_memory: u64,
    required_gpu: Option<GPURequirement>,
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
            hash: hash(&to_yaml(&pod_no_hash)?),
            ..pod_no_hash
        })
    }
}

/// Struct to represent ``PodJob``
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct PodJob {
    /// Optional annotation for pod job
    pub annotation: Option<Annotation>,
    /// Computed by coverting it to yaml then hash
    pub hash: String,
    /// Details about the pod from which the pod job was created from
    pub pod_hash: String,
    /// String is the key, variable, input is the actual path to look up
    input_volume_map: BTreeMap<String, Input>,
    output_volume_map: BTreeMap<String, PathBuf>,
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
        pod_hash: String,
        input_volume_map: BTreeMap<String, Input>,
        output_volume_map: BTreeMap<String, PathBuf>,
        cpu_limit: f32,
        mem_limit: u64,
        retry_policy: RetryPolicy,
    ) -> Result<Self> {
        let pod_job_no_hash = Self {
            annotation,
            hash: String::new(),
            pod_hash,
            input_volume_map,
            output_volume_map,
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

// --- util types ---

/// Standard metadata structure for all model instances.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
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
    /// Naming pattern for the stream.
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

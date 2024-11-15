use crate::{
    error::{Kind, OrcaError, Result},
    model::{from_yaml, to_yaml, Annotation, Pod, PodJob},
    store::{ModelID, ModelInfo, Store},
    util::get_type_name,
};
use colored::Colorize;
use merkle_hash::{Algorithm, Encodable, MerkleTree};
use regex::Regex;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

static FILE_STORE_FOLDER_NAME: &str = "file_store";

/// Support for a storage backend on a local filesystem directory.
#[derive(Debug)]
pub struct LocalFileStore {
    /// A local path to a directory where store will be located.
    directory: PathBuf,
}

impl Store for LocalFileStore {
    fn save_pod(&self, pod: &Pod) -> Result<()> {
        self.save_model(pod, &pod.hash, &pod.annotation)
    }

    fn load_pod(&self, model_id: &ModelID) -> Result<Pod> {
        self.load_model(model_id)
    }

    fn list_pod(&self) -> Result<Vec<ModelInfo>> {
        self.list_model::<Pod>()
    }

    fn delete_pod(&self, model_id: &ModelID) -> Result<()> {
        self.delete_model::<Pod>(model_id)
    }

    fn delete_annotation<T>(&self, name: &str, version: &str) -> Result<()> {
        let hash = self.lookup_hash::<T>(name, version)?;

        let annotation_file =
            self.make_hash_rel_path::<T>(&hash, &Self::make_annotation_relpath(name, version));
        fs::remove_file(&annotation_file)?;

        Ok(())
    }

    fn save_pod_job(&self, pod_job: &PodJob) -> Result<()> {
        self.save_model(pod_job, &pod_job.hash, &pod_job.annotation)
    }

    fn load_pod_job(&self, model_id: &ModelID) -> Result<PodJob> {
        self.load_model(model_id)
    }

    fn list_pod_job(&self) -> Result<Vec<ModelInfo>> {
        self.list_model::<PodJob>()
    }

    fn delete_pod_job(&self, model_id: &ModelID) -> Result<()> {
        self.delete_model::<PodJob>(model_id)
    }

    fn compute_checksum_for_file_or_dir(&self, path: impl AsRef<Path>) -> Result<String> {
        Ok(MerkleTree::builder(
            self.directory
                .join(FILE_STORE_FOLDER_NAME)
                .join(path)
                .to_string_lossy(),
        )
        .algorithm(Algorithm::Blake3)
        .hash_names(true)
        .build()?
        .root
        .item
        .hash
        .to_hex_string())
    }

    fn load_file(&self, path: impl AsRef<Path>) -> Result<Vec<u8>> {
        Ok(fs::read(
            self.directory.join(FILE_STORE_FOLDER_NAME).join(path),
        )?)
    }

    fn save_file(&self, path: impl AsRef<Path>, content: Vec<u8>) -> Result<()> {
        Self::save_file_internal(
            self.directory.join(FILE_STORE_FOLDER_NAME).join(path),
            content,
            true,
        )
    }

    fn wipe(&self) -> Result<()> {
        Ok(fs::remove_dir_all(&self.directory)?)
    }
}

impl LocalFileStore {
    /// Construct a local file store instance in a specific directory.
    pub fn new(directory: impl AsRef<Path>) -> Self {
        Self {
            directory: directory.as_ref().into(),
        }
    }

    /// Relative path where model specification is stored within the model directory.
    pub const SPEC_RELPATH: &str = "spec.yaml";
    /// Relative path where model annotation is stored within the model directory.
    pub fn make_annotation_relpath(name: &str, version: &str) -> PathBuf {
        PathBuf::from(format!("annotation/{name}-{version}.yaml"))
    }

    /// Makes the path to model type
    pub fn make_model_path<T>(&self) -> PathBuf {
        PathBuf::from(format!(
            "{}/{}",
            self.directory.to_string_lossy(),
            get_type_name::<T>()
        ))
    }

    /// Makes path to the hash object
    pub fn make_hash_path<T>(&self, hash: impl AsRef<Path>) -> PathBuf {
        self.make_model_path::<T>().join(hash)
    }

    /// Build the storage path with the model directory (`hash`) and a file's relative path.
    pub fn make_hash_rel_path<T>(&self, hash: &str, relpath: impl AsRef<Path>) -> PathBuf {
        self.make_hash_path::<T>(hash).join(relpath)
    }

    fn find_annotation(glob_pattern: &Path) -> Result<impl Iterator<Item = Result<ModelInfo>>> {
        let re = Regex::new(
            r"(?x)
            ^.*
                \/(?<hash>[0-9a-f]+)
                    \/annotation
                        \/
                        (?<name>[0-9a-zA-Z\-]+)
                        -
                        (?<version>[0-9]+\.[0-9]+\.[0-9]+)
                        \.yaml
            $",
        )?;
        let paths = glob::glob(&glob_pattern.to_string_lossy())?.map(move |filepath| {
            let filepath_string = String::from(filepath?.to_string_lossy());
            let group = re
                .captures(&filepath_string)
                .ok_or_else(|| OrcaError::from(Kind::NoRegexMatch))?;

            Ok(ModelInfo {
                name: group["name"].to_string(),
                version: group["version"].to_string(),
                hash: group["hash"].to_string(),
            })
        });
        Ok(paths)
    }

    fn lookup_hash<T>(&self, name: &str, version: &str) -> Result<String> {
        let model_info = Self::find_annotation(
            &self.make_hash_rel_path::<T>("*", &Self::make_annotation_relpath(name, version)),
        )?
        .next()
        .ok_or_else(|| {
            OrcaError::from(Kind::NoAnnotationFound(
                get_type_name::<T>(),
                name.to_owned(),
                version.to_owned(),
            ))
        })??;
        Ok(model_info.hash)
    }

    fn save_file_internal(
        file: impl AsRef<Path>,
        content: impl AsRef<[u8]>,
        fail_if_exists: bool,
    ) -> Result<()> {
        if let Some(parent) = file.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }
        let file_exists = file.as_ref().exists();
        if file_exists && fail_if_exists {
            return Err(OrcaError::from(Kind::FileExists(
                file.as_ref().to_path_buf(),
            )));
        } else if file_exists {
            println!(
                "Skip saving `{}` since it is already stored.",
                file.as_ref().to_string_lossy().bright_cyan(),
            );
        } else {
            fs::write(file, content)?;
        }
        Ok(())
    }

    fn save_model<T: Serialize>(
        &self,
        model: &T,
        hash: &str,
        annotation: &Option<Annotation>,
    ) -> Result<()> {
        // Save the annotation file and throw and error if exist if exist
        if let Some(annotation_unwrap) = annotation {
            Self::save_file_internal(
                self.make_hash_rel_path::<T>(
                    hash,
                    &Self::make_annotation_relpath(
                        &annotation_unwrap.name,
                        &annotation_unwrap.version,
                    ),
                ),
                &serde_yaml::to_string(&annotation_unwrap)?,
                true,
            )?;
        }

        // Save the pod and skip if it already exist, for the case of many annotation to a single pod
        Self::save_file_internal(
            self.make_hash_rel_path::<T>(hash, Self::SPEC_RELPATH),
            &to_yaml(model)?,
            false,
        )?;

        Ok(())
    }

    fn load_model<T: DeserializeOwned>(&self, model_id: &ModelID) -> Result<T> {
        match model_id {
            ModelID::Hash(hash) => from_yaml(
                hash,
                &fs::read_to_string(self.make_hash_rel_path::<T>(hash, Self::SPEC_RELPATH))?,
                None,
            ),
            ModelID::Annotation(name, version) => {
                let hash = self.lookup_hash::<T>(name, version)?;
                from_yaml(
                    &hash,
                    &fs::read_to_string(self.make_hash_rel_path::<T>(&hash, Self::SPEC_RELPATH))?,
                    Some(&fs::read_to_string(self.make_hash_rel_path::<T>(
                        &hash,
                        &Self::make_annotation_relpath(name, version),
                    ))?),
                )
            }
        }
    }

    fn list_model<T>(&self) -> Result<Vec<ModelInfo>> {
        // Get all hashes first and store them in the btreemap
        let mut unique_hashes = HashSet::new();

        let hash_paths = glob::glob(&self.make_model_path::<T>().join("*").to_string_lossy())?;
        let re = Regex::new(r"^.*\/(?<hash>[0-9a-f]+)$")?;

        for path in hash_paths {
            let path_string = String::from(path?.to_string_lossy());
            let captures = re
                .captures(&path_string)
                .ok_or_else(|| OrcaError::from(Kind::NoRegexMatch))?;

            unique_hashes.insert(captures["hash"].to_owned());
        }

        // Get the vector of model info
        let mut model_infos_with_annotation = Self::find_annotation(
            &self.make_hash_rel_path::<T>("*", &Self::make_annotation_relpath("*", "*")),
        )?
        .collect::<Result<Vec<ModelInfo>>>()?;

        // Go through all the model info and remove hash from unique hashes
        for model_info in &model_infos_with_annotation {
            unique_hashes.remove(&model_info.hash);
        }

        // Insert whatever remains with empty fields for name and version
        for annotationless_hash in &unique_hashes {
            model_infos_with_annotation.push(ModelInfo {
                name: String::new(),
                version: String::new(),
                hash: annotationless_hash.to_owned(),
            });
        }

        Ok(model_infos_with_annotation)
    }

    fn delete_model<T>(&self, model_id: &ModelID) -> Result<()> {
        // assumes propagate = false
        let hash = match model_id {
            ModelID::Hash(hash) => hash,
            ModelID::Annotation(name, version) => &self.lookup_hash::<T>(name, version)?,
        };
        let spec_dir = self.make_hash_rel_path::<T>(hash, "");
        fs::remove_dir_all(spec_dir)?;

        Ok(())
    }
}

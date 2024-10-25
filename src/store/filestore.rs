use crate::{
    error::{FileExists, FileHasNoParent, NoAnnotationFound},
    model::{from_yaml, to_yaml, Annotation, Pod},
    util::get_type_name,
};
use anyhow::Result;
use colored::Colorize;
use regex::Regex;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use super::{ItemKey, Store};

#[derive(Debug)]
pub struct LocalFileStore {
    directory: PathBuf,
}

impl Store for LocalFileStore {
    fn save_pod(&self, pod: &Pod) -> Result<()> {
        self.save_item(pod, &pod.hash, pod.annotation.as_ref())
    }

    fn load_pod(&self, item_key: &ItemKey) -> Result<Pod> {
        self.load_item::<Pod>(item_key)
    }

    /// Return the name version index btree where key is name-version and value is hash of pod
    fn list_pod(&self) -> Result<BTreeMap<String, String>> {
        self.build_name_ver_tree::<Pod>()
    }

    fn delete_pod(&self, item_key: &ItemKey) -> Result<()> {
        self.delete_item::<Pod>(item_key)
    }

    fn delete_annotation<T>(&self, name: &str, version: &str) -> Result<()> {
        // Search the name ver index for the hash
        let hash = self.get_hash_from_name_ver_tree::<T>(name, version)?;

        fs::remove_file(self.make_annotation_path::<T>(&hash, name, version))?;
        Ok(())
    }

    /// Pod job will check will confirm that the pod is valid first before saving the pod_job
    fn save_pod_job(&self, pod_job: &crate::model::PodJob) -> Result<(), Box<dyn Error>> {
        // Check if pod exists if not save it
        if !self.make_spec_path::<Pod>(&pod_job.pod.hash).exists() {
            self.save_pod(&pod_job.pod)?;
        }

        self.save_item(pod_job, &pod_job.annotation, &pod_job.hash)
    }

    fn load_pod_job(
        &self,
        name: &str,
        version: &str,
    ) -> Result<crate::model::PodJob, Box<dyn Error>> {
        self.load_item::<PodJob>(name, version)
    }

    fn list_pod_job(&self) -> Result<Vec<ItemInfo>, Box<dyn Error>> {
        self.list_item::<PodJob>()
    }

    fn delete_pod_job(&self, name: &str, version: &str) -> Result<(), Box<dyn Error>> {
        self.delete_item::<PodJob>(name, version)
    }
}

impl LocalFileStore {
    pub fn new(directory: impl AsRef<Path>) -> Self {
        Self {
            directory: directory.as_ref().into(),
        }
    }

    pub fn get_directory(&self) -> &Path {
        &self.directory
    }

    fn make_dir_path<T>(&self, hash: &str) -> PathBuf {
        PathBuf::from(format!(
            "{}/{}/{}",
            self.directory.to_string_lossy(),
            get_type_name::<T>(),
            hash,
        ))
    }

    pub fn make_path<T>(&self, hash: &str, file_name: &str) -> PathBuf {
        self.make_dir_path::<T>(hash).join(file_name)
    }

    pub fn make_annotation_path<T>(&self, hash: &str, name: &str, version: &str) -> PathBuf {
        self.make_dir_path::<T>(hash)
            .join("annotations")
            .join(format!("{name}-{version}.yaml"))
    }

    // Generic function for save load list delete
    /// Generic func to save all sorts of item
    ///
    /// Example usage inside `LocalFileStore`
    /// ``` markdown
    /// let pod = Pod::new(); // For example doesn't actually work
    /// self.save_item(pod, &pod.annotation, &pod.hash).unwrap()
    /// ```
    fn save_item<T: Serialize>(
        &self,
        item: &T,
        hash: &str,
        annotation: Option<&Annotation>,
    ) -> Result<()> {
        // Save the item first
        Self::save_file(
            &self.make_path::<T>(hash, "spec.yaml"),
            &to_yaml::<T>(item)?,
            false,
        )?;

        // Save the annotation file and throw and error if exist
        if let Some(value) = annotation {
            // Annotation exist, thus save it
            Self::save_file(
                &self.make_annotation_path::<T>(hash, &value.name, &value.version),
                &serde_yaml::to_string(value)?,
                true,
            )?;
        }

        Ok(())
    }

    /// Generic function for loading spec.yaml into memory
    fn load_item<T: DeserializeOwned>(&self, item_key: &ItemKey) -> Result<T> {
        match item_key {
            ItemKey::NameVer(name, version) => {
                // Search the name-ver index
                let hash = self.get_hash_from_name_ver_tree::<T>(name, version)?;

                // Get the spec and annotation yaml
                let spec_yaml = fs::read_to_string(self.make_path::<T>(&hash, "spec.yaml"))?;

                let annotation_yaml =
                    fs::read_to_string(self.make_annotation_path::<T>(&hash, name, version))?;

                from_yaml::<T>(&spec_yaml, &hash, Some(&annotation_yaml))
            }
            ItemKey::Hash(hash) => {
                // Get the spec and annotation yaml
                let spec_yaml = fs::read_to_string(self.make_path::<T>(hash, "spec.yaml"))?;
                from_yaml::<T>(&spec_yaml, hash, None)
            }
        }
    }

    fn delete_item<T>(&self, item_key: &ItemKey) -> Result<()> {
        let hash = match item_key {
            ItemKey::NameVer(name, version) => {
                // Search the name-ver index
                self.get_hash_from_name_ver_tree::<T>(name, version)?
            }
            ItemKey::Hash(hash) => hash.to_owned(),
        };

        fs::remove_dir_all(self.make_dir_path::<T>(&hash))?;
        Ok(())
    }

    fn build_name_ver_tree<T>(&self) -> Result<BTreeMap<String, String>> {
        // Construct the cache with glob and regex
        let type_name = get_type_name::<T>();
        let re = Regex::new(&format!(
            r"^.*\/{type_name}\/(?<hash>[a-z0-9]+)\/annotations\/(?<name>[A-z0-9\- ]+)-(?<ver>[0-9]+.[0-9]+.[0-9]+).yaml$"
        ))?;

        // Create the missing cache btree for the item
        let mut name_ver_tree = BTreeMap::new();

        let search_pattern = self.make_dir_path::<T>("*").join("annotations/*");

        for path in glob::glob(&search_pattern.to_string_lossy())? {
            let path_str: String = path?.to_string_lossy().to_string();

            let Some(cap) = re.captures(&path_str) else {
                continue;
            };

            // Insert into the cache
            name_ver_tree.insert(
                format!("{}-{}", &cap["name"].to_string(), &cap["ver"].to_string()),
                cap["hash"].into(),
            );
        }
        Ok(name_ver_tree)
    }

    fn get_hash_from_name_ver_tree<T>(&self, name: &str, version: &str) -> Result<String> {
        Ok(self
            .build_name_ver_tree::<T>()?
            .get(&Self::make_name_ver_tree_key(name, version))
            .ok_or_else(|| NoAnnotationFound {
                class: get_type_name::<T>(),
                name: name.into(),
                version: version.into(),
            })?
            .to_owned())
    }

    fn make_name_ver_tree_key(name: &str, version: &str) -> String {
        format!("{name}-{version}")
    }

    // Help save file function
    fn save_file(path: &PathBuf, content: impl AsRef<[u8]>, fail_if_exists: bool) -> Result<()> {
        fs::create_dir_all(
            path.parent()
                .ok_or_else(|| FileHasNoParent { path: path.clone() })?,
        )?;
        let file_exists = fs::exists(path)?;
        if file_exists {
            if fail_if_exists {
                return Err(FileExists { path: path.clone() }.into());
            }

            println!(
                "Skip saving `{}` since it is already stored.",
                path.to_string_lossy().bright_cyan(),
            );
            return Ok(());
        }

        fs::write(path, content.as_ref())?;
        Ok(())
    }
}

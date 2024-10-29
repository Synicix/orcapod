use crate::{
    error::{Kind, OrcaError, Result},
    model::{from_yaml, to_yaml, Annotation, Pod},
    util::get_type_name,
};
use colored::Colorize;
use regex::Regex;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use super::{ItemKey, Store};

/// Local storage system for orca items implmenting store
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

    /// Return Btree where key is column name and value is a vec of values
    /// Are we okay with returning a bunch of strings? For now it works but later on like adding version
    /// this will break...
    #[expect(
        clippy::unwrap_in_result,
        reason = "It shouldn't failed as we added the keys ourselves at the start"
    )]
    #[expect(
        clippy::unwrap_used,
        reason = "It shouldn't failed as we added the keys ourselves at the start"
    )]
    fn list_pod(&self) -> Result<BTreeMap<String, Vec<String>>> {
        let mut table: BTreeMap<String, Vec<String>> = BTreeMap::new();
        table.insert("name".to_owned(), Vec::new());
        table.insert("version".to_owned(), Vec::new());
        table.insert("hash".to_owned(), Vec::new());
        for (key, hash) in self.build_name_ver_tree::<Pod>()? {
            let split_result = key.rsplit_once('-').ok_or_else(|| {
                OrcaError::from(Kind::SplitResultError(key.clone(), '-'.to_string()))
            })?;
            table
                .get_mut("name")
                .unwrap()
                .push(split_result.0.to_owned());
            table
                .get_mut("version")
                .unwrap()
                .push(split_result.1.to_owned());
            table.get_mut("hash").unwrap().push(hash.clone());
        }

        Ok(table)
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
}

impl LocalFileStore {
    /// New function that takes the directory as where to save the files
    pub fn new(directory: impl AsRef<Path>) -> Self {
        Self {
            directory: directory.as_ref().into(),
        }
    }

    /// Getter function for directory
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

    /// Helper function for making path to a given item type T
    pub fn make_path<T>(&self, hash: &str, file_name: &str) -> PathBuf {
        self.make_dir_path::<T>(hash).join(file_name)
    }

    /// Helper function to create the path to the annotations files
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
            self.make_path::<T>(hash, "spec.yaml"),
            &to_yaml::<T>(item)?,
            false,
        )?;

        // Save the annotation file and throw and error if exist
        if let Some(value) = annotation {
            // Annotation exist, thus save it
            Self::save_file(
                self.make_annotation_path::<T>(hash, &value.name, &value.version),
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

        // Create tree where name_ver is key and value is hash
        let mut name_ver_tree = BTreeMap::new();

        let search_pattern = self.make_dir_path::<T>("*").join("annotations/*");

        for path in glob::glob(&search_pattern.to_string_lossy())? {
            let path_str: String = path?.to_string_lossy().to_string();

            let Some(cap) = re.captures(&path_str) else {
                continue;
            };

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
            .ok_or_else(|| {
                OrcaError::from(Kind::NoAnnotationFound(
                    get_type_name::<T>(),
                    name.into(),
                    version.into(),
                ))
            })?
            .to_owned())
    }

    fn make_name_ver_tree_key(name: &str, version: &str) -> String {
        format!("{name}-{version}")
    }

    // Help save file function
    fn save_file(
        path: impl AsRef<Path>,
        content: impl AsRef<[u8]>,
        fail_if_exists: bool,
    ) -> Result<()> {
        fs::create_dir_all(
            path.as_ref().parent().ok_or_else(|| {
                OrcaError::from(Kind::FileHasNoParent(path.as_ref().to_path_buf()))
            })?,
        )?;
        if path.as_ref().exists() {
            if fail_if_exists {
                return Err(OrcaError::from(Kind::FileExists(
                    path.as_ref().to_path_buf(),
                )));
            }

            println!(
                "Skip saving `{}` since it is already stored.",
                path.as_ref().to_string_lossy().bright_cyan(),
            );
            return Ok(());
        }

        fs::write(path.as_ref(), content.as_ref())?;
        Ok(())
    }
}

#![expect(clippy::expect_used, reason = "Expect OK in tests.")]
#![expect(clippy::panic_in_result_fn, reason = "Panics OK in tests.")]

pub mod fixture;
use fixture::{add_storage, pod_style, store_test, TestSetup};
use orcapod::{
    error::Result,
    model::{Annotation, Pod},
    store::{filestore::LocalFileStore, ModelID, ModelInfo, Store},
};
use std::{fmt::Debug, fs, path::Path};
use tempfile::tempdir;

fn is_dir_empty(file: &Path, levels_up: usize) -> Option<bool> {
    Some(
        file.ancestors()
            .nth(levels_up)?
            .read_dir()
            .ok()?
            .next()
            .is_none(),
    )
}

fn basic_test<T: TestSetup + Debug>(model: T) -> Result<()>
where
    T::Target: PartialEq<T> + Debug,
{
    let store = store_test(None)?;
    let stored_model = add_storage(model, &store)?;
    let annotation = stored_model
        .model
        .get_annotation()
        .expect("Annotation missing from `pod_style`");
    assert_eq!(
        store.list_pod()?,
        vec![ModelInfo {
            name: annotation.name.clone(),
            version: annotation.version.clone(),
            hash: stored_model.model.get_hash().to_owned()
        }],
        "List didn't match."
    );
    let loaded_model = stored_model.model.load(&store)?;
    assert_eq!(
        loaded_model, stored_model.model,
        "Loaded model doesn't match."
    );
    Ok(())
}

#[test]
fn test_pod_with_file_store() -> Result<()> {
    test_item_store_with_annotation(&ModelType::Pod)
}

#[test]
fn pod_files() -> Result<()> {
    let store_directory = String::from(tempdir()?.path().to_string_lossy());
    {
        let item = get_test_item(item_type)?;
        let mut store = store_test(Some(&tempdir()?.path().to_string_lossy()))?; // new tests can just call store_test(None)?

        let annotation_file = store.make_annotation_path(&item);

        let spec_file = store.make_hash_rel_path(item_type, item.get_hash(), "spec.yaml");

        // Test save
        store.save_model(&item)?;
        assert!(
            spec_file.exists(),
            "Spec file could not be found after item creation"
        );
        assert_eq!(
            fs::read_to_string(&spec_file)?,
            item.to_yaml()?,
            "spec.yaml does not match item.to_yaml, something went wrong during the write"
        );
        assert!(
            annotation_file.exists(),
            "Cannot find annotation file after saving"
        );

        // Test load
        let loaded_item = store.load_model(
            item_type,
            &ModelID::NameVer(item.get_name().into(), item.get_version().into()),
        )?;

        assert!(
            loaded_item == item,
            "Loaded item does not match the item was saved"
        );

        let loaded_item_by_hash =
            store.load_model(item_type, &ModelID::Hash(item.get_hash().into()))?;

        assert!(
            loaded_item_by_hash.is_annotation_none(),
            "Annotation should be empty"
        );

        // Test list pod
        // Should only return a result of 1
        let items = store.list_model(item_type)?;
        assert!(items.len() == 1, "List item should be length of 1");
        assert!(
            items[0].name == item.get_name(),
            "Item name from list_model didn't match what was saved"
        );
        assert!(
            items[0].version == item.get_version(),
            "Item version from list_model didn't match what was saved"
        );
        assert!(
            items[0].hash == item.get_hash(),
            "Item hash from list_model didn't match what was saved"
        );

        // Add another pod with a new version
        let mut item_2 = item.clone();
        item_2.set_name("Second Item Test");

        store.save_model(&item_2)?;

        assert!(
            store.list_model(item_type)?.len() == 2,
            "List item should be length of 2"
        );

        // Test delete
        store.delete_item_annotation(item_type, item_2.get_name(), item_2.get_version())?;

        assert!(
            store.list_model(item_type)?.len() == 1,
            "List item should be length of 1"
        );

        // Delete the first pod
        store.delete_item(
            item_type,
            &ModelID::NameVer(item.get_name().into(), item.get_version().into()),
        )?;

        assert!(
            store.list_model(item_type)?.is_empty(),
            "List item should be empty"
        );

        // Test the case with where delete wipes out all annotation
        store.save_model(&item)?;
        store.save_model(&item_2)?;

        assert!(
            store.list_model(item_type)?.len() == 2,
            "List item should be length of 2"
        );

        // Delete the entire pod which should get rid of annotation
        store.delete_item(
            item_type,
            &ModelID::NameVer(item.get_name().into(), item.get_version().into()),
        )?;

        assert!(store.list_model(item_type)?.is_empty(), "List item should be empty after deleting the object itself regardless of how many annotations there are");

        // Test the hash version
        // Test the case with where delete wipes out all annotation
        store.save_model(&item)?;
        store.save_model(&item_2)?;

        assert!(
            store.list_model(item_type)?.len() == 2,
            "List item should be length of 2"
        );

        // Delete the entire pod which should get rid of annotation
        store.delete_item(item_type, &ModelID::Hash(item.get_hash().into()))?;

        assert!(store.list_model(item_type)?.is_empty(), "List item should be empty after deleting the object itself regardless of how many annotations there are");

        assert!(
            !spec_file.exists(),
            "Spec was found when it should have been deleted"
        );
        assert!(
            !annotation_file.exists(),
            "Annotation file wasn't cleaned up."
        );
        assert_eq!(
            is_dir_empty(&spec_file, 2),
            Some(true),
            "Model directory wasn't cleaned up."
        );
    };
    assert!(
        !fs::exists(&store_directory)?,
        "Store directory wasn't cleaned up."
    );
    Ok(())
}

#[test]
fn pod_list_empty() -> Result<()> {
    let store = store_test(None)?;
    assert_eq!(store.list_pod()?, vec![], "Pod list is not empty.");
    Ok(())
}

#[test]
fn pod_load_from_hash() -> Result<()> {
    let store = store_test(None)?;
    let mut stored_model = add_storage(pod_style()?, &store)?;
    stored_model.model.annotation = None;
    let loaded_pod = stored_model
        .store
        .load_pod(&ModelID::Hash(stored_model.model.hash.clone()))?;
    assert_eq!(
        loaded_pod, stored_model.model,
        "Loaded model from hash doesn't match."
    );
    Ok(())
}

#[test]
fn pod_annotation_delete() -> Result<()> {
    let store = store_test(None)?;
    let mut stored_model = add_storage(pod_style()?, &store)?;
    stored_model.model.annotation = Some(Annotation {
        name: "new-name".to_owned(),
        version: "0.5.0".to_owned(),
        description: String::new(),
    });
    store.save_pod(&stored_model.model)?;
    assert_eq!(
        store.list_pod()?,
        vec![
            ModelInfo {
                name: "new-name".to_owned(),
                version: "0.5.0".to_owned(),
                hash: "13d69656d396c272588dd875b2802faee1a56bd985e3c43c7db276a373bc9ddb".to_owned()
            },
            ModelInfo {
                name: "style-transfer".to_owned(),
                version: "0.67.0".to_owned(),
                hash: "13d69656d396c272588dd875b2802faee1a56bd985e3c43c7db276a373bc9ddb".to_owned()
            }
        ],
        "Pod list didn't return 2 expected entries."
    );
    store.delete_annotation::<Pod>("new-name", "0.5.0")?;
    assert_eq!(
        store.list_pod()?,
        vec![ModelInfo {
            name: "style-transfer".to_owned(),
            version: "0.67.0".to_owned(),
            hash: "13d69656d396c272588dd875b2802faee1a56bd985e3c43c7db276a373bc9ddb".to_owned()
        }],
        "Pod list didn't return 1 expected entry."
    );
    assert!(
        store
            .delete_annotation::<Pod>("style-transfer", "0.67.0")
            .expect_err("Unexpectedly succeeded.")
            .is_deleting_last_annotation(),
        "Returned a different OrcaError than one expected for deleting the last annotation."
    );
    Ok(())
}

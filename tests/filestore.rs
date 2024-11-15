#![expect(
    clippy::panic_in_result_fn,
    reason = "Asserts usage panics inside of function"
)]

pub mod fixture;
use std::path::Path;

use anyhow::{Ok, Result};
use fixture::{Model, ModelType, StoreScaffold};
use orcapod::store::{filestore::LocalFileStore, ModelID};
use tempfile::tempdir;

// Store clean up test
#[test]
fn test_store_wipe() -> Result<()> {
    let temp_dir = tempdir()?.into_path();
    {
        let (_, _) = scaffold_store_with_model(&ModelType::Pod, &temp_dir)?;
    }

    assert!(
        !temp_dir.exists(),
        "Store failed to clean up local file store"
    );
    Ok(())
}

// Pod Store Test
#[test]
fn test_list_pod() -> Result<()> {
    test_list_model(&ModelType::Pod)
}

#[test]
fn test_load_pod() -> Result<()> {
    test_load_model(&ModelType::Pod)
}

#[test]
fn test_delete_pod_by_hash() -> Result<()> {
    test_delete_model_by_hash(&ModelType::Pod)
}

#[test]
fn test_delete_pod_by_annotation() -> Result<()> {
    test_delete_model_by_annotation(&ModelType::Pod)
}

#[test]
fn test_delete_pod_annotation() -> Result<()> {
    test_delete_annotation(&ModelType::Pod)
}

// Pod Job Store Tests
#[test]
fn test_list_pod_job() -> Result<()> {
    test_list_model(&ModelType::PodJob)
}

#[test]
fn test_load_pod_job() -> Result<()> {
    test_load_model(&ModelType::PodJob)
}

#[test]
fn test_delete_pod_job_by_hash() -> Result<()> {
    test_delete_model_by_hash(&ModelType::PodJob)
}

#[test]
fn test_delete_pod_job_by_annotation() -> Result<()> {
    test_delete_model_by_annotation(&ModelType::PodJob)
}

#[test]
fn test_delete_pod_job_annotation() -> Result<()> {
    test_delete_annotation(&ModelType::PodJob)
}

fn scaffold_store_with_model(
    model_type: &ModelType,
    store_dir: impl AsRef<Path>,
) -> Result<(Model, StoreScaffold<LocalFileStore>)> {
    // Upon going out of scope, the scaffold should wipe the directory it used for local file store
    let store = StoreScaffold {
        store: LocalFileStore::new(store_dir),
    };

    // Test saving
    let model = model_type.get_model(&store)?;
    store.save_model(&model)?;
    Ok((model, store))
}

fn test_list_model(model_type: &ModelType) -> Result<()> {
    let temp_dir = tempdir()?.into_path();
    let (model, store) = scaffold_store_with_model(model_type, temp_dir)?;

    let list_result = store.list_model(model_type)?;
    assert!(
        list_result.len() == 1,
        "List result return more than 1 value"
    );
    assert!(
        list_result[0].hash == model.get_hash(),
        "Model hash didn't match what was put in"
    );
    assert!(
        list_result[0].name == model.get_annotation().name,
        "Model name didn't match what was put in"
    );
    assert!(
        list_result[0].version == model.get_annotation().version,
        "Model version didn't match what was put in"
    );

    Ok(())
}

fn test_load_model(model_type: &ModelType) -> Result<()> {
    let temp_dir = tempdir()?.into_path();
    let (mut model, store) = scaffold_store_with_model(model_type, temp_dir)?;

    // By name and version
    assert!(
        model
            == store.load_model(
                &ModelID::Annotation(
                    model.get_annotation().name.clone(),
                    model.get_annotation().version.clone()
                ),
                model_type
            )?,
        "model loaded from store didn't match what was put in"
    );

    // Test Load by hash
    // Set annotation to None
    model.set_annotation(None);
    assert!(
        model == store.load_model(&ModelID::Hash(model.get_hash().to_owned()), model_type)?,
        "model loaded from store didn't match what was put in"
    );

    Ok(())
}

fn test_delete_annotation(model_type: &ModelType) -> Result<()> {
    let temp_dir = tempdir()?.into_path();
    let (model, store) = scaffold_store_with_model(model_type, temp_dir)?;

    store.delete_item_annotation(
        &model.get_annotation().name,
        &model.get_annotation().version,
        model_type,
    )?;

    let list_result = store.list_model(model_type)?;
    assert!(
        list_result.len() == 1,
        "List result return more than 1 value"
    );
    assert!(
        list_result[0].hash == model.get_hash(),
        "Model hash didn't match what was put in"
    );
    assert!(
        list_result[0].name == String::new(),
        "Model name didn't match what was put in"
    );
    assert!(
        list_result[0].version == String::new(),
        "Model version didn't match what was put in"
    );

    Ok(())
}

fn test_delete_model_by_hash(model_type: &ModelType) -> Result<()> {
    let temp_dir = tempdir()?.into_path();
    let (model, store) = scaffold_store_with_model(model_type, temp_dir)?;

    store.delete_item(&ModelID::Hash(model.get_hash().to_owned()), model_type)?;
    assert!(
        store.list_model(model_type)?.is_empty(),
        "List is not empty after requesting delete"
    );
    Ok(())
}

fn test_delete_model_by_annotation(model_type: &ModelType) -> Result<()> {
    let temp_dir = tempdir()?.into_path();
    let (model, store) = scaffold_store_with_model(model_type, temp_dir)?;

    store.delete_item(
        &ModelID::Annotation(
            model.get_annotation().name.clone(),
            model.get_annotation().version.clone(),
        ),
        model_type,
    )?;
    assert!(
        store.list_model(model_type)?.is_empty(),
        "List is not empty after requesting delete"
    );
    Ok(())
}

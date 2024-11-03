#![expect(clippy::panic_in_result_fn, reason = "Panics OK in tests.")]
#![expect(clippy::expect_used, reason = "Expect OK in tests.")]

pub mod fixture;
use fixture::{add_storage, pod_style, store_test};
use orcapod::{
    error::Result,
    model::{to_yaml, Pod},
    store::{filestore::LocalFileStore, ModelID, Store},
};
use std::{collections::BTreeMap, fs, path::Path};
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

#[test]
fn verify_pod_save_and_delete() -> Result<()> {
    let store_directory = String::from(tempdir()?.path().to_string_lossy());
    {
        let pod_style = pod_style()?;
        let store = store_test(Some(&store_directory))?; // new tests can just call store_test(None)?
        let annotation = pod_style
            .annotation
            .as_ref()
            .expect("Annotation missing from `pod_style`");
        let annotation_file = store.make_path::<Pod>(
            &pod_style.hash,
            &LocalFileStore::make_annotation_relpath(&annotation.name, &annotation.version),
        );
        let spec_file = store.make_path::<Pod>(&pod_style.hash, LocalFileStore::SPEC_RELPATH);
        {
            let pod = add_storage(pod_style, &store)?;
            assert!(spec_file.exists());
            assert_eq!(fs::read_to_string(&spec_file)?, to_yaml::<Pod>(&pod)?);
            assert!(annotation_file.exists());
        };
        assert!(!spec_file.exists());
        assert!(!annotation_file.exists());
        assert_eq!(is_dir_empty(&spec_file, 2), Some(true));
        assert_eq!(is_dir_empty(&annotation_file, 3), Some(true));
    };
    assert!(!fs::exists(&store_directory)?);
    Ok(())
}

#[test]
fn verify_pod_load() -> Result<()> {
    let store = store_test(None)?;
    let stored_pod = add_storage(pod_style()?, &store)?;
    let annotation = stored_pod
        .annotation
        .as_ref()
        .expect("Annotation missing from `pod_style`");
    let loaded_pod = store.load_pod(&ModelID::Annotation(
        annotation.name.clone(),
        annotation.version.clone(),
    ))?;
    assert_eq!(loaded_pod, stored_pod.model);
    Ok(())
}

#[test]
fn verify_pod_list() -> Result<()> {
    let store = store_test(None)?;
    assert_eq!(
        store.list_pod()?,
        BTreeMap::from([
            ("hash".to_owned(), vec![],),
            ("name".to_owned(), vec![],),
            ("version".to_owned(), vec![],),
        ])
    );
    Ok(())
}

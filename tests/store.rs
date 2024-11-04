#![expect(clippy::expect_used, reason = "Expect OK in tests.")]
#![expect(clippy::panic_in_result_fn, reason = "Panics OK in tests.")]

pub mod fixture;
use fixture::{add_storage, pod_style, store_test, TestSetup};
use orcapod::{
    error::Result,
    model::Pod,
    store::{filestore::LocalFileStore, Store},
};
use std::{collections::BTreeMap, fmt::Debug, fs, path::Path};
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
        .get_annotation()
        .expect("Annotation missing from `pod_style`");
    assert_eq!(
        store.list_pod()?,
        BTreeMap::from([
            ("hash".to_owned(), vec![stored_model.get_hash().to_owned()],),
            ("name".to_owned(), vec![annotation.name.clone()],),
            ("version".to_owned(), vec![annotation.version.clone()],),
        ]),
        "List didn't match."
    );
    let loaded_model = stored_model.load(&store)?;
    assert_eq!(loaded_model, stored_model.model, "Models don't match");
    Ok(())
}

#[test]
fn pod_basic() -> Result<()> {
    basic_test(pod_style()?)
}

#[test]
fn pod_files() -> Result<()> {
    let store_directory = String::from(tempdir()?.path().to_string_lossy());
    {
        let pod_style = pod_style()?;
        let store = store_test(Some(&store_directory))?;
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
            let _pod = add_storage(pod_style, &store)?;
            assert!(spec_file.exists());
            assert!(annotation_file.exists());
        };
        assert!(!spec_file.exists());
        assert!(!annotation_file.exists());
        assert_eq!(is_dir_empty(&spec_file, 2), Some(true));
    };
    assert!(!fs::exists(&store_directory)?);
    Ok(())
}

#[test]
fn pod_list_empty() -> Result<()> {
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

#![expect(clippy::panic_in_result_fn, reason = "Panics OK in tests.")]

pub mod fixture;
use anyhow::Result;
use fixture::{pod_fixture, pod_job_fixture, store_pointer_fixture};
use indoc::indoc;
use orcapod::{
    model::{to_yaml, Pod, PodJob, StorePointer},
    store::{localstore::LocalStore, FileStore},
};
use tempfile::tempdir;

#[test]
fn hash() -> Result<()> {
    assert_eq!(
        pod_fixture()?.hash,
        "5c6d2467f5f1cbfc6b321208ae9628be6a61255a810a08ddad60a7abb8953e53",
        "Hash didn't match."
    );
    Ok(())
}

#[test]
fn pod_to_yaml() -> Result<()> {
    assert_eq!(
        to_yaml::<Pod>(&pod_fixture()?)?,
        indoc! {"
            class: pod
            command: tail -f /dev/null
            image: zenmldocker/zenml-server:0.67.0
            input_stream_map:
              image:
                path: /input/image.png
                match_pattern: /input/image.png
              painting:
                path: /input/painting.png
                match_pattern: /input/painting.png
            output_dir: /output
            output_stream_map:
              styled:
                path: styled.png
                match_pattern: styled.png
            recommended_cpus: 0.25
            recommended_memory: 2147483648
            required_gpu: null
            source_commit_url: https://github.com/zenml-io/zenml/tree/0.67.0
        "},
        "YAML serialization didn't match."
    );
    Ok(())
}

#[test]
fn pod_job_to_yaml() -> Result<()> {
    let temp_dir = tempdir()?.into_path();
    assert_eq!(
        // Use LocalFileStore as store example
        to_yaml::<PodJob>(&pod_job_fixture(&LocalStore::new(temp_dir))?)?,
        indoc! {"
        class: pod_job
        cpu_limit: 2.0
        input_store_mapping:
          image: !File
            path: image.png
            store_name: null
            content_check_sum: ''
          style: !File
            path: style.png
            store_name: null
            content_check_sum: ''
        mem_limit: 4294967296
        output_store_mapping:
          path: stylized_image
          store_name: null
        pod_hash: 5c6d2467f5f1cbfc6b321208ae9628be6a61255a810a08ddad60a7abb8953e53
        retry_policy: NoRetry
    "},
        "YAML serialization didn't match."
    );
    Ok(())
}

#[test]
fn store_pointer_to_yaml() -> Result<()> {
    let temp_dir = tempdir()?.into_path();

    let expected_yaml = format!(
        "class: store_pointer\nuri: LocalStore::{}\n",
        temp_dir.to_string_lossy()
    );

    assert_eq!(
        // Use LocalFileStore as store example
        to_yaml::<StorePointer>(&store_pointer_fixture(&LocalStore::new(temp_dir))?)?,
        expected_yaml,
        "YAML serialization didn't match."
    );
    Ok(())
}

#[test]
fn store_pointer_get_store() -> Result<()> {
    let temp_dir = tempdir()?.into_path();
    let store = LocalStore::new(temp_dir);

    // Create the fixture
    let store_pointer = store_pointer_fixture(&store)?;
    let loaded_store = store_pointer.get_store()?;
    assert!(loaded_store.get_uri() == store.get_uri());
    Ok(())
}

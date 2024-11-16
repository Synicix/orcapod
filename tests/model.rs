#![expect(clippy::panic_in_result_fn, reason = "Panics OK in tests.")]

pub mod fixture;
use anyhow::Result;
use fixture::{pod_job_style, pod_style};
use indoc::indoc;
use orcapod::{
    model::{to_yaml, Pod, PodJob},
    store::filestore::LocalFileStore,
};
use tempfile::tempdir;

#[test]
fn hash() -> Result<()> {
    assert_eq!(
        pod_style()?.hash,
        "13d69656d396c272588dd875b2802faee1a56bd985e3c43c7db276a373bc9ddb",
        "Hash didn't match."
    );
    Ok(())
}

#[test]
fn pod_to_yaml() -> Result<()> {
    assert_eq!(
        to_yaml::<Pod>(&pod_style()?)?,
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
                path: ./styled.png
                match_pattern: ./styled.png
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
        to_yaml::<PodJob>(&pod_job_style(&LocalFileStore::new(temp_dir))?)?,
        indoc! {"
        class: pod_job
        cpu_limit: 2.0
        input_volume_map:
          image: !File
            path: image.png
            content_check_sum: 314491a82a5e07fb8dfc36dfb33209ed9edfcfdadd3e7a4ee5623eff90b6d7ad
          style: !File
            path: style.png
            content_check_sum: af20a283acdf3d6a18d1b29a1e011955c57bf64e3167016283df1fe42cf9e1db
        mem_limit: 4294967296
        output_volume_map:
          stylized_image: stylized_image.png
        pod_hash: 13d69656d396c272588dd875b2802faee1a56bd985e3c43c7db276a373bc9ddb
        retry_policy: NoRetry
    "},
        "YAML serialization didn't match."
    );
    Ok(())
}

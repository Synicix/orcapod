#![allow(clippy::panic_in_result_fn, clippy::unwrap_used, reason = "test code")]
//! Tests for pipeline creation functionality.
//!
//! This module contains tests that verify the correct creation of pipelines
//! using the `pipeline` fixture. The tests ensure that the pipeline creation
//! process completes successfully and outputs the expected results.

pub mod fixture;
use fixture::{pipeline, pipeline_builder, pipeline_job};
use orcapod::{core::pipeline_runner::docker::DockerPipelineRunner, uniffi::error::Result};

#[test]
fn root_nodes() -> Result<()> {
    let pipeline = pipeline()?;

    assert_eq!(pipeline.get_root_nodes().count(), 1);
    Ok(())
}

#[test]
fn get_leaf_nodes() -> Result<()> {
    let pipeline = pipeline()?;

    assert_eq!(pipeline.get_leaf_nodes().count(), 1);
    Ok(())
}

#[test]
fn get_parents_key_for_node() -> Result<()> {
    let pipeline = pipeline()?;
    let node_key = pipeline.get_root_nodes().next().unwrap();

    assert_eq!(pipeline.get_parents_key_for_node(node_key).count(), 0);
    Ok(())
}

#[test]
fn builder_with_input_nodes() -> Result<()> {
    pipeline_builder()?;
    Ok(())
}

#[test]
fn test_pipeline_creation() -> Result<()> {
    let pipeline = pipeline()?;

    assert!(
        pipeline.annotation.is_some(),
        "Pipeline annotation is missing."
    );

    assert!(
        pipeline.input_nodes.len() == 1,
        "Pipeline should have exactly one input node."
    );

    assert!(
        pipeline.output_nodes.len() == 1,
        "Pipeline should have exactly one output node."
    );

    assert!(pipeline.kernel_lut.len() == 4, "Pipeline should have three kernels in the LUT.");
    Ok(())
}

/// Pipeline Runner Tests
/// This module contains tests for the pipeline runner functionality.
#[tokio::test]
async fn pipeline_run() -> Result<()> {
    let pipeline_job = pipeline_job()?;

    let mut docker_pipeline_runner = DockerPipelineRunner::new();

    let pipeline_run = docker_pipeline_runner.start(pipeline_job)?;

    Ok(())
}

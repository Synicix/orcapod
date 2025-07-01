#![allow(clippy::panic_in_result_fn, clippy::unwrap_used, reason = "test code")]
//! Tests for pipeline creation functionality.
//!
//! This module contains tests that verify the correct creation of pipelines
//! using the `pipeline` fixture. The tests ensure that the pipeline creation
//! process completes successfully and outputs the expected results.

pub mod fixture;
use fixture::{pipeline, pipeline_builder};
use orcapod::uniffi::error::Result;

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
fn pipeline_creation() -> Result<()> {
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

    assert!(
        pipeline.kernel_lut.len() == 4,
        "Pipeline should have three kernels in the LUT."
    );
    Ok(())
}

#[test]
fn unconnected_root_nodes() -> Result<()> {
    let mut pipeline = pipeline_builder()?.pipeline;
    // Set the output nodes to all leaves of the node
    pipeline.output_nodes = pipeline
        .get_leaf_nodes()
        .map(|node| node.hash.clone())
        .collect();

    // Run verify and make sure it fails
    assert!(
        pipeline.verify().is_err(),
        "Pipeline verification should fail when input_nodes nodes are not connected to root nodes."
    );

    // Fix the input nodes
    pipeline.input_nodes = pipeline
        .get_root_nodes()
        .map(|node| node.hash.clone())
        .collect();

    // Verify should pass now
    assert!(
        pipeline.verify().is_ok(),
        "Pipeline verification should pass after fixing input nodes."
    );
    Ok(())
}

#[test]
fn dangling_child_nodes() -> Result<()> {
    let mut pipeline = pipeline_builder()?.pipeline;
    // Set the input nodes to root nodes
    pipeline.input_nodes = pipeline
        .get_root_nodes()
        .map(|node| node.hash.clone())
        .collect();

    // Run verify to make sure it fails
    assert!(
        pipeline.verify().is_err(),
        "Pipeline verification should fail when output nodes doesn't include all children"
    );

    // Fix the output nodes
    pipeline.output_nodes = pipeline
        .get_leaf_nodes()
        .map(|node| node.hash.clone())
        .collect();

    // Verify should pass now
    assert!(
        pipeline.verify().is_ok(),
        "Pipeline verification should pass after fixing output nodes."
    );
    Ok(())
}

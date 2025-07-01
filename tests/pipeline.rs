#![allow(clippy::panic_in_result_fn, clippy::unwrap_used, reason = "test code")]
//! Tests for pipeline creation functionality.
//!
//! This module contains tests that verify the correct creation of pipelines
//! using the `pipeline` fixture. The tests ensure that the pipeline creation
//! process completes successfully and outputs the expected results.

pub mod fixture;
use fixture::{pipeline, pipeline_builder};
use orcapod::uniffi::error::Result;

use crate::fixture::pod_append_name;

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

#[test]
/// This test two things:
/// 1. An edge can be added between two nodes that are already in the pipeline, which will trigger a rehash of the `to_node` and its children.
/// 2. The pipeline verification will fail if a node has two parents and the node is not a `JoinNode`.
///
/// Pending Design Question: Should we keep the pipeline builder which will later be mapped to python or pull it out and dump in a simpler one for the this stride?
fn none_join_node_with_two_parents() -> Result<()> {
    // Get the fixture pipeline A -> B -> C
    let mut pipeline_builder = pipeline_builder()?;

    // Set the parent and child nodes
    pipeline_builder.pipeline.input_nodes = pipeline_builder
        .pipeline
        .get_root_nodes()
        .map(|node| node.hash.clone())
        .collect();

    pipeline_builder.pipeline.output_nodes = pipeline_builder
        .pipeline
        .get_leaf_nodes()
        .map(|node| node.hash.clone())
        .collect();

    // Create a new node called D and set it as root node
    let node_d = pod_append_name("D")?;

    // Get the node B hash from the graph
    let (node_b_hash_ref, _) = pipeline_builder
        .pipeline
        .labels
        .iter()
        .find(|(_, label)| *label == "B")
        .unwrap();

    let node_b_hash = node_b_hash_ref.clone();

    // Get the node C hash for comparison later
    let old_node_c_hash = pipeline_builder
        .pipeline
        .get_leaf_nodes()
        .next()
        .unwrap()
        .hash
        .clone();

    // Add it the the pipeline
    let node_d_handle = pipeline_builder.add_node(node_d);

    // Add the edge from D -> B
    node_d_handle
        .pipeline_builder
        .add_edge(&node_d_handle.node_hash, &node_b_hash)?;

    // Verify that Node B hash is now updated
    assert!(
        node_d_handle
            .pipeline_builder
            .pipeline
            .get_leaf_nodes()
            .next()
            .unwrap()
            .hash
            != node_b_hash,
        "Node B hash was not updated after adding a new node D."
    );

    // Verify that Node C hash is also update
    assert!(
        node_d_handle
            .pipeline_builder
            .pipeline
            .get_leaf_nodes()
            .next()
            .unwrap()
            .hash
            != old_node_c_hash,
        "Node C hash was not updated after adding a new edge D -> B."
    );

    // Verify the pipeline which should fail
    assert!(
        pipeline_builder.pipeline.verify().is_err(),
        "Pipeline verification should fail when a node has two parents."
    );
    Ok(())
}

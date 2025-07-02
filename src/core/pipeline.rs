use serde::{Deserialize, Serialize};
use snafu::OptionExt as _;
use std::{
    backtrace::Backtrace,
    collections::{HashMap, HashSet},
    string::String,
    vec,
};

use crate::{
    core::{crypto::hash_buffer, model::to_yaml},
    uniffi::{
        error::{Kind, OrcaError, Result, selector},
        model::{Annotation, PathSet, Pod},
    },
};
use petgraph::prelude::NodeIndex;
use petgraph::{
    Directed,
    Direction::{Incoming, Outgoing},
    Graph,
};

use crate::core::model::serialize_hashmap;

use super::util::get;

/// Pipeline Components
/// Mapper
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Mapper {
    /// Hash of the Mapper
    pub hash: String,
    #[serde(serialize_with = "serialize_hashmap")]
    /**
    Mapping of `input_stream_keys` to `output_stream_keys` of the mapper
    */
    pub mapping: HashMap<String, String>,
}

impl Mapper {
    /// New function for mapping that computes the hash for
    /// # Errors
    /// Will error if it fails to convert to yaml
    pub fn new(mapping: HashMap<String, String>) -> Result<Self> {
        let no_hash = Self {
            hash: String::new(),
            mapping,
        };

        Ok(Self {
            hash: hash_buffer(to_yaml(&no_hash)?),
            ..no_hash
        })
    }
}

#[derive(Serialize, PartialEq, Debug, Clone)]
/// Enum to store different types of nodes explicitly
pub enum Kernel {
    /// Pod node
    Pod(Box<Pod>),
    /// Mapper node
    Mapper(Mapper),
    /// Joiner node
    Joiner,
}

impl Kernel {
    /// Get the hash of the node
    pub fn get_hash(&self) -> String {
        match self {
            Self::Pod(pod) => pod.hash.clone(),
            Self::Mapper(mapper) => mapper.hash.clone(),
            Self::Joiner => hash_buffer(b"Joiner"),
        }
    }

    fn extract_annotation_name_if_exist(&self) -> Option<String> {
        match self {
            Self::Pod(pod) => pod
                .annotation
                .as_ref()
                .map(|annotation| annotation.name.clone()),
            Self::Mapper(_) | Self::Joiner => None,
        }
    }

    fn get_input_keys(&self) -> Vec<&String> {
        match self {
            Self::Pod(pod) => pod.input_spec.keys().collect(),
            Self::Mapper(mapper) => mapper.mapping.keys().collect(),
            Self::Joiner => Vec::new(), // Joiner does not have input keys
        }
    }

    /// Returns the output keys for the kernel, except for joiner which returns None
    fn get_output_keys(&self) -> Vec<&String> {
        match self {
            Self::Pod(pod) => pod.output_spec.keys().collect(),
            Self::Mapper(mapper) => mapper.mapping.values().collect(),
            Self::Joiner => Vec::new(), // Joiner does not have output keys
        }
    }

    const fn is_joiner(&self) -> bool {
        matches!(self, Self::Joiner)
    }
}

impl From<Pod> for Kernel {
    fn from(pod: Pod) -> Self {
        Self::Pod(Box::new(pod))
    }
}
impl From<Mapper> for Kernel {
    fn from(mapper: Mapper) -> Self {
        Self::Mapper(mapper)
    }
}

#[derive(Serialize, Debug, Default, Clone, PartialEq, Eq)]
/// Struct to represent a node in the pipeline graph
pub struct Node {
    /// Hash is kernel hash + `parent_node_hashes`
    pub hash: String,
    /// Hash of the kernel to use in `kernel_lut`
    pub kernel_hash: String,
}

impl Node {
    /// Creates a new `Node` instance and computes its hash based on the kernel hash and parent hashes.
    pub fn new(kernel_hash: &str, parent_hashes: Vec<&str>) -> Self {
        Self {
            hash: Self::compute_hash(kernel_hash, parent_hashes),
            kernel_hash: kernel_hash.to_owned(),
        }
    }

    fn compute_hash(kernel_hash: &str, parent_hashes: Vec<&str>) -> String {
        // Sort the parent hashes to ensure consistent ordering
        let mut sorted_hashes = parent_hashes;
        sorted_hashes.sort_unstable();

        // Combine all parent hashes with the kernel hash
        let mut buffer = kernel_hash.to_owned();
        for hash in sorted_hashes {
            buffer.push_str(hash);
        }

        hash_buffer(buffer.as_bytes())
    }
}

/// Pipeline struct
#[derive(Serialize, Debug, Default, Clone)]
pub struct Pipeline {
    hash: String,
    #[serde(skip)]
    /// Annotation for the pipeline
    pub annotation: Option<Annotation>,
    /// Strings are the hash of the kernel, and the value is the kernel itself.
    /// Mainly used to prevent duplicate storage of kernels share by multiple nodes.
    pub kernel_lut: HashMap<String, Kernel>,
    /// Labels provided by the user for each node where the key is the node hash and the value the actual label
    pub labels: HashMap<String, String>,
    /// Strings are unique hashes of the nodes with the _{`num_matches`}
    pub graph: Graph<Node, ()>,
    /// Nodes where the input data should be fed into
    pub input_nodes: HashSet<String>,
    /// Nodes where the output data should be collected from and outputted.
    pub output_nodes: HashSet<String>,
}

impl Pipeline {
    /// Creates a new `Pipeline` instance.
    /// # Errors
    /// Will error out if the pipeline is not valid
    pub fn new(
        annotation: Option<Annotation>,
        kernel_lut: HashMap<String, Kernel>,
        labels: HashMap<String, String>,
        graph: Graph<Node, (), Directed>,
        input_nodes: HashSet<String>,
        output_nodes: HashSet<String>,
    ) -> Result<Self> {
        let pipeline = Self {
            hash: String::new(), // TODO: Need to implement to yaml then hash that
            annotation,
            kernel_lut,
            labels,
            graph,
            input_nodes,
            output_nodes,
        };

        pipeline.verify()?;
        Ok(pipeline)
    }

    /// Function to verify that the pipeline is valid
    ///
    /// # Errors
    /// Will return an error if
    /// - There are disconnected root nodes (nodes that have no parents but are not in the `input_nodes`)
    pub fn verify(&self) -> Result<()> {
        // Verify that the graph is valid

        // Check if all root nodes are listed in the input_nodes, if not there are disconnected nodes
        self.get_root_nodes().try_for_each(|node| {
            if self.input_nodes.contains(&node.hash) {
                Ok::<(), OrcaError>(())
            } else {
                Err(OrcaError {
                    kind: Kind::DisconnectedRootNode {
                        node_name: get(&self.labels, &node.hash)?.to_owned(),
                        backtrace: Some(Backtrace::capture()),
                    },
                })
            }
        })?;

        // Check if all leaf nodes are listed in the output_nodes, if not there are disconnected nodes
        self.get_leaf_nodes().try_for_each(|node| {
            if self.output_nodes.contains(&node.hash) {
                Ok::<(), OrcaError>(())
            } else {
                Err(OrcaError {
                    kind: Kind::DisconnectedLeafNode {
                        node_name: get(&self.labels, &node.hash)?.to_owned(),
                        backtrace: Some(Backtrace::capture()),
                    },
                })
            }
        })?;

        // Check if any node that has more than one parent, if so, it must be of type joiner otherwise throw error
        self.graph.node_indices().try_for_each(|idx| {
            let num_of_parents = self.graph.neighbors_directed(idx, Incoming).count();

            if num_of_parents > 1 {
                let node = &self.graph[idx];
                return match self.get_kernel(&node.kernel_hash)? {
                    Kernel::Joiner => Ok(()),
                    Kernel::Pod(_) | Kernel::Mapper(_) => Err(OrcaError {
                        kind: Kind::NonJoinerNodeHasMoreThanOneParent {
                            node_name: node.hash.clone(), // Change to label later
                            backtrace: Some(Backtrace::capture()),
                        },
                    }),
                };
            }

            Ok(())
        })?;

        // For every node that is not a joiner or root node, verify that input specs are met
        self.graph.node_indices().try_for_each(|node_idx| {
            let node = &self.graph[node_idx];
            if !self.input_nodes.contains(&node.hash)
                && !get(&self.kernel_lut, &node.kernel_hash)?.is_joiner()
            {
                self.verify_input_spec_are_met(node_idx)?;
            }
            Ok::<_, OrcaError>(())
        })?;

        Ok(())
    }

    fn verify_input_spec_are_met(&self, node_idx: NodeIndex) -> Result<()> {
        // Get the kernel for the node
        let kernel = get(&self.kernel_lut, &self.graph[node_idx].kernel_hash)?;

        // Get the parent node for the current node which should only be one parent
        let parent_node = self
            .get_parents_for_node(&self.graph[node_idx])
            .next()
            .context(selector::ParentNodeNotFound {
                parent_node_hash: self.graph[node_idx].hash.clone(),
            })?;

        // Get the output_keys from the parent kernel, unless it is a joiner.
        let parent_output_keys = self.get_output_keys(parent_node)?;

        // Verify that the input spec of the kernel is met by the parent output keys
        kernel.get_input_keys().iter().try_for_each(|input_key| {
            if parent_output_keys.contains(input_key) {
                Ok(())
            } else {
                Err(OrcaError {
                    kind: Kind::InputSpecNotMet {
                        node_hash: self.graph[node_idx].hash.clone(),
                        input_spec_key: (*input_key).clone(),
                        backtrace: Some(Backtrace::capture()),
                    },
                })
            }
        })?;

        Ok(())
    }

    fn get_output_keys(&self, node: &Node) -> Result<Vec<&String>> {
        // Get the kernel for the node
        let kernel = get(&self.kernel_lut, &node.kernel_hash)?;

        if kernel.is_joiner() {
            // Parent node is type joiner, thus we need get its parent's output_keys
            Ok(self
                .get_parents_for_node(node)
                .map(|parent_node| self.get_output_keys(parent_node))
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .flatten()
                .collect())
        } else {
            Ok(kernel.get_output_keys())
        }
    }

    /// # Errors
    /// Error out if the `kernel_key` is not found in the `kernel_lut`
    #[expect(clippy::string_slice, reason = "Should never fail as we are in")]
    pub fn get_kernel(&self, kernel_key: &str) -> Result<&Kernel> {
        let char_to_cut_at = '_';

        let key = kernel_key
            .rfind(char_to_cut_at)
            .map_or(kernel_key, |index| &kernel_key[..index]);
        get(&self.kernel_lut, &key.to_owned())
    }

    /// Function to get the root nodes of the pipeline
    pub fn get_root_nodes(&self) -> impl Iterator<Item = &Node> {
        self.graph
            .node_indices()
            .filter(|&node_index| self.graph.neighbors_directed(node_index, Incoming).count() == 0)
            .map(|node_index| &self.graph[node_index])
    }

    /// Looks through the graph and find nodes that don't have any parents
    pub fn get_root_nodes_idx(&self) -> impl Iterator<Item = NodeIndex> {
        self.graph
            .node_indices()
            .filter(|&node_index| self.graph.neighbors_directed(node_index, Incoming).count() == 0)
    }

    /// Function to get the leaf nodes of the pipeline
    /// Mainly used to get the output nodes when user does not specify them
    pub fn get_leaf_nodes(&self) -> impl Iterator<Item = &Node> {
        // Leaf nodes are those that are not keys in the edges map (i.e., not parents of any node)
        self.graph
            .node_indices()
            .filter(|&node_index| {
                self.graph
                    .neighbors_directed(node_index, Outgoing)
                    .next()
                    .is_none()
            })
            .map(|node_index| &self.graph[node_index])
    }

    /// Function to get the parents of a node
    pub fn get_parents_for_node(&self, node: &Node) -> impl Iterator<Item = &Node> {
        // Find the NodeIndex for the given node_key
        let node_index = self
            .graph
            .node_indices()
            .find(|&idx| self.graph[idx] == *node);
        node_index.into_iter().flat_map(move |idx| {
            self.graph
                .neighbors_directed(idx, Incoming)
                .map(move |parent_idx| &self.graph[parent_idx])
        })
    }
}

impl PartialEq for Pipeline {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
            && self.kernel_lut == other.kernel_lut
            && self.output_nodes == other.output_nodes
    }
}

#[derive(Serialize, Debug, Clone)]
/// `PipelineJob` struct
/// This struct is used to store the pipeline and the input map
pub struct PipelineJob {
    /// Hash of the pipeline job
    pub hash: String,
    /// Pipeline struct
    pub pipeline: Pipeline,
    #[serde(serialize_with = "serialize_hashmap")]
    /// Mapping of outside input to keys to be match with the pipeline `input_map`
    pub input_map: HashMap<String, PathSet>,
    annotation: Option<Annotation>,
}

impl PipelineJob {
    /// New function for pipeline job
    /// # Errors
    /// Error out if there are missing keys or failed to convert to yaml
    pub fn new(
        pipeline: Pipeline,
        input_packet: HashMap<String, PathSet>,
        annotation: Option<Annotation>,
    ) -> Result<Self> {
        // Check if input_map has all the requires keys
        let missing_keys = pipeline
            .get_root_nodes()
            .map(|node| match pipeline.get_kernel(&node.kernel_hash)? {
                Kernel::Pod(pod) => Ok(find_missing_keys(&input_packet, pod.input_spec.keys())),
                Kernel::Mapper(mapper) => {
                    Ok(find_missing_keys(&input_packet, mapper.mapping.keys()))
                }
                Kernel::Joiner => Ok(Vec::<String>::new()), // Should probably error out because joiner should not be a root node
            })
            .collect::<Result<Vec<Vec<String>>>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<String>>();

        if !missing_keys.is_empty() {
            return Err(OrcaError {
                kind: Kind::MissingStreamKey {
                    missing_keys,
                    backtrace: Some(Backtrace::capture()),
                },
            });
        }

        Ok(Self {
            pipeline,
            input_map: input_packet,
            annotation,
            hash: String::new(),
        })
    }
}

fn find_missing_keys<'a>(
    input_map: &HashMap<String, PathSet>,
    keys_to_check: impl Iterator<Item = &'a String>,
) -> Vec<String> {
    keys_to_check
        .filter_map(|key| {
            if input_map.contains_key(key) {
                None
            } else {
                Some(key.clone())
            }
        })
        .collect()
}

/// Helper struct to assist in defining a pipeline in Rust
pub struct PipelineBuilder {
    /// Internal representation of the pipeline being built
    pub pipeline: Pipeline,
}

impl Default for PipelineBuilder {
    fn default() -> Self {
        Self {
            pipeline: Pipeline {
                hash: String::new(),
                annotation: None,
                kernel_lut: HashMap::new(),
                labels: HashMap::new(),
                graph: Graph::new(),
                input_nodes: HashSet::new(),
                output_nodes: HashSet::new(),
            },
        }
    }
}

impl PipelineBuilder {
    /// Creates a new `PipelineBuilder` instance.
    pub fn new(annotation: Option<Annotation>) -> Self {
        Self {
            pipeline: Pipeline {
                annotation,
                ..Default::default()
            },
        }
    }

    /// Convert the nodes given by the user into a `Kernel` and add it to the pipeline.
    ///
    /// Cases:
    /// 1. If the kernel is not in the `pipeline.kernel_lut`, then add it to the lut and graph
    /// 2. If the kernel is already in the `pipeline.kernel_lut`, then skip adding it to the lut and graph
    pub fn add_node(&mut self, node: impl Into<Kernel>) -> NodeHandle<'_> {
        // Convert the node into a Kernel and add it to kernel_lut if it does not exist
        let kernel = node.into();
        self.add_kernel_to_lut_if_not_exists(&kernel);

        // Build the node
        let node_to_add = Node::new(&kernel.get_hash(), vec![]);

        self.add_label_from_kernel_annotation_if_not_exist(&node_to_add.hash, &kernel);

        // Insert the node into the graph
        self.pipeline.graph.add_node(node_to_add.clone());

        NodeHandle {
            node_hash: node_to_add.hash,
            pipeline_builder: self,
        }
    }

    /// Function to add an edge from one node to another in the pipeline graph
    /// This requires them to already in the pipeline
    ///
    /// # Errors
    /// Will error out if the `from_node_hash` or `to_node_hash` is not found in the graph
    pub fn add_edge(&mut self, from_node_hash: &str, to_node_hash: &str) -> Result<String> {
        // Check if both nodes exist in the graph
        let from_node_idx = self
            .pipeline
            .graph
            .node_indices()
            .find(|&idx| self.pipeline.graph[idx].hash == from_node_hash)
            .ok_or(OrcaError {
                kind: Kind::ParentNodeNotFound {
                    parent_node_hash: from_node_hash.to_owned(),
                    backtrace: Some(Backtrace::capture()),
                },
            })?;

        let to_node_idx = self
            .pipeline
            .graph
            .node_indices()
            .find(|&idx| self.pipeline.graph[idx].hash == to_node_hash)
            .ok_or(OrcaError {
                kind: Kind::ParentNodeNotFound {
                    parent_node_hash: to_node_hash.to_owned(),
                    backtrace: Some(Backtrace::capture()),
                },
            })?;

        // Figure out if node already has a parent and it not of type joiner
        // If so, we need to inject a joiner in between the multiple parents and
        // Get all the parents indices
        let mut parents_indices: Vec<NodeIndex> = self
            .pipeline
            .graph
            .neighbors_directed(to_node_idx, Incoming)
            .collect();

        if parents_indices.is_empty() {
            // Add the new edge
            self.pipeline.graph.add_edge(from_node_idx, to_node_idx, ());
        } else {
            // There is already a preexisting parent, so we need to add a joiner node

            // Delete all the edges from the parents to the target node
            for parent_idx in &parents_indices {
                self.pipeline.graph.remove_edge(
                    self.pipeline
                        .graph
                        .find_edge(*parent_idx, to_node_idx)
                        .context(selector::NoEdgeFound {
                            from_node_hash: from_node_hash.to_owned(),
                            to_node_hash: to_node_hash.to_owned(),
                        })?,
                );
            }

            // Get all the parents hashes
            let parent_hashes_string = parents_indices
                .iter()
                .map(|idx| self.pipeline.graph[*idx].hash.clone())
                .collect::<Vec<_>>();

            let mut parent_hashes = parent_hashes_string
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>();

            // Add in the new parent hash to the list of parent hashes
            parent_hashes.push(from_node_hash);

            // Add the joiner node if it doesn't already exists to the kernel_lut
            let joiner_kernel_hash = self.add_kernel_to_lut_if_not_exists(&Kernel::Joiner);

            // Create the joiner node
            let joiner_node = Node::new(&joiner_kernel_hash, parent_hashes);

            // Add the joiner node to the graph
            let joiner_node_idx = self.pipeline.graph.add_node(joiner_node);

            // Add the from_node_idx as a parent of the joiner node
            parents_indices.push(from_node_idx);

            // Add the new parents edges to the joiner node
            for parent_idx in &parents_indices {
                self.pipeline
                    .graph
                    .add_edge(*parent_idx, joiner_node_idx, ());
            }

            // Add edges from the joiner node to the target node
            self.pipeline
                .graph
                .add_edge(joiner_node_idx, to_node_idx, ());
        }

        self.propagate_new_parent_hash_to_children(to_node_idx, from_node_hash)
    }

    /// Function to convert the `PipelineBuilder` into a `Pipeline`
    ///
    /// # Errors
    /// Will error out if the pipeline is not valid
    pub fn to_pipeline(&self) -> Result<Pipeline> {
        Pipeline::new(
            self.pipeline.annotation.clone(),
            self.pipeline.kernel_lut.clone(),
            self.pipeline.labels.clone(),
            self.pipeline.graph.clone(),
            if self.pipeline.input_nodes.is_empty() {
                // If there are no input nodes, then we need to set the input nodes to the root nodes
                self.pipeline
                    .get_root_nodes()
                    .map(|node| node.hash.clone())
                    .collect()
            } else {
                self.pipeline.input_nodes.clone()
            },
            if self.pipeline.output_nodes.is_empty() {
                // If there are no output nodes, then we need to set the output nodes to the leaf nodes
                self.pipeline
                    .get_leaf_nodes()
                    .map(|node| node.hash.clone())
                    .collect()
            } else {
                self.pipeline.output_nodes.clone()
            },
        )
    }

    fn propagate_new_parent_hash_to_children(
        &mut self,
        node_idx: NodeIndex,
        new_parent_hash: &str,
    ) -> Result<String> {
        // First, collect the parent hashes before mutably borrowing the graph
        let node = &self.pipeline.graph[node_idx];
        let kernel_hash = node.kernel_hash.clone();
        let old_hash = node.hash.clone();

        let mut parent_hashes = self
            .pipeline
            .get_parents_for_node(node)
            .map(|parent_node| parent_node.hash.clone())
            .collect::<Vec<_>>();
        parent_hashes.push(new_parent_hash.to_owned());

        let new_hash = Node::compute_hash(
            &kernel_hash,
            parent_hashes.iter().map(String::as_str).collect(),
        );

        // Now, safely mutably borrow the node and update its hash
        if let Some(mut_node) = self.pipeline.graph.node_weight_mut(node_idx) {
            mut_node.hash.clone_from(&new_hash);
        } else {
            return Err(OrcaError {
                kind: Kind::ParentNodeNotFound {
                    parent_node_hash: old_hash.clone(),
                    backtrace: Some(Backtrace::capture()),
                },
            });
        }

        // Update the labels if the node had a label
        if let Some(label) = self.pipeline.labels.get(&old_hash) {
            // Insert new one
            self.pipeline.labels.insert(new_hash.clone(), label.clone());
            // Delete the old
            self.pipeline.labels.remove(&old_hash);
        }

        // Now, propagate the new hash to all children of this node
        let children_indices = self
            .pipeline
            .graph
            .neighbors_directed(node_idx, Outgoing)
            .collect::<Vec<_>>();

        for child_idx in children_indices {
            // Recursively propagate the new parent hash to each child
            self.propagate_new_parent_hash_to_children(child_idx, &new_hash)?;
        }

        Ok(new_hash)
    }

    /// Function to add a kernel to the pipeline's kernel lookup table (`kernel_lut`)
    /// If the kernel already exists, it will not be added again.
    /// # Returns
    /// Returns the hash of the kernel that was added or already exists
    fn add_kernel_to_lut_if_not_exists(&mut self, kernel: &Kernel) -> String {
        let kernel_hash = kernel.get_hash();
        // Check if the kernel is already in the pipeline.kernel_lut
        self.pipeline
            .kernel_lut
            .entry(kernel_hash.clone())
            .or_insert_with(|| kernel.clone());

        kernel_hash
    }

    fn add_label_from_kernel_annotation_if_not_exist(&mut self, node_hash: &str, kernel: &Kernel) {
        // Add label if there is an annotation that exist from kernel
        if let Some(label) = Kernel::extract_annotation_name_if_exist(kernel) {
            // Check if the label already exists
            self.pipeline
                .labels
                .entry(node_hash.to_owned())
                .or_insert_with(|| label.clone());
        }
    }
}

/// Handle to store the `node_hash` for the user to add children to it
pub struct NodeHandle<'a> {
    /// The hash of the node
    pub node_hash: String,
    /// Mutable reference to the `PipelineBuilder` to allow adding children
    pub pipeline_builder: &'a mut PipelineBuilder,
}

impl NodeHandle<'_> {
    /// Add an kernel as a child to the current `node_hash`
    ///
    /// # Errors
    /// Errors out if `node_hash` is not in the pipeline or if the kernel is not a valid kernel
    pub fn add_child(&mut self, kernel: impl Into<Kernel>) -> Result<NodeHandle<'_>> {
        // Add the node to the pipeline first
        let mut node_handle = self.pipeline_builder.add_node(kernel.into());

        // Add the edge from the current node to the new node
        node_handle.node_hash = node_handle
            .pipeline_builder
            .add_edge(&self.node_hash, node_handle.node_hash.as_str())?;
        // Print out pipeline for debugging

        Ok(node_handle)
    }
}

/// Result of a pipeline job
/// For now it is bare bones due to incomplete implementation of the`pipeline_runner`r
pub struct PipelineResult {
    /// Reference to the pipeline job
    pub pipeline_job: PipelineJob,
}

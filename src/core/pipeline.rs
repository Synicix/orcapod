use serde::{Deserialize, Serialize};
use std::{
    backtrace::Backtrace,
    collections::{HashMap, HashSet},
    vec,
};

use crate::{
    core::{crypto::hash_buffer, model::to_yaml},
    uniffi::{
        error::{Kind, OrcaError, Result},
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
    hash: String,
    /// Hash of the kernel to use in `kernel_lut`
    kernel_hash: String,
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
                Ok(())
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
                Ok(())
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

        Ok(())
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
    pub fn get_parents_key_for_node(&self, node_hash: &str) -> impl Iterator<Item = &Node> {
        // Find the NodeIndex for the given node_key
        let node_index = self
            .graph
            .node_indices()
            .find(|&idx| self.graph[idx].hash == node_hash);
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

impl From<PipelineBuilder> for Pipeline {
    fn from(val: PipelineBuilder) -> Self {
        let mut pipeline = val.pipeline;

        if pipeline.input_nodes.is_empty() {
            // If there are no input nodes, then we need to set the input nodes to the root nodes
            pipeline.input_nodes = pipeline
                .get_root_nodes()
                .map(|node| node.hash.clone())
                .collect();
        }

        if pipeline.output_nodes.is_empty() {
            // If there are no output nodes, then we need to set the output nodes to the leaf nodes
            pipeline.output_nodes = pipeline
                .get_leaf_nodes()
                .map(|node| node.hash.clone())
                .collect();
        }

        pipeline
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
    pub fn add_node(&mut self, node: impl Into<Kernel>, label: Option<&str>) -> NodeHandle<'_> {
        // Convert the node into a Kernel and add it to kernel_lut if it does not exist
        let kernel = node.into();
        self.add_kernel_to_lut_if_not_exists(&kernel);

        // Build the node
        let node_to_add = Node::new(&kernel.get_hash(), vec![]);

        // Add the label to the pipeline.labels if it exists
        if let Some(node_label) = label {
            self.add_node_label_if_not_exists(&node_to_add.hash, node_label);
        }

        // Insert the node into the graph
        self.pipeline.graph.add_node(node_to_add.clone());

        NodeHandle {
            node_hash: node_to_add.hash,
            pipeline_builder: self,
        }
    }

    fn add_edge_from_node(
        &mut self,
        from_node_hash: &str,
        to_kernel: impl Into<Kernel>,
    ) -> Result<NodeHandle> {
        // Convert the node into a Kernel and add it to kernel_lut if it does not exist
        let kernel = to_kernel.into();
        self.add_kernel_to_lut_if_not_exists(&kernel);

        // Create the node
        let node = Node {
            hash: Node::compute_hash(&kernel.get_hash(), vec![from_node_hash]),
            kernel_hash: kernel.get_hash(),
        };

        // Add the node to the pipeline
        let new_node_idx = self.pipeline.graph.add_node(node.clone());
        self.pipeline.graph.add_edge(
            self.pipeline
                .graph
                .node_indices()
                .find(|&idx| self.pipeline.graph[idx].hash == from_node_hash)
                .ok_or(OrcaError {
                    kind: Kind::NodeNotFound {
                        parent_node_key: from_node_hash.to_owned(),
                        backtrace: Some(Backtrace::capture()),
                    },
                })?,
            new_node_idx,
            (),
        );

        Ok(NodeHandle {
            node_hash: node.hash,
            pipeline_builder: self,
        })
    }

    fn add_kernel_to_lut_if_not_exists(&mut self, kernel: &Kernel) {
        // Check if the kernel is already in the pipeline.kernel_lut
        self.pipeline
            .kernel_lut
            .entry(kernel.get_hash())
            .or_insert_with(|| kernel.clone());
    }

    fn add_node_label_if_not_exists(&mut self, node_hash: &str, label: &str) {
        // Check if the label already exists
        if !self.pipeline.labels.contains_key(node_hash) {
            // If it does not, then we need to add it to the labels
            self.pipeline
                .labels
                .insert(node_hash.to_owned(), label.to_owned());
        }
    }
}

/// Handle to store the `node_key` for the user to add children to it
pub struct NodeHandle<'a> {
    node_hash: String,
    pipeline_builder: &'a mut PipelineBuilder,
}

impl NodeHandle<'_> {
    /// Add an node as a child to the current `node_key`
    /// # Errors
    /// Shouldn't error as long the self is in the graph
    pub fn add_child(&mut self, node: impl Into<Kernel>) -> Result<NodeHandle<'_>> {
        self.pipeline_builder
            .add_edge_from_node(&self.node_hash, node)
    }
}

/// Result of a pipeline job
/// For now it is bare bones due to incomplete implementation of the`pipeline_runner`r
pub struct PipelineResult {
    /// Reference to the pipeline job
    pub pipeline_job: PipelineJob,
}

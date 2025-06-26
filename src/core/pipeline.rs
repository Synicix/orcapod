use serde::{Deserialize, Serialize};
use std::{
    backtrace::Backtrace,
    collections::{HashMap, HashSet},
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

#[derive(Serialize, PartialEq, Eq, Debug, Clone)]
pub struct Joiner {
    hash: String,
    /// Storage buffer to store the results from n parent nodes +
    buffer: HashMap<String, Vec<HashMap<String, PathSet>>>,
}

impl Joiner {
    fn new(mut parent_hashes: Vec<String>) -> Self {
        // Sort the parent hashes to ensure consistent ordering
        parent_hashes.sort();

        // Combine all parent hashes into a single hash
        let mut buffer = String::new();
        for hash in &parent_hashes {
            buffer.push_str(hash);
        }

        Self {
            hash: hash_buffer(buffer.as_bytes()),
            buffer: HashMap::new(),
        }
    }
}

#[derive(Serialize, PartialEq, Debug, Clone)]
/// Enum to store different types of nodes explicitly
pub enum Kernel {
    /// Pod node
    Pod(Box<Pod>),
    /// Mapper node
    Mapper(Mapper),
    Joiner(Joiner),
}

impl Kernel {
    /// Get the hash of the node
    pub fn get_hash(&self) -> String {
        match self {
            Self::Pod(pod) => pod.hash.clone(),
            Self::Mapper(mapper) => mapper.hash.clone(),
            Self::Joiner(joiner) => joiner.hash.clone(),
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

/// Pipeline struct
#[derive(Serialize, Debug, Default, Clone)]
pub struct Pipeline {
    hash: String,
    #[serde(skip)]
    /// Annotation for the pipeline
    pub annotation: Option<Annotation>,
    /// String are hashes of the nodes without the _{`num_matches`}
    pub kernel_lut: HashMap<String, Kernel>,
    /// Strings are unique hashes of the nodes with the _{`num_matches`}
    pub graph: Graph<String, ()>,
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
        nodes: HashMap<String, Kernel>,
        graph: Graph<String, (), Directed>,
        input_nodes: HashSet<String>,
        output_nodes: HashSet<String>,
    ) -> Result<Self> {
        let pipeline = Self {
            hash: String::new(), // TODO: Need to implement to yaml then hash that
            annotation,
            kernel_lut: nodes,
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
        self.get_root_nodes().try_for_each(|node_key| {
            if self.input_nodes.contains(node_key) {
                Ok(())
            } else {
                Err(OrcaError {
                    kind: Kind::DisconnectedRootNode {
                        node_key: node_key.to_owned(),
                        backtrace: Some(Backtrace::capture()),
                    },
                })
            }
        })?;

        // Check if all leaf nodes are listed in the output_nodes, if not there are disconnected nodes
        self.get_leaf_nodes().try_for_each(|node_key| {
            if self.output_nodes.contains(node_key) {
                Ok(())
            } else {
                Err(OrcaError {
                    kind: Kind::DisconnectedLeafNode {
                        node_key: node_key.to_owned(),
                        backtrace: Some(Backtrace::capture()),
                    },
                })
            }
        })?;

        Ok(())
    }

    /// # Errors
    /// Error out if the `node_key` is not found in the pipeline.nodes
    #[expect(clippy::string_slice, reason = "Should never fail as we are in")]
    pub fn get_node(&self, node_key: &str) -> Result<&Kernel> {
        let char_to_cut_at = '_';

        let key = node_key
            .rfind(char_to_cut_at)
            .map_or(node_key, |index| &node_key[..index]);
        get(&self.kernel_lut, &key.to_owned())
    }

    /// Function to get the root nodes of the pipeline
    pub fn get_root_nodes(&self) -> impl Iterator<Item = &String> {
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
    pub fn get_leaf_nodes(&self) -> impl Iterator<Item = &String> {
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
    pub fn get_parents_key_for_node(&self, node_key: &str) -> impl Iterator<Item = &String> {
        // Find the NodeIndex for the given node_key
        let node_index = self
            .graph
            .node_indices()
            .find(|&idx| self.graph[idx] == node_key);
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
            pipeline.input_nodes = pipeline.get_root_nodes().cloned().collect();
        }

        if pipeline.output_nodes.is_empty() {
            // If there are no output nodes, then we need to set the output nodes to the leaf nodes
            pipeline.output_nodes = pipeline.get_leaf_nodes().cloned().collect();
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
            .map(|node_id| match pipeline.get_node(node_id)? {
                Kernel::Pod(pod) => Ok(find_missing_keys(&input_packet, pod.input_spec.keys())),
                Kernel::Mapper(mapper) => {
                    Ok(find_missing_keys(&input_packet, mapper.mapping.keys()))
                }
                Kernel::Joiner(_) => Ok(Vec::<String>::new()),
            })
            .collect::<Result<Vec<Vec<String>>>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<String>>();

        if !missing_keys.is_empty() {
            return Err(OrcaError {
                kind: Kind::MissingStreamKey {
                    input_packet,
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
                hash: String::new(), // TODO: Need to implement to yaml then hash that
                annotation,
                kernel_lut: HashMap::new(),
                graph: Graph::new(),
                input_nodes: HashSet::new(),
                output_nodes: HashSet::new(),
            },
        }
    }

    /// Add nodes to the pipeline and return key to put in edges
    ///
    /// Cases:
    /// 1. If the node is not in the pipeline.nodes, then it is added to the `hash_map` and the key is the node hash
    /// 2. If the node is already in the pipeline.nodes, then the key is the hash + _{`num_matches`} to prevent collision
    pub fn add_node(&mut self, node: impl Into<Kernel>) -> NodeHandle<'_> {
        let node_to_insert = node.into();
        let hash = node_to_insert.get_hash();

        // Get the node_key to add to the edge
        let node_key = self.compute_node_key(&hash);

        // Insert into node hash_map if does not exist, else skip
        self.pipeline
            .kernel_lut
            .entry(node_to_insert.get_hash())
            .or_insert(node_to_insert);

        // Add it to the graph
        self.pipeline.graph.add_node(node_key.clone());

        NodeHandle {
            node_key,
            pipeline_builder: self,
        }
    }

    fn add_edge_from_node(&mut self, from: &str, node: impl Into<Kernel>) -> Result<NodeHandle> {
        // Check if node exists in the pipeline.nodes
        let node_to_insert = node.into();
        let hash = node_to_insert.get_hash();

        // Get the node_key to add to the graph
        let node_key = self.compute_node_key(&hash);
        let new_node_idx = self.pipeline.graph.add_node(node_key.clone());

        // Insert node into the pipeline.nodes lut if it does not exist
        self.pipeline
            .kernel_lut
            .entry(hash)
            .or_insert(node_to_insert);

        self.pipeline.graph.add_edge(
            self.pipeline
                .graph
                .node_indices()
                .find(|&idx| self.pipeline.graph[idx] == from)
                .ok_or(OrcaError {
                    kind: Kind::NodeNotFound {
                        parent_node_key: from.to_owned(),
                        backtrace: Some(Backtrace::capture()),
                    },
                })?,
            new_node_idx,
            (),
        );

        Ok(NodeHandle {
            node_key,
            pipeline_builder: self,
        })
    }

    fn compute_node_key(&self, node_hash: &str) -> String {
        // Check if node is already in the pipeline, if so then we need to add a numerator to the hash
        let num_matches = self
            .pipeline
            .kernel_lut
            .iter()
            .filter(|(key, _)| *key == node_hash)
            .count();

        if num_matches > 0 {
            // Node already exists, thus we need to add a numerator to the hash
            format!("{node_hash}_{num_matches}")
        } else {
            node_hash.to_owned()
        }
    }
}

/// Handle to store the `node_key` for the user to add children to it
pub struct NodeHandle<'a> {
    node_key: String,
    pipeline_builder: &'a mut PipelineBuilder,
}

impl NodeHandle<'_> {
    /// Add an node as a child to the current `node_key`
    /// # Errors
    /// Shouldn't error as long the self is in the graph
    pub fn add_child(&mut self, node: impl Into<Kernel>) -> Result<NodeHandle<'_>> {
        self.pipeline_builder
            .add_edge_from_node(&self.node_key, node)
    }
}

pub struct PipelineResult {
    /// Reference to the pipeline job
    pub pipeline_job: PipelineJob,
}

//! DAG resolution for pipeline stages.

use oxide_core::ids::StageId;
use oxide_core::pipeline::{PipelineDefinition, StageDefinition};
use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DagError {
    #[error("Cycle detected in stage dependencies")]
    CycleDetected,
    #[error("Unknown stage dependency: {0}")]
    UnknownDependency(String),
    #[error("Empty pipeline")]
    EmptyPipeline,
}

use crate::matrix::MatrixExpander;

/// A node in the pipeline DAG.
#[derive(Debug, Clone)]
pub struct DagNode {
    #[allow(dead_code)]
    pub stage_id: StageId,
    pub name: String,
    pub definition: StageDefinition,
}

/// Directed acyclic graph representing stage dependencies.
#[derive(Debug)]
pub struct PipelineDag {
    pub graph: DiGraph<DagNode, ()>,
    // Map logical stage name to all its expanded nodes (indices)
    pub name_to_nodes: HashMap<String, Vec<NodeIndex>>,
}

impl PipelineDag {
    /// Get the root stages (stages with no dependencies).
    #[allow(dead_code)]
    pub fn roots(&self) -> Vec<&DagNode> {
        self.graph
            .node_indices()
            .filter(|&idx| {
                self.graph
                    .neighbors_directed(idx, petgraph::Direction::Incoming)
                    .count()
                    == 0
            })
            .filter_map(|idx| self.graph.node_weight(idx))
            .collect()
    }

    /// Get stages that can run after a given stage completes.
    #[allow(dead_code)]
    pub fn successors(&self, stage_name: &str) -> Vec<&DagNode> {
        self.name_to_nodes
            .get(stage_name)
            .map(|indices| {
                indices
                    .iter()
                    .flat_map(|&idx| {
                        self.graph
                            .neighbors_directed(idx, petgraph::Direction::Outgoing)
                            .filter_map(|n| self.graph.node_weight(n))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get stages that must complete before a given stage can run.
    pub fn predecessors(&self, stage_name: &str) -> Vec<&DagNode> {
        self.name_to_nodes
            .get(stage_name)
            .map(|indices| {
                indices
                    .iter()
                    .flat_map(|&idx| {
                        self.graph
                            .neighbors_directed(idx, petgraph::Direction::Incoming)
                            .filter_map(|n| self.graph.node_weight(n))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get topologically sorted stages.
    pub fn topological_order(&self) -> Result<Vec<&DagNode>, DagError> {
        toposort(&self.graph, None)
            .map(|indices| {
                indices
                    .iter()
                    .filter_map(|&idx| self.graph.node_weight(idx))
                    .collect()
            })
            .map_err(|_| DagError::CycleDetected)
    }

    /// Get all stages.
    pub fn stages(&self) -> Vec<&DagNode> {
        self.graph
            .node_indices()
            .filter_map(|idx| self.graph.node_weight(idx))
            .collect()
    }

    /// Check if a stage is ready to run given completed stages.
    pub fn is_ready(&self, stage_name: &str, completed: &[String]) -> bool {
        self.predecessors(stage_name)
            .iter()
            .all(|pred| completed.contains(&pred.name))
    }
}

/// Builder for constructing pipeline DAGs.
pub struct DagBuilder;

impl DagBuilder {
    pub fn new() -> Self {
        Self
    }

    /// Build a DAG from a pipeline definition.
    pub fn build(&self, pipeline: &PipelineDefinition) -> Result<PipelineDag, DagError> {
        if pipeline.stages.is_empty() {
            return Err(DagError::EmptyPipeline);
        }

        let mut graph = DiGraph::new();
        let mut name_to_nodes: HashMap<String, Vec<NodeIndex>> = HashMap::new();
        let matrix_expander = MatrixExpander::new();

        // Add all stages as nodes (expanding matrix stages)
        for stage in &pipeline.stages {
            if let Some(matrix_jobs) = matrix_expander.expand(stage) {
                // Matrix stage: create multiple nodes
                for job in matrix_jobs {
                    let mut definition = stage.clone();
                    // Inject matrix variables
                    for (k, v) in job.variables {
                        let v_str = match v {
                            serde_json::Value::String(s) => s,
                            _ => v.to_string(),
                        };
                        definition.variables.insert(format!("matrix.{}", k), v_str);
                    }
                    // Update name to include variant
                    definition.name = job.display_name.clone();

                    let node = DagNode {
                        stage_id: StageId::new(&definition.name),
                        name: definition.name.clone(),
                        definition,
                    };
                    let idx = graph.add_node(node);
                    name_to_nodes
                        .entry(stage.name.clone())
                        .or_default()
                        .push(idx);
                }
            } else {
                // Normal stage
                let node = DagNode {
                    stage_id: StageId::new(&stage.name),
                    name: stage.name.clone(),
                    definition: stage.clone(),
                };
                let idx = graph.add_node(node);
                name_to_nodes
                    .entry(stage.name.clone())
                    .or_default()
                    .push(idx);
            }
        }

        // Add edges for dependencies
        for stage in &pipeline.stages {
            let stage_indices = name_to_nodes.get(&stage.name).unwrap().clone();

            for dep in &stage.depends_on {
                let dep_indices = name_to_nodes
                    .get(dep)
                    .ok_or_else(|| DagError::UnknownDependency(dep.clone()))?;

                // Cartesian product: all dependency variants -> all stage variants
                for &dep_idx in dep_indices {
                    for &stage_idx in &stage_indices {
                        graph.add_edge(dep_idx, stage_idx, ());
                    }
                }
            }
        }

        let dag = PipelineDag {
            graph,
            name_to_nodes,
        };

        // Verify no cycles
        dag.topological_order()?;

        Ok(dag)
    }
}

impl Default for DagBuilder {
    fn default() -> Self {
        Self::new()
    }
}

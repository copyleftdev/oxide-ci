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
    pub name_to_index: HashMap<String, NodeIndex>,
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
        self.name_to_index
            .get(stage_name)
            .map(|&idx| {
                self.graph
                    .neighbors_directed(idx, petgraph::Direction::Outgoing)
                    .filter_map(|n| self.graph.node_weight(n))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get stages that must complete before a given stage can run.
    pub fn predecessors(&self, stage_name: &str) -> Vec<&DagNode> {
        self.name_to_index
            .get(stage_name)
            .map(|&idx| {
                self.graph
                    .neighbors_directed(idx, petgraph::Direction::Incoming)
                    .filter_map(|n| self.graph.node_weight(n))
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
        let mut name_to_index = HashMap::new();

        // Add all stages as nodes
        for stage in &pipeline.stages {
            let node = DagNode {
                stage_id: StageId::new(&stage.name),
                name: stage.name.clone(),
                definition: stage.clone(),
            };
            let idx = graph.add_node(node);
            name_to_index.insert(stage.name.clone(), idx);
        }

        // Add edges for dependencies
        for stage in &pipeline.stages {
            let stage_idx = name_to_index[&stage.name];
            for dep in &stage.depends_on {
                let dep_idx = name_to_index
                    .get(dep)
                    .ok_or_else(|| DagError::UnknownDependency(dep.clone()))?;
                graph.add_edge(*dep_idx, stage_idx, ());
            }
        }

        let dag = PipelineDag {
            graph,
            name_to_index,
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

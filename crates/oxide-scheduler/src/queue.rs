//! Queue management for pending jobs.

use chrono::{DateTime, Utc};
use oxide_core::ids::{PipelineId, RunId};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

/// Priority for queue items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Priority {
    Low = 0,
    #[default]
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// A queued job waiting for execution.
#[derive(Debug, Clone)]
pub struct QueuedJob {
    pub run_id: RunId,
    pub pipeline_id: PipelineId,
    pub stage_name: String,
    pub job_index: Option<usize>,
    pub priority: Priority,
    pub queued_at: DateTime<Utc>,
    pub labels: Vec<String>,
    pub concurrency_group: Option<String>,
}

impl PartialEq for QueuedJob {
    fn eq(&self, other: &Self) -> bool {
        self.run_id == other.run_id && self.stage_name == other.stage_name
    }
}

impl Eq for QueuedJob {}

impl PartialOrd for QueuedJob {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueuedJob {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first, then earlier queued time
        match (self.priority as u8).cmp(&(other.priority as u8)) {
            Ordering::Equal => other.queued_at.cmp(&self.queued_at),
            other => other,
        }
    }
}

/// Queue manager for job scheduling.
pub struct QueueManager {
    queue: BinaryHeap<QueuedJob>,
    concurrency_groups: HashMap<String, usize>,
    concurrency_limits: HashMap<String, usize>,
    pipeline_rate_limits: HashMap<PipelineId, RateLimit>,
}

struct RateLimit {
    max_concurrent: usize,
    current: usize,
}

impl QueueManager {
    pub fn new() -> Self {
        Self {
            queue: BinaryHeap::new(),
            concurrency_groups: HashMap::new(),
            concurrency_limits: HashMap::new(),
            pipeline_rate_limits: HashMap::new(),
        }
    }

    /// Add a job to the queue.
    pub fn enqueue(&mut self, job: QueuedJob) {
        self.queue.push(job);
    }

    /// Get the next job that can be executed.
    pub fn dequeue(&mut self) -> Option<QueuedJob> {
        let mut temp = Vec::new();
        let mut result = None;

        while let Some(job) = self.queue.pop() {
            if self.can_execute(&job) {
                // Mark as running
                if let Some(ref group) = job.concurrency_group {
                    *self.concurrency_groups.entry(group.clone()).or_insert(0) += 1;
                }
                if let Some(limit) = self.pipeline_rate_limits.get_mut(&job.pipeline_id) {
                    limit.current += 1;
                }
                result = Some(job);
                break;
            } else {
                temp.push(job);
            }
        }

        // Put back jobs that couldn't be executed
        for job in temp {
            self.queue.push(job);
        }

        result
    }

    /// Mark a job as completed, freeing up concurrency slots.
    pub fn complete(&mut self, job: &QueuedJob) {
        if let Some(ref group) = job.concurrency_group
            && let Some(count) = self.concurrency_groups.get_mut(group)
        {
            *count = count.saturating_sub(1);
        }
        if let Some(limit) = self.pipeline_rate_limits.get_mut(&job.pipeline_id) {
            limit.current = limit.current.saturating_sub(1);
        }
    }

    /// Set the concurrency limit for a group.
    pub fn set_concurrency_limit(&mut self, group: String, limit: usize) {
        self.concurrency_limits.insert(group, limit);
    }

    /// Set the rate limit for a pipeline.
    pub fn set_pipeline_rate_limit(&mut self, pipeline_id: PipelineId, max_concurrent: usize) {
        self.pipeline_rate_limits.insert(
            pipeline_id,
            RateLimit {
                max_concurrent,
                current: 0,
            },
        );
    }

    /// Get the current queue length.
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Check if the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Get the position of a run in the queue.
    pub fn position(&self, run_id: RunId) -> Option<usize> {
        let sorted: Vec<_> = self.queue.iter().collect();
        sorted.iter().position(|j| j.run_id == run_id)
    }

    fn can_execute(&self, job: &QueuedJob) -> bool {
        // Check concurrency group
        if let Some(ref group) = job.concurrency_group {
            let current = self.concurrency_groups.get(group).copied().unwrap_or(0);
            let limit = self.concurrency_limits.get(group).copied().unwrap_or(1);
            if current >= limit {
                return false;
            }
        }

        // Check pipeline rate limit
        if let Some(limit) = self.pipeline_rate_limits.get(&job.pipeline_id)
            && limit.current >= limit.max_concurrent
        {
            return false;
        }

        true
    }
}

impl Default for QueueManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_ordering() {
        let mut queue = QueueManager::new();
        let now = Utc::now();

        queue.enqueue(QueuedJob {
            run_id: RunId::default(),
            pipeline_id: PipelineId::default(),
            stage_name: "low".to_string(),
            job_index: None,
            priority: Priority::Low,
            queued_at: now,
            labels: vec![],
            concurrency_group: None,
        });

        queue.enqueue(QueuedJob {
            run_id: RunId::default(),
            pipeline_id: PipelineId::default(),
            stage_name: "high".to_string(),
            job_index: None,
            priority: Priority::High,
            queued_at: now,
            labels: vec![],
            concurrency_group: None,
        });

        let first = queue.dequeue().unwrap();
        assert_eq!(first.stage_name, "high");
    }

    #[test]
    fn test_concurrency_limit() {
        let mut queue = QueueManager::new();
        let now = Utc::now();

        queue.set_concurrency_limit("deploy".to_string(), 1);

        let job1 = QueuedJob {
            run_id: RunId::default(),
            pipeline_id: PipelineId::default(),
            stage_name: "deploy-1".to_string(),
            job_index: None,
            priority: Priority::Normal,
            queued_at: now,
            labels: vec![],
            concurrency_group: Some("deploy".to_string()),
        };

        let job2 = QueuedJob {
            run_id: RunId::default(),
            pipeline_id: PipelineId::default(),
            stage_name: "deploy-2".to_string(),
            job_index: None,
            priority: Priority::Normal,
            queued_at: now,
            labels: vec![],
            concurrency_group: Some("deploy".to_string()),
        };

        queue.enqueue(job1.clone());
        queue.enqueue(job2);

        // First job should be dequeued
        let first = queue.dequeue().unwrap();
        assert_eq!(first.stage_name, "deploy-1");

        // Second should be blocked
        assert!(queue.dequeue().is_none());

        // Complete first job
        queue.complete(&first);

        // Now second can run
        let second = queue.dequeue().unwrap();
        assert_eq!(second.stage_name, "deploy-2");
    }
}

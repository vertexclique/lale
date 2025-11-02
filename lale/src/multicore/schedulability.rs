//! Multi-core schedulability analysis for actor systems

use crate::async_analysis::{Actor, SchedulingPolicy};
use crate::scheduling::{EDFScheduler, RMAScheduler, SchedulabilityResult, Task};
use serde::{Deserialize, Serialize};

/// Multi-core scheduler
pub struct MultiCoreScheduler {
    pub num_cores: usize,
    pub policy: SchedulingPolicy,
}

/// Multi-core schedulability result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiCoreResult {
    /// Per-core results
    pub per_core: Vec<CoreSchedulabilityResult>,

    /// Overall schedulability
    pub overall_schedulable: bool,

    /// Total system utilization
    pub total_utilization: f64,

    /// Per-core utilization
    pub core_utilizations: Vec<f64>,
}

/// Per-core schedulability result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreSchedulabilityResult {
    pub core_id: usize,
    pub schedulable: bool,
    pub utilization: f64,
    pub actors: Vec<String>,
    pub violations: Vec<DeadlineViolation>,
}

/// Deadline violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadlineViolation {
    pub actor_name: String,
    pub response_time_us: f64,
    pub deadline_us: f64,
    pub slack_us: f64,
}

impl MultiCoreScheduler {
    /// Create new multi-core scheduler
    pub fn new(num_cores: usize, policy: SchedulingPolicy) -> Self {
        Self { num_cores, policy }
    }

    /// Analyze schedulability for actor system
    pub fn analyze(&self, actors: &[Actor]) -> MultiCoreResult {
        // Partition actors by core affinity
        let partitions = self.partition_actors(actors);

        // Analyze each core independently
        let mut per_core = Vec::new();
        let mut overall_schedulable = true;
        let mut core_utilizations = Vec::new();

        for core_id in 0..self.num_cores {
            let core_actors = partitions.get(&core_id).cloned().unwrap_or_default();

            let result = self.analyze_core(core_id, &core_actors);

            overall_schedulable &= result.schedulable;
            core_utilizations.push(result.utilization);
            per_core.push(result);
        }

        let total_utilization = actors.iter().map(|a| a.utilization()).sum();

        MultiCoreResult {
            per_core,
            overall_schedulable,
            total_utilization,
            core_utilizations,
        }
    }

    /// Partition actors by core affinity
    fn partition_actors<'a>(&self, actors: &'a [Actor]) -> ahash::AHashMap<usize, Vec<&'a Actor>> {
        let mut partitions = ahash::AHashMap::new();

        for actor in actors {
            let core = actor.core_affinity.unwrap_or(0);
            partitions.entry(core).or_insert_with(Vec::new).push(actor);
        }

        partitions
    }

    /// Analyze single core
    fn analyze_core(&self, core_id: usize, actors: &[&Actor]) -> CoreSchedulabilityResult {
        if actors.is_empty() {
            return CoreSchedulabilityResult {
                core_id,
                schedulable: true,
                utilization: 0.0,
                actors: vec![],
                violations: vec![],
            };
        }

        // Convert actors to tasks
        let tasks: Vec<_> = actors.iter().map(|a| a.to_task()).collect();

        // Perform schedulability analysis
        let result = match self.policy {
            SchedulingPolicy::RMA => RMAScheduler::schedulability_test(&tasks),
            SchedulingPolicy::EDF => EDFScheduler::schedulability_test(&tasks),
        };

        // Check if schedulable and extract violations
        let (schedulable, violations) = match result {
            SchedulabilityResult::Schedulable => (true, vec![]),
            SchedulabilityResult::Unschedulable {
                failing_task,
                response_time,
                deadline,
            } => {
                let violation = DeadlineViolation {
                    actor_name: failing_task,
                    response_time_us: response_time,
                    deadline_us: deadline,
                    slack_us: deadline - response_time,
                };
                (false, vec![violation])
            }
        };

        let utilization = actors.iter().map(|a| a.utilization()).sum();

        CoreSchedulabilityResult {
            core_id,
            schedulable,
            utilization,
            actors: actors.iter().map(|a| a.name.clone()).collect(),
            violations,
        }
    }
}

impl MultiCoreResult {
    /// Check if system is schedulable
    pub fn is_schedulable(&self) -> bool {
        self.overall_schedulable
    }

    /// Get all deadline violations
    pub fn violations(&self) -> Vec<&DeadlineViolation> {
        self.per_core
            .iter()
            .flat_map(|core| core.violations.iter())
            .collect()
    }

    /// Export to JSON
    pub fn export_json(&self, path: &str) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize: {}", e))?;

        std::fs::write(path, json).map_err(|e| format!("Failed to write file: {}", e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multicore_scheduler_creation() {
        let scheduler = MultiCoreScheduler::new(2, SchedulingPolicy::RMA);
        assert_eq!(scheduler.num_cores, 2);
    }

    #[test]
    fn test_empty_analysis() {
        let scheduler = MultiCoreScheduler::new(2, SchedulingPolicy::RMA);
        let result = scheduler.analyze(&[]);

        assert!(result.is_schedulable());
        assert_eq!(result.total_utilization, 0.0);
        assert_eq!(result.per_core.len(), 2);
    }
}

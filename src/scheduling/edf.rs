use crate::scheduling::{rma::SchedulabilityResult, Task};

/// Earliest Deadline First scheduler
pub struct EDFScheduler;

impl EDFScheduler {
    /// Perform EDF schedulability test
    pub fn schedulability_test(tasks: &[Task]) -> SchedulabilityResult {
        // Filter tasks with periods
        let periodic_tasks: Vec<_> = tasks.iter().filter(|t| t.period_us.is_some()).collect();

        if periodic_tasks.is_empty() {
            return SchedulabilityResult::Schedulable;
        }

        // EDF schedulability: U ≤ 1.0
        let total_utilization: f64 = periodic_tasks
            .iter()
            .map(|t| t.wcet_us / t.period_us.unwrap())
            .sum();

        if total_utilization <= 1.0 {
            SchedulabilityResult::Schedulable
        } else {
            // Find which task would miss deadline
            // In EDF, all tasks fail together when U > 1
            SchedulabilityResult::Unschedulable {
                failing_task: "system".to_string(),
                response_time: 0.0,
                deadline: 0.0,
            }
        }
    }

    /// Calculate system utilization
    pub fn calculate_utilization(tasks: &[Task]) -> f64 {
        tasks
            .iter()
            .filter(|t| t.period_us.is_some())
            .map(|t| t.wcet_us / t.period_us.unwrap())
            .sum()
    }

    /// Assign dynamic priorities based on absolute deadlines
    /// Returns task instances with their absolute deadlines
    pub fn generate_task_instances(tasks: &[Task], hyperperiod: f64) -> Vec<TaskInstance> {
        let mut instances = Vec::new();

        for task in tasks {
            if let Some(period) = task.period_us {
                let num_instances = (hyperperiod / period).ceil() as usize;

                for i in 0..num_instances {
                    let release_time = i as f64 * period;
                    let deadline = task.deadline_us.unwrap_or(period);
                    let absolute_deadline = release_time + deadline;

                    instances.push(TaskInstance {
                        task_name: task.name.clone(),
                        release_time,
                        absolute_deadline,
                        wcet_us: task.wcet_us,
                        remaining_time: task.wcet_us,
                    });
                }
            }
        }

        // Sort by absolute deadline (EDF order)
        instances.sort_by(|a, b| {
            a.absolute_deadline
                .partial_cmp(&b.absolute_deadline)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        instances
    }
}

/// Task instance for EDF scheduling
#[derive(Debug, Clone)]
pub struct TaskInstance {
    pub task_name: String,
    pub release_time: f64,
    pub absolute_deadline: f64,
    pub wcet_us: f64,
    pub remaining_time: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edf_schedulable() {
        let tasks = vec![
            Task {
                name: "task1".to_string(),
                function: "func1".to_string(),
                wcet_cycles: 3000,
                wcet_us: 300.0,
                period_us: Some(1000.0),
                deadline_us: Some(1000.0),
                priority: None,
                preemptible: true,
                dependencies: vec![],
            },
            Task {
                name: "task2".to_string(),
                function: "func2".to_string(),
                wcet_cycles: 4000,
                wcet_us: 400.0,
                period_us: Some(2000.0),
                deadline_us: Some(2000.0),
                priority: None,
                preemptible: true,
                dependencies: vec![],
            },
        ];

        let result = EDFScheduler::schedulability_test(&tasks);
        assert_eq!(result, SchedulabilityResult::Schedulable);

        let utilization = EDFScheduler::calculate_utilization(&tasks);
        assert!((utilization - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_edf_unschedulable() {
        let tasks = vec![
            Task {
                name: "task1".to_string(),
                function: "func1".to_string(),
                wcet_cycles: 9000,
                wcet_us: 900.0,
                period_us: Some(1000.0),
                deadline_us: Some(1000.0),
                priority: None,
                preemptible: true,
                dependencies: vec![],
            },
            Task {
                name: "task2".to_string(),
                function: "func2".to_string(),
                wcet_cycles: 3000,
                wcet_us: 300.0,
                period_us: Some(2000.0),
                deadline_us: Some(2000.0),
                priority: None,
                preemptible: true,
                dependencies: vec![],
            },
        ];

        let result = EDFScheduler::schedulability_test(&tasks);
        assert!(matches!(result, SchedulabilityResult::Unschedulable { .. }));
    }

    #[test]
    fn test_task_instance_generation() {
        let tasks = vec![Task {
            name: "task1".to_string(),
            function: "func1".to_string(),
            wcet_cycles: 1000,
            wcet_us: 100.0,
            period_us: Some(1000.0),
            deadline_us: Some(1000.0),
            priority: None,
            preemptible: true,
            dependencies: vec![],
        }];

        let instances = EDFScheduler::generate_task_instances(&tasks, 3000.0);
        assert_eq!(instances.len(), 3);

        // Check deadlines are in order
        for i in 1..instances.len() {
            assert!(instances[i - 1].absolute_deadline <= instances[i].absolute_deadline);
        }
    }
}

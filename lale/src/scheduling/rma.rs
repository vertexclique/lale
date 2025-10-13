use crate::scheduling::Task;

/// Rate Monotonic Analysis result
#[derive(Debug, Clone, PartialEq)]
pub enum SchedulabilityResult {
    Schedulable,
    Unschedulable {
        failing_task: String,
        response_time: f64,
        deadline: f64,
    },
}

/// Rate Monotonic Analysis scheduler
pub struct RMAScheduler;

impl RMAScheduler {
    /// Perform RMA schedulability test
    pub fn schedulability_test(tasks: &[Task]) -> SchedulabilityResult {
        // Filter tasks with periods
        let mut periodic_tasks: Vec<_> = tasks.iter().filter(|t| t.period_us.is_some()).collect();

        if periodic_tasks.is_empty() {
            return SchedulabilityResult::Schedulable;
        }

        // Sort by period (shorter period = higher priority)
        periodic_tasks.sort_by(|a, b| {
            a.period_us
                .partial_cmp(&b.period_us)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Liu & Layland utilization bound test
        let n = periodic_tasks.len() as f64;
        let utilization_bound = n * (2.0_f64.powf(1.0 / n) - 1.0);

        let total_utilization: f64 = periodic_tasks
            .iter()
            .map(|t| t.wcet_us / t.period_us.unwrap())
            .sum();

        // Quick test: if utilization is below bound, definitely schedulable
        if total_utilization <= utilization_bound {
            return SchedulabilityResult::Schedulable;
        }

        // Exact response time analysis
        for (i, task) in periodic_tasks.iter().enumerate() {
            let response_time = Self::calculate_response_time(task, &periodic_tasks[..i]);
            let deadline = task.deadline_us.unwrap_or(task.period_us.unwrap());

            if response_time > deadline {
                return SchedulabilityResult::Unschedulable {
                    failing_task: task.name.clone(),
                    response_time,
                    deadline,
                };
            }
        }

        SchedulabilityResult::Schedulable
    }

    /// Calculate response time for a task
    fn calculate_response_time(task: &Task, higher_priority: &[&Task]) -> f64 {
        let mut r = task.wcet_us;
        let max_iterations = 100;

        for _ in 0..max_iterations {
            let interference: f64 = higher_priority
                .iter()
                .map(|hp| {
                    let period = hp.period_us.unwrap();
                    (r / period).ceil() * hp.wcet_us
                })
                .sum();

            let new_r = task.wcet_us + interference;

            // Check convergence
            if (new_r - r).abs() < 0.001 {
                return new_r;
            }

            // Check if already failed
            let deadline = task.deadline_us.unwrap_or(task.period_us.unwrap());
            if new_r > deadline {
                return new_r;
            }

            r = new_r;
        }

        r
    }

    /// Assign priorities based on RMA (shorter period = higher priority)
    pub fn assign_priorities(tasks: &mut [Task]) {
        // Sort by period
        tasks.sort_by(|a, b| {
            let period_a = a.period_us.unwrap_or(f64::MAX);
            let period_b = b.period_us.unwrap_or(f64::MAX);
            period_a
                .partial_cmp(&period_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Assign priorities (0 = highest)
        for (i, task) in tasks.iter_mut().enumerate() {
            task.priority = Some(i as u8);
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rma_schedulable() {
        let tasks = vec![
            Task {
                name: "task1".to_string(),
                function: "func1".to_string(),
                wcet_cycles: 1000,
                wcet_us: 100.0,
                period_us: Some(1000.0),
                deadline_us: Some(1000.0),
                priority: None,
                preemptible: true,
                dependencies: vec![],
            },
            Task {
                name: "task2".to_string(),
                function: "func2".to_string(),
                wcet_cycles: 2000,
                wcet_us: 200.0,
                period_us: Some(2000.0),
                deadline_us: Some(2000.0),
                priority: None,
                preemptible: true,
                dependencies: vec![],
            },
        ];

        let result = RMAScheduler::schedulability_test(&tasks);
        assert_eq!(result, SchedulabilityResult::Schedulable);

        let utilization = RMAScheduler::calculate_utilization(&tasks);
        assert!((utilization - 0.2).abs() < 0.001);
    }

    #[test]
    fn test_rma_unschedulable() {
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
                wcet_cycles: 1800,
                wcet_us: 900.0,
                period_us: Some(2000.0),
                deadline_us: Some(2000.0),
                priority: None,
                preemptible: true,
                dependencies: vec![],
            },
        ];

        let result = RMAScheduler::schedulability_test(&tasks);
        assert!(matches!(result, SchedulabilityResult::Unschedulable { .. }));
    }
}

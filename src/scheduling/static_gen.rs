use crate::scheduling::Task;
use serde::{Deserialize, Serialize};

/// Time slot in static schedule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSlot {
    pub start_us: f64,
    pub duration_us: f64,
    pub task: String,
    pub preemptible: bool,
}

/// Static schedule timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleTimeline {
    pub hyperperiod_us: f64,
    pub slots: Vec<TimeSlot>,
}

/// Static schedule generator
pub struct StaticScheduleGenerator;

impl StaticScheduleGenerator {
    /// Generate static schedule for time-triggered architecture
    pub fn generate_schedule(tasks: &[Task]) -> ScheduleTimeline {
        // Calculate hyperperiod (LCM of all periods)
        let periods: Vec<u64> = tasks
            .iter()
            .filter_map(|t| t.period_us.map(|p| p as u64))
            .collect();

        let hyperperiod = if periods.is_empty() {
            10000.0 // Default 10ms
        } else {
            Self::lcm_of_list(&periods) as f64
        };

        // Generate all task instances within hyperperiod
        let mut instances = Vec::new();
        for task in tasks {
            if let Some(period) = task.period_us {
                let num_instances = (hyperperiod / period) as usize;
                for i in 0..num_instances {
                    instances.push(TaskInstance {
                        task: task.clone(),
                        release_time: i as f64 * period,
                        absolute_deadline: i as f64 * period + task.deadline_us.unwrap_or(period),
                    });
                }
            }
        }

        // Sort by deadline (EDF order for static schedule)
        instances.sort_by(|a, b| {
            a.absolute_deadline
                .partial_cmp(&b.absolute_deadline)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Allocate time slots
        let mut slots = Vec::new();
        let mut current_time = 0.0;

        for instance in instances {
            // Add idle slot if needed
            if current_time < instance.release_time {
                slots.push(TimeSlot {
                    start_us: current_time,
                    duration_us: instance.release_time - current_time,
                    task: "IDLE".to_string(),
                    preemptible: true,
                });
                current_time = instance.release_time;
            }

            // Add task execution slot
            slots.push(TimeSlot {
                start_us: current_time,
                duration_us: instance.task.wcet_us,
                task: instance.task.name.clone(),
                preemptible: instance.task.preemptible,
            });

            current_time += instance.task.wcet_us;
        }

        // Fill remaining time with idle
        if current_time < hyperperiod {
            slots.push(TimeSlot {
                start_us: current_time,
                duration_us: hyperperiod - current_time,
                task: "IDLE".to_string(),
                preemptible: true,
            });
        }

        ScheduleTimeline {
            hyperperiod_us: hyperperiod,
            slots,
        }
    }

    /// Calculate LCM of a list of numbers
    fn lcm_of_list(numbers: &[u64]) -> u64 {
        if numbers.is_empty() {
            return 1;
        }

        numbers.iter().fold(numbers[0], |acc, &x| Self::lcm(acc, x))
    }

    /// Calculate LCM of two numbers
    fn lcm(a: u64, b: u64) -> u64 {
        (a * b) / Self::gcd(a, b)
    }

    /// Calculate GCD of two numbers
    fn gcd(mut a: u64, mut b: u64) -> u64 {
        while b != 0 {
            let temp = b;
            b = a % b;
            a = temp;
        }
        a
    }
}

/// Task instance for scheduling
#[derive(Debug, Clone)]
struct TaskInstance {
    task: Task,
    release_time: f64,
    absolute_deadline: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gcd_lcm() {
        assert_eq!(StaticScheduleGenerator::gcd(12, 8), 4);
        assert_eq!(StaticScheduleGenerator::lcm(12, 8), 24);

        let numbers = vec![10, 15, 20];
        assert_eq!(StaticScheduleGenerator::lcm_of_list(&numbers), 60);
    }

    #[test]
    fn test_static_schedule_generation() {
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

        let schedule = StaticScheduleGenerator::generate_schedule(&tasks);

        assert_eq!(schedule.hyperperiod_us, 2000.0);
        assert!(!schedule.slots.is_empty());

        // Verify schedule covers entire hyperperiod
        let total_time: f64 = schedule.slots.iter().map(|s| s.duration_us).sum();
        assert!((total_time - schedule.hyperperiod_us).abs() < 0.001);
    }
}

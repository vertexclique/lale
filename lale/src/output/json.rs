use crate::scheduling::rma::SchedulabilityResult;
use crate::scheduling::{static_gen::ScheduleTimeline, Task};
use serde::{Deserialize, Serialize};
use ahash::AHashMap;

/// Complete analysis report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisReport {
    pub analysis_info: AnalysisInfo,
    pub wcet_analysis: WCETAnalysis,
    pub task_model: TaskModel,
    pub schedulability: SchedulabilityAnalysis,
    pub schedule: Option<ScheduleTimeline>,
}

/// Analysis metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisInfo {
    pub tool: String,
    pub version: String,
    pub timestamp: String,
    pub platform: String,
}

/// WCET analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WCETAnalysis {
    pub functions: Vec<FunctionWCET>,
}

/// WCET for a single function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionWCET {
    pub name: String,
    pub llvm_name: String,
    pub wcet_cycles: u64,
    pub wcet_us: f64,
    pub bcet_cycles: u64,
    pub bcet_us: f64,
    pub loop_count: usize,
}

/// Task model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskModel {
    pub tasks: Vec<Task>,
}

/// Schedulability analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulabilityAnalysis {
    pub method: String,
    pub result: String,
    pub utilization: f64,
    pub utilization_bound: Option<f64>,
    pub response_times: AHashMap<String, f64>,
}

/// JSON output generator
pub struct JSONOutput;

impl JSONOutput {
    /// Generate complete analysis report
    pub fn generate_report(
        wcet_results: &AHashMap<String, u64>,
        tasks: &[Task],
        schedulability: &SchedulabilityResult,
        schedule: Option<ScheduleTimeline>,
        platform_name: &str,
        cpu_freq_mhz: u32,
    ) -> AnalysisReport {
        let analysis_info = AnalysisInfo {
            tool: "LALE".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            platform: platform_name.to_string(),
        };

        let functions: Vec<FunctionWCET> = wcet_results
            .iter()
            .map(|(name, &wcet_cycles)| {
                let wcet_us = wcet_cycles as f64 / cpu_freq_mhz as f64;
                FunctionWCET {
                    name: name.clone(),
                    llvm_name: format!("@{}", name),
                    wcet_cycles,
                    wcet_us,
                    bcet_cycles: wcet_cycles / 2, // Simplified
                    bcet_us: wcet_us / 2.0,
                    loop_count: 0, // Would need loop analysis results
                }
            })
            .collect();

        let wcet_analysis = WCETAnalysis { functions };

        let task_model = TaskModel {
            tasks: tasks.to_vec(),
        };

        let (result_str, utilization, utilization_bound) = match schedulability {
            SchedulabilityResult::Schedulable => {
                let util: f64 = tasks
                    .iter()
                    .filter(|t| t.period_us.is_some())
                    .map(|t| t.wcet_us / t.period_us.unwrap())
                    .sum();
                ("schedulable".to_string(), util, Some(1.0))
            }
            SchedulabilityResult::Unschedulable { .. } => {
                let util: f64 = tasks
                    .iter()
                    .filter(|t| t.period_us.is_some())
                    .map(|t| t.wcet_us / t.period_us.unwrap())
                    .sum();
                ("unschedulable".to_string(), util, Some(1.0))
            }
        };

        let response_times: AHashMap<String, f64> =
            tasks.iter().map(|t| (t.name.clone(), t.wcet_us)).collect();

        let schedulability_analysis = SchedulabilityAnalysis {
            method: "RMA".to_string(),
            result: result_str,
            utilization,
            utilization_bound,
            response_times,
        };

        AnalysisReport {
            analysis_info,
            wcet_analysis,
            task_model,
            schedulability: schedulability_analysis,
            schedule,
        }
    }

    /// Export report to JSON string
    pub fn to_json(report: &AnalysisReport) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(report)
    }

    /// Export report to JSON file
    pub fn to_file(report: &AnalysisReport, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = Self::to_json(report)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_generation() {
        let mut wcet_results = AHashMap::new();
        wcet_results.insert("test_func".to_string(), 1000);

        let tasks = vec![Task {
            name: "task1".to_string(),
            function: "func1".to_string(),
            wcet_cycles: 1000,
            wcet_us: 100.0,
            period_us: Some(1000.0),
            deadline_us: Some(1000.0),
            priority: Some(0),
            preemptible: true,
            dependencies: vec![],
        }];

        let schedulability = SchedulabilityResult::Schedulable;

        let report = JSONOutput::generate_report(
            &wcet_results,
            &tasks,
            &schedulability,
            None,
            "ARM Cortex-M4",
            168,
        );

        let json = JSONOutput::to_json(&report).unwrap();
        assert!(json.contains("LALE"));
        assert!(json.contains("task1"));
        assert!(json.contains("schedulable"));
    }
}

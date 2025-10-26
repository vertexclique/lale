use ahash::AHashMap;
use llvm_ir::Module;
use serde::{Deserialize, Serialize};

/// Real-time task model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub name: String,
    pub function: String,
    pub wcet_cycles: u64,
    pub wcet_us: f64,
    pub period_us: Option<f64>,
    pub deadline_us: Option<f64>,
    pub priority: Option<u8>,
    pub preemptible: bool,
    pub dependencies: Vec<String>,
}

/// Task attributes from annotations
#[derive(Debug, Clone)]
pub struct TaskAttributes {
    pub name: String,
    pub period: Option<f64>,
    pub deadline: Option<f64>,
    pub priority: Option<u8>,
    pub preemptible: Option<bool>,
}

/// Task extractor
pub struct TaskExtractor;

impl TaskExtractor {
    /// Extract tasks from LLVM IR module with WCET results
    pub fn extract_tasks(
        module: &Module,
        wcet_results: &AHashMap<String, u64>,
        cpu_freq_mhz: u32,
    ) -> Vec<Task> {
        let mut tasks = Vec::new();

        for function in &module.functions {
            // Check if function has task metadata
            if let Some(task_attr) = Self::parse_task_metadata(function) {
                let function_name = function.name.to_string();
                let wcet_cycles = wcet_results.get(&function_name).copied().unwrap_or(0);
                let wcet_us = Self::cycles_to_us(wcet_cycles, cpu_freq_mhz);

                tasks.push(Task {
                    name: task_attr.name,
                    function: function_name.clone(),
                    wcet_cycles,
                    wcet_us,
                    period_us: task_attr.period,
                    deadline_us: task_attr.deadline,
                    priority: task_attr.priority,
                    preemptible: task_attr.preemptible.unwrap_or(true),
                    dependencies: Self::extract_dependencies(function, module),
                });
            }
        }

        tasks
    }

    /// Parse task metadata from function
    fn parse_task_metadata(function: &llvm_ir::Function) -> Option<TaskAttributes> {
        // Look for task metadata in function metadata
        // Format: !task !{ !"name", !"task_name", !"period_us", i64 10000, ... }

        // For now, detect async functions or functions with specific naming patterns
        let func_name = function.name.to_string();

        if func_name.contains("task") || func_name.contains("async") {
            Some(TaskAttributes {
                name: func_name.clone(),
                period: Some(10000.0), // Default 10ms
                deadline: Some(10000.0),
                priority: Some(10),
                preemptible: Some(true),
            })
        } else {
            None
        }
    }

    /// Extract task dependencies from function calls
    fn extract_dependencies(function: &llvm_ir::Function, module: &Module) -> Vec<String> {
        let mut deps = Vec::new();

        for bb in &function.basic_blocks {
            for instr in &bb.instrs {
                if let llvm_ir::Instruction::Call(call) = instr {
                    let callee_name = format!("{:?}", call.function)
                        .split_whitespace()
                        .last()
                        .unwrap_or("")
                        .trim_matches(|c| c == '@' || c == '"')
                        .to_string();

                    // Check if callee is also a task
                    if let Some(callee_func) =
                        module.functions.iter().find(|f| f.name == callee_name)
                    {
                        if Self::parse_task_metadata(callee_func).is_some() {
                            deps.push(callee_name);
                        }
                    }
                }
            }
        }

        deps
    }

    /// Convert cycles to microseconds
    fn cycles_to_us(cycles: u64, cpu_freq_mhz: u32) -> f64 {
        cycles as f64 / cpu_freq_mhz as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::parser::IRParser;

    #[test]
    fn test_task_extraction() {
        let sample_path = "data/armv7e-m/56e3741adeae4068.ll";
        if std::path::Path::new(sample_path).exists() {
            let module = IRParser::parse_file(sample_path).unwrap();
            let mut wcet_results = AHashMap::new();

            // Mock WCET results
            for function in &module.functions {
                wcet_results.insert(function.name.to_string(), 1000);
            }

            let tasks = TaskExtractor::extract_tasks(&module, &wcet_results, 168);

            println!("Extracted {} tasks", tasks.len());
            for task in &tasks {
                println!("Task: {} (WCET: {} us)", task.name, task.wcet_us);
            }
        }
    }
}

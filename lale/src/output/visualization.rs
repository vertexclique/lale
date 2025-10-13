use crate::ir::CFG;
use crate::scheduling::static_gen::ScheduleTimeline;
use petgraph::graph::NodeIndex;
use ahash::AHashMap;

/// Graphviz DOT format generator
pub struct GraphvizOutput;

impl GraphvizOutput {
    /// Export CFG to Graphviz DOT format
    pub fn export_cfg(cfg: &CFG, timings: &AHashMap<NodeIndex, u32>) -> String {
        let mut dot = String::from("digraph CFG {\n");
        dot.push_str("  node [shape=box];\n");
        dot.push_str("  rankdir=TB;\n");

        // Add nodes
        for node_idx in cfg.graph.node_indices() {
            let block = &cfg.graph[node_idx];
            let timing = timings.get(&node_idx).copied().unwrap_or(0);

            let label = format!(
                "{}\\n{} cycles\\n{} instrs",
                block.label,
                timing,
                block.instructions.len()
            );

            dot.push_str(&format!("  n{} [label=\"{}\"];\n", node_idx.index(), label));
        }

        // Add edges
        for edge in cfg.graph.edge_references() {
            use petgraph::visit::EdgeRef;
            let source = edge.source();
            let target = edge.target();
            let edge_type = edge.weight();

            let style = match edge_type {
                crate::ir::cfg::EdgeType::ConditionalTrue => "color=green",
                crate::ir::cfg::EdgeType::ConditionalFalse => "color=red",
                crate::ir::cfg::EdgeType::LoopBack => "color=blue, style=dashed",
                _ => "color=black",
            };

            dot.push_str(&format!(
                "  n{} -> n{} [{}];\n",
                source.index(),
                target.index(),
                style
            ));
        }

        dot.push_str("}\n");
        dot
    }

    /// Export CFG to file
    pub fn export_cfg_to_file(
        cfg: &CFG,
        timings: &AHashMap<NodeIndex, u32>,
        path: &str,
    ) -> Result<(), std::io::Error> {
        let dot = Self::export_cfg(cfg, timings);
        std::fs::write(path, dot)
    }
}

/// Gantt chart data generator
pub struct GanttOutput;

impl GanttOutput {
    /// Generate Gantt chart data from schedule
    pub fn generate_gantt_data(schedule: &ScheduleTimeline) -> GanttData {
        let mut task_instances: AHashMap<String, Vec<TaskExecution>> = AHashMap::new();

        for slot in &schedule.slots {
            if slot.task != "IDLE" {
                task_instances
                    .entry(slot.task.clone())
                    .or_insert_with(Vec::new)
                    .push(TaskExecution {
                        start: slot.start_us,
                        end: slot.start_us + slot.duration_us,
                        execution_type: if slot.preemptible {
                            "execution".to_string()
                        } else {
                            "critical".to_string()
                        },
                    });
            }
        }

        GanttData {
            time_unit: "us".to_string(),
            hyperperiod: schedule.hyperperiod_us,
            tasks: task_instances,
        }
    }

    /// Export Gantt data to JSON
    pub fn to_json(data: &GanttData) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(data)
    }

    /// Export Gantt data to file
    pub fn to_file(data: &GanttData, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = Self::to_json(data)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

/// Gantt chart data structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GanttData {
    pub time_unit: String,
    pub hyperperiod: f64,
    pub tasks: AHashMap<String, Vec<TaskExecution>>,
}

/// Task execution instance
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskExecution {
    pub start: f64,
    pub end: f64,
    pub execution_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::parser::IRParser;
    use crate::ir::CFG;

    #[test]
    fn test_graphviz_export() {
        let sample_path = "data/armv7e-m/56e3741adeae4068.ll";
        if std::path::Path::new(sample_path).exists() {
            let module = IRParser::parse_file(sample_path).unwrap();

            if let Some(function) = module.functions.first() {
                let cfg = CFG::from_function(function);
                let mut timings = AHashMap::new();

                for node in cfg.graph.node_indices() {
                    timings.insert(node, 100);
                }

                let dot = GraphvizOutput::export_cfg(&cfg, &timings);

                assert!(dot.contains("digraph CFG"));
                assert!(dot.contains("node [shape=box]"));
            }
        }
    }

    #[test]
    fn test_gantt_generation() {
        use crate::scheduling::static_gen::TimeSlot;

        let schedule = ScheduleTimeline {
            hyperperiod_us: 2000.0,
            slots: vec![
                TimeSlot {
                    start_us: 0.0,
                    duration_us: 100.0,
                    task: "task1".to_string(),
                    preemptible: true,
                },
                TimeSlot {
                    start_us: 100.0,
                    duration_us: 200.0,
                    task: "task2".to_string(),
                    preemptible: false,
                },
            ],
        };

        let gantt = GanttOutput::generate_gantt_data(&schedule);

        assert_eq!(gantt.time_unit, "us");
        assert_eq!(gantt.hyperperiod, 2000.0);
        assert_eq!(gantt.tasks.len(), 2);

        let json = GanttOutput::to_json(&gantt).unwrap();
        assert!(json.contains("task1"));
        assert!(json.contains("task2"));
    }
}

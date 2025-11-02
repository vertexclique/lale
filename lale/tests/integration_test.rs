use ahash::AHashMap;
use lale::analysis::LoopBounds;
use lale::*;
use petgraph::visit::EdgeRef;

/// Test complete WCET analysis pipeline
#[test]
fn test_wcet_analysis_pipeline() {
    let sample_path = "ral/futures_util-d2cc80ffae74ea83.ll";
    if !std::path::Path::new(sample_path).exists() {
        eprintln!("Skipping test: {} not found", sample_path);
        return;
    }

    // Parse IR
    let module = IRParser::parse_file(sample_path).expect("Failed to parse IR");
    assert!(
        !module.functions.is_empty(),
        "Module should contain functions"
    );

    // Create platform model
    let platform = CortexM4Model::new();
    assert_eq!(platform.name, "ARM Cortex-M4");
    assert_eq!(platform.cpu_frequency_mhz, 168);

    // Analyze first function
    let function = module
        .functions
        .first()
        .expect("Should have at least one function");

    // Build CFG
    let cfg = CFG::from_function(function);
    assert!(cfg.block_count() > 0, "CFG should have basic blocks");
    assert!(cfg.exits.len() > 0, "CFG should have exit blocks");

    // Analyze loops
    let loops = LoopAnalyzer::analyze_loops(&cfg);
    // Loops may or may not exist, just verify it doesn't crash

    // Calculate timings
    let timings = TimingCalculator::calculate_block_timings(function, &cfg, &platform);
    assert!(!timings.is_empty(), "Should calculate timings for blocks");

    // Verify all timings are non-negative
    for (_, cycles) in &timings {
        assert!(
            cycles.worst_case >= cycles.best_case,
            "Worst case should be >= best case"
        );
        assert!(cycles.best_case > 0, "Best case should be positive");
    }

    // Calculate WCET with IPET
    let wcet_result = IPETSolver::solve_wcet(&cfg, &timings, &loops);

    // WCET should either succeed or fail gracefully
    match wcet_result {
        Ok(wcet) => {
            assert!(wcet > 0, "WCET should be positive");
            println!("WCET calculated: {} cycles", wcet);
        }
        Err(e) => {
            println!(
                "WCET calculation failed (expected for some functions): {}",
                e
            );
        }
    }
}

/// Test CFG construction correctness
#[test]
fn test_cfg_correctness() {
    let sample_path = "ral/futures_util-d2cc80ffae74ea83.ll";
    if !std::path::Path::new(sample_path).exists() {
        eprintln!("Skipping test: {} not found", sample_path);
        return;
    }

    let module = IRParser::parse_file(sample_path).unwrap();

    for function in &module.functions {
        let cfg = CFG::from_function(function);

        // Verify CFG properties
        assert!(cfg.block_count() > 0, "CFG must have at least one block");
        assert!(cfg.exits.len() > 0, "CFG must have at least one exit");

        // Entry block should be in node indices
        let has_entry = cfg.graph.node_indices().any(|n| n == cfg.entry);
        assert!(has_entry, "Entry block must exist in graph");

        // All exit blocks should be in graph
        for &exit in &cfg.exits {
            let has_exit = cfg.graph.node_indices().any(|n| n == exit);
            assert!(has_exit, "Exit block must exist in graph");
        }

        // Verify no self-loops on entry
        let self_loops: Vec<_> = cfg
            .graph
            .edges_directed(cfg.entry, petgraph::Direction::Outgoing)
            .filter(|e| e.target() == cfg.entry)
            .collect();
        assert!(self_loops.is_empty(), "Entry should not have self-loop");
    }
}

/// Test loop detection correctness
#[test]
fn test_loop_detection_correctness() {
    let sample_path = "ral/futures_util-d2cc80ffae74ea83.ll";
    if !std::path::Path::new(sample_path).exists() {
        eprintln!("Skipping test: {} not found", sample_path);
        return;
    }

    let module = IRParser::parse_file(sample_path).unwrap();

    for function in &module.functions {
        let cfg = CFG::from_function(function);
        let loops = LoopAnalyzer::analyze_loops(&cfg);

        for loop_info in &loops {
            // Loop header must be in body
            assert!(
                loop_info.body_blocks.contains(&loop_info.header),
                "Loop header must be in loop body"
            );

            // Back edges must point to header
            for (tail, head) in &loop_info.back_edges {
                assert_eq!(*head, loop_info.header, "Back edge must point to header");
                assert!(
                    loop_info.body_blocks.contains(tail),
                    "Back edge tail must be in loop body"
                );
            }

            // Nesting level should be reasonable
            assert!(
                loop_info.nesting_level < 100,
                "Nesting level should be reasonable"
            );

            // Verify bounds are valid
            match &loop_info.bounds {
                LoopBounds::Constant { min, max } => {
                    assert!(min <= max, "Min should be <= max");
                    assert!(*max > 0, "Max iterations should be positive");
                }
                LoopBounds::Parametric { expr } => {
                    assert!(
                        !expr.is_empty(),
                        "Parametric expression should not be empty"
                    );
                }
                LoopBounds::Unknown => {
                    // Unknown is valid
                }
            }
        }
    }
}

/// Test timing calculation correctness
#[test]
fn test_timing_calculation_correctness() {
    let platform = CortexM4Model::new();

    // Verify platform model sanity
    assert!(
        platform.cpu_frequency_mhz > 0,
        "CPU frequency must be positive"
    );
    assert!(
        !platform.instruction_timings.is_empty(),
        "Must have instruction timings"
    );

    // Verify all timings are reasonable
    for (class, cycles) in &platform.instruction_timings {
        assert!(
            cycles.best_case > 0,
            "Best case must be positive for {:?}",
            class
        );
        assert!(
            cycles.worst_case >= cycles.best_case,
            "Worst >= best for {:?}",
            class
        );
        assert!(
            cycles.worst_case < 1000,
            "Worst case should be reasonable for {:?}",
            class
        );
    }

    // Test cycle conversions
    let cycles = 168;
    let us = TimingCalculator::cycles_to_us(cycles, platform.cpu_frequency_mhz);
    assert!(
        (us - 1.0).abs() < 0.001,
        "168 cycles @ 168MHz should be ~1us"
    );

    let back = TimingCalculator::us_to_cycles(us, platform.cpu_frequency_mhz);
    assert_eq!(back, cycles, "Conversion should be reversible");
}

/// Test schedulability analysis correctness
#[test]
fn test_schedulability_correctness() {
    // Test case 1: Schedulable task set
    let schedulable_tasks = vec![
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
            wcet_cycles: 1000,
            wcet_us: 100.0,
            period_us: Some(2000.0),
            deadline_us: Some(2000.0),
            priority: None,
            preemptible: true,
            dependencies: vec![],
        },
    ];

    // RMA test
    let rma_result = RMAScheduler::schedulability_test(&schedulable_tasks);
    assert_eq!(
        rma_result,
        SchedulabilityResult::Schedulable,
        "Should be schedulable"
    );

    let utilization = RMAScheduler::calculate_utilization(&schedulable_tasks);
    assert!(
        (utilization - 0.15).abs() < 0.01,
        "Utilization should be ~0.15"
    );
    assert!(utilization <= 1.0, "Utilization must be <= 1.0");

    // EDF test
    let edf_result = EDFScheduler::schedulability_test(&schedulable_tasks);
    assert_eq!(
        edf_result,
        SchedulabilityResult::Schedulable,
        "EDF should be schedulable"
    );

    // Test case 2: Unschedulable task set
    let unschedulable_tasks = vec![
        Task {
            name: "task1".to_string(),
            function: "func1".to_string(),
            wcet_cycles: 9500,
            wcet_us: 950.0,
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

    let rma_result = RMAScheduler::schedulability_test(&unschedulable_tasks);
    assert!(
        matches!(rma_result, SchedulabilityResult::Unschedulable { .. }),
        "Should be unschedulable"
    );

    let utilization = RMAScheduler::calculate_utilization(&unschedulable_tasks);
    assert!(utilization > 1.0, "Utilization should exceed 1.0");
}

/// Test static schedule generation correctness
#[test]
fn test_static_schedule_correctness() {
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

    // Verify hyperperiod
    assert_eq!(
        schedule.hyperperiod_us, 2000.0,
        "Hyperperiod should be LCM(1000, 2000) = 2000"
    );

    // Verify schedule covers entire hyperperiod
    let total_time: f64 = schedule.slots.iter().map(|s| s.duration_us).sum();
    assert!(
        (total_time - schedule.hyperperiod_us).abs() < 0.001,
        "Schedule must cover hyperperiod"
    );

    // Verify no overlapping slots
    for i in 1..schedule.slots.len() {
        let prev_end = schedule.slots[i - 1].start_us + schedule.slots[i - 1].duration_us;
        let curr_start = schedule.slots[i].start_us;
        assert!(
            (prev_end - curr_start).abs() < 0.001,
            "Slots must not overlap"
        );
    }

    // Verify all slots have positive duration
    for slot in &schedule.slots {
        assert!(slot.duration_us > 0.0, "Slot duration must be positive");
    }

    // Count task executions
    let task1_count = schedule.slots.iter().filter(|s| s.task == "task1").count();
    let task2_count = schedule.slots.iter().filter(|s| s.task == "task2").count();

    assert_eq!(
        task1_count, 2,
        "task1 should execute 2 times in hyperperiod"
    );
    assert_eq!(task2_count, 1, "task2 should execute 1 time in hyperperiod");
}

/// Test JSON output correctness
#[test]
fn test_json_output_correctness() {
    let mut wcet_results = AHashMap::new();
    wcet_results.insert("test_func".to_string(), 1680);

    let tasks = vec![Task {
        name: "task1".to_string(),
        function: "func1".to_string(),
        wcet_cycles: 1680,
        wcet_us: 10.0,
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

    // Verify report structure
    assert_eq!(report.analysis_info.tool, "LALE");
    assert_eq!(report.analysis_info.platform, "ARM Cortex-M4");
    assert!(!report.analysis_info.timestamp.is_empty());

    // Verify WCET analysis
    assert!(!report.wcet_analysis.functions.is_empty());
    let func = &report.wcet_analysis.functions[0];
    assert_eq!(func.wcet_cycles, 1680);
    assert!((func.wcet_us - 10.0).abs() < 0.001);

    // Verify task model
    assert_eq!(report.task_model.tasks.len(), 1);
    assert_eq!(report.task_model.tasks[0].name, "task1");

    // Verify schedulability
    assert_eq!(report.schedulability.result, "schedulable");
    assert!(report.schedulability.utilization >= 0.0);
    assert!(report.schedulability.utilization <= 1.0);

    // Verify JSON serialization
    let json = JSONOutput::to_json(&report).expect("Should serialize to JSON");
    assert!(json.contains("LALE"));
    assert!(json.contains("task1"));
    assert!(json.contains("schedulable"));
}

/// Test edge case: Empty task set
#[test]
fn test_empty_task_set() {
    let empty_tasks: Vec<Task> = vec![];

    let rma_result = RMAScheduler::schedulability_test(&empty_tasks);
    assert_eq!(
        rma_result,
        SchedulabilityResult::Schedulable,
        "Empty set is schedulable"
    );

    let edf_result = EDFScheduler::schedulability_test(&empty_tasks);
    assert_eq!(
        edf_result,
        SchedulabilityResult::Schedulable,
        "Empty set is schedulable"
    );

    let utilization = RMAScheduler::calculate_utilization(&empty_tasks);
    assert_eq!(utilization, 0.0, "Empty set has zero utilization");
}

/// Test edge case: Single task
#[test]
fn test_single_task() {
    let single_task = vec![Task {
        name: "only_task".to_string(),
        function: "func".to_string(),
        wcet_cycles: 500,
        wcet_us: 50.0,
        period_us: Some(1000.0),
        deadline_us: Some(1000.0),
        priority: None,
        preemptible: true,
        dependencies: vec![],
    }];

    let rma_result = RMAScheduler::schedulability_test(&single_task);
    assert_eq!(rma_result, SchedulabilityResult::Schedulable);

    let utilization = RMAScheduler::calculate_utilization(&single_task);
    assert!((utilization - 0.05).abs() < 0.001);

    let schedule = StaticScheduleGenerator::generate_schedule(&single_task);
    assert_eq!(schedule.hyperperiod_us, 1000.0);
}

/// Test edge case: Tasks with no periods (aperiodic)
#[test]
fn test_aperiodic_tasks() {
    let aperiodic_tasks = vec![Task {
        name: "aperiodic".to_string(),
        function: "func".to_string(),
        wcet_cycles: 1000,
        wcet_us: 100.0,
        period_us: None, // No period
        deadline_us: None,
        priority: None,
        preemptible: true,
        dependencies: vec![],
    }];

    // Should handle gracefully
    let rma_result = RMAScheduler::schedulability_test(&aperiodic_tasks);
    assert_eq!(rma_result, SchedulabilityResult::Schedulable);

    let utilization = RMAScheduler::calculate_utilization(&aperiodic_tasks);
    assert_eq!(
        utilization, 0.0,
        "Aperiodic tasks don't contribute to utilization"
    );
}

/// Test LCM calculation correctness
#[test]
fn test_lcm_calculation() {
    use lale::scheduling::static_gen::StaticScheduleGenerator;

    // Test various period combinations
    let test_cases = vec![
        (vec![10, 15], 30),
        (vec![12, 18], 36),
        (vec![10, 20, 30], 60),
        (vec![7, 11], 77),
        (vec![100, 200, 300], 600),
    ];

    for (periods, expected_lcm) in test_cases {
        let tasks: Vec<Task> = periods
            .iter()
            .enumerate()
            .map(|(i, &period)| Task {
                name: format!("task{}", i),
                function: format!("func{}", i),
                wcet_cycles: 100,
                wcet_us: 10.0,
                period_us: Some(period as f64),
                deadline_us: Some(period as f64),
                priority: None,
                preemptible: true,
                dependencies: vec![],
            })
            .collect();

        let schedule = StaticScheduleGenerator::generate_schedule(&tasks);
        assert_eq!(
            schedule.hyperperiod_us, expected_lcm as f64,
            "LCM of {:?} should be {}",
            periods, expected_lcm
        );
    }
}

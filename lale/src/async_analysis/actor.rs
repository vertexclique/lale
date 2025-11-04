//! Actor model for Veecle OS systems
//!
//! Represents actors with timing constraints and WCET analysis results.

use crate::async_analysis::inkwell_segment::ActorSegment;
use crate::scheduling::Task;
use ahash::AHashMap;
use serde::{Deserialize, Serialize};

/// Actor in Veecle OS system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    /// Actor name
    pub name: String,

    /// LLVM function name
    pub function: String,

    /// Priority (higher = more important)
    pub priority: u8,

    /// Deadline in microseconds
    pub deadline_us: f64,

    /// Period in microseconds (None = aperiodic)
    pub period_us: Option<f64>,

    /// Core affinity (None = any core)
    pub core_affinity: Option<usize>,

    /// Execution segments
    pub segments: Vec<ActorSegment>,

    /// Per-segment WCET in cycles
    pub segment_wcets: AHashMap<u32, u64>,

    /// Actor-level WCET in cycles
    pub actor_wcet_cycles: u64,

    /// Actor-level WCET in microseconds
    pub actor_wcet_us: f64,
}

impl Actor {
    /// Create new actor
    pub fn new(
        name: String,
        function: String,
        priority: u8,
        deadline_us: f64,
        period_us: Option<f64>,
        core_affinity: Option<usize>,
    ) -> Self {
        Self {
            name,
            function,
            priority,
            deadline_us,
            period_us,
            core_affinity,
            segments: vec![],
            segment_wcets: AHashMap::new(),
            actor_wcet_cycles: 0,
            actor_wcet_us: 0.0,
        }
    }

    /// Compute actor-level WCET from segment WCETs
    pub fn compute_actor_wcet(&mut self, cpu_freq_mhz: u32) {
        // Strategy: Maximum segment WCET (conservative)
        self.actor_wcet_cycles = self.segment_wcets.values().copied().max().unwrap_or(0);

        self.actor_wcet_us = self.actor_wcet_cycles as f64 / cpu_freq_mhz as f64;
    }

    /// Convert to schedulable task
    pub fn to_task(&self) -> Task {
        Task {
            name: self.name.clone(),
            function: self.function.clone(),
            wcet_cycles: self.actor_wcet_cycles,
            wcet_us: self.actor_wcet_us,
            period_us: self.period_us,
            deadline_us: Some(self.deadline_us),
            priority: Some(self.priority),
            preemptible: false, // Cooperative scheduling
            dependencies: vec![],
        }
    }

    /// Get utilization (WCET / Period)
    pub fn utilization(&self) -> f64 {
        if let Some(period) = self.period_us {
            self.actor_wcet_us / period
        } else {
            0.0
        }
    }
}

/// Actor system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorSystem {
    /// System name
    pub name: String,

    /// All actors
    pub actors: Vec<Actor>,

    /// Platform name
    pub platform: String,

    /// Number of cores
    pub num_cores: usize,

    /// CPU frequency in MHz
    pub cpu_freq_mhz: u32,
}

impl ActorSystem {
    /// Create new actor system
    pub fn new(name: String, platform: String, num_cores: usize, cpu_freq_mhz: u32) -> Self {
        Self {
            name,
            actors: vec![],
            platform,
            num_cores,
            cpu_freq_mhz,
        }
    }

    /// Add actor to system
    pub fn add_actor(&mut self, actor: Actor) {
        self.actors.push(actor);
    }

    /// Get total system utilization
    pub fn total_utilization(&self) -> f64 {
        self.actors.iter().map(|a| a.utilization()).sum()
    }

    /// Get actors assigned to specific core
    pub fn actors_on_core(&self, core_id: usize) -> Vec<&Actor> {
        self.actors
            .iter()
            .filter(|a| a.core_affinity == Some(core_id))
            .collect()
    }
}

/// Actor configuration from external file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorConfig {
    pub name: String,
    pub function: String,
    pub priority: u8,
    pub deadline_ms: f64,
    pub period_ms: Option<f64>,
    pub core_affinity: Option<usize>,
}

impl ActorConfig {
    /// Convert to Actor (without WCET data)
    pub fn to_actor(&self) -> Actor {
        Actor::new(
            self.name.clone(),
            self.function.clone(),
            self.priority,
            self.deadline_ms * 1000.0, // ms to us
            self.period_ms.map(|p| p * 1000.0),
            self.core_affinity,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_actor_creation() {
        let actor = Actor::new(
            "test_actor".to_string(),
            "test::actor".to_string(),
            10,
            100000.0,
            Some(50000.0),
            Some(0),
        );

        assert_eq!(actor.name, "test_actor");
        assert_eq!(actor.priority, 10);
        assert_eq!(actor.deadline_us, 100000.0);
    }

    #[test]
    fn test_actor_utilization() {
        let mut actor = Actor::new(
            "test".to_string(),
            "test::actor".to_string(),
            10,
            100000.0,
            Some(50000.0),
            None,
        );

        actor.actor_wcet_us = 25000.0;
        assert_eq!(actor.utilization(), 0.5);
    }
}

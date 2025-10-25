use crate::microarch::pipeline::AbstractAddress;
use crate::microarch::state::MemoryConfig;

/// Memory system state (load/store buffers + background memory)
#[derive(Debug, Clone)]
pub struct MemorySystemState {
    /// Load buffer
    pub load_buffer: Vec<LoadEntry>,
    
    /// Store buffer
    pub store_buffer: Vec<StoreEntry>,
    
    /// Configuration
    config: MemoryConfig,
}

impl MemorySystemState {
    /// Create new memory system state
    pub fn new(config: &MemoryConfig) -> Self {
        Self {
            load_buffer: Vec::with_capacity(config.load_buffer_size),
            store_buffer: Vec::with_capacity(config.store_buffer_size),
            config: config.clone(),
        }
    }
    
    /// Check if load buffer is full
    pub fn load_buffer_full(&self) -> bool {
        self.load_buffer.len() >= self.config.load_buffer_size
    }
    
    /// Check if store buffer is full
    pub fn store_buffer_full(&self) -> bool {
        self.store_buffer.len() >= self.config.store_buffer_size
    }
    
    /// Issue a load operation
    pub fn issue_load(&mut self, address: AbstractAddress) -> Result<(), String> {
        if self.load_buffer_full() {
            return Err("Load buffer full".to_string());
        }
        
        self.load_buffer.push(LoadEntry {
            address,
            state: LoadState::Waiting,
            issued_cycle: 0,
        });
        
        Ok(())
    }
    
    /// Issue a store operation
    pub fn issue_store(&mut self, address: AbstractAddress) -> Result<(), String> {
        if self.store_buffer_full() {
            return Err("Store buffer full".to_string());
        }
        
        self.store_buffer.push(StoreEntry {
            address,
            state: StoreState::Waiting,
        });
        
        Ok(())
    }
    
    /// Advance memory system by one cycle
    pub fn tick(&mut self) {
        // Update load buffer entries
        for entry in &mut self.load_buffer {
            entry.issued_cycle += 1;
            
            // Transition states based on latency
            match entry.state {
                LoadState::Waiting => {
                    entry.state = LoadState::InFlight;
                }
                LoadState::InFlight => {
                    // Check if completed based on memory latency
                    let latency = match &self.config.memory_latency {
                        crate::microarch::state::MemoryLatency::Fixed { cycles } => *cycles,
                        crate::microarch::state::MemoryLatency::Variable { max, .. } => *max,
                    };
                    
                    if entry.issued_cycle >= latency {
                        entry.state = LoadState::Complete;
                    }
                }
                LoadState::Complete => {}
            }
        }
        
        // Remove completed loads
        self.load_buffer.retain(|e| !matches!(e.state, LoadState::Complete));
        
        // Update store buffer (simplified - stores complete immediately for now)
        self.store_buffer.clear();
    }
    
    /// Join two memory system states
    pub fn join(&self, other: &Self) -> Self {
        // Conservative join: take union of buffers
        let mut load_buffer = self.load_buffer.clone();
        load_buffer.extend(other.load_buffer.iter().cloned());
        
        let mut store_buffer = self.store_buffer.clone();
        store_buffer.extend(other.store_buffer.iter().cloned());
        
        Self {
            load_buffer,
            store_buffer,
            config: self.config.clone(),
        }
    }
}

/// Load buffer entry
#[derive(Debug, Clone)]
pub struct LoadEntry {
    /// Address being loaded
    pub address: AbstractAddress,
    
    /// Current state
    pub state: LoadState,
    
    /// Cycle when issued
    pub issued_cycle: u32,
}

/// Load operation state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadState {
    /// Waiting to be issued
    Waiting,
    
    /// In flight (memory access in progress)
    InFlight,
    
    /// Completed
    Complete,
}

/// Store buffer entry
#[derive(Debug, Clone)]
pub struct StoreEntry {
    /// Address being stored to
    pub address: AbstractAddress,
    
    /// Current state
    pub state: StoreState,
}

/// Store operation state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreState {
    /// Waiting to be issued
    Waiting,
    
    /// Completed
    Complete,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::microarch::state::MemoryLatency;
    
    fn test_config() -> MemoryConfig {
        MemoryConfig {
            load_buffer_size: 4,
            store_buffer_size: 4,
            memory_latency: MemoryLatency::Fixed { cycles: 10 },
        }
    }
    
    #[test]
    fn test_memory_system_creation() {
        let config = test_config();
        let mem = MemorySystemState::new(&config);
        
        assert_eq!(mem.load_buffer.len(), 0);
        assert_eq!(mem.store_buffer.len(), 0);
        assert!(!mem.load_buffer_full());
        assert!(!mem.store_buffer_full());
    }
    
    #[test]
    fn test_issue_load() {
        let config = test_config();
        let mut mem = MemorySystemState::new(&config);
        
        let result = mem.issue_load(AbstractAddress::Concrete(0x1000));
        assert!(result.is_ok());
        assert_eq!(mem.load_buffer.len(), 1);
    }
    
    #[test]
    fn test_issue_store() {
        let config = test_config();
        let mut mem = MemorySystemState::new(&config);
        
        let result = mem.issue_store(AbstractAddress::Concrete(0x2000));
        assert!(result.is_ok());
        assert_eq!(mem.store_buffer.len(), 1);
    }
    
    #[test]
    fn test_load_buffer_full() {
        let config = test_config();
        let mut mem = MemorySystemState::new(&config);
        
        // Fill buffer
        for i in 0..4 {
            mem.issue_load(AbstractAddress::Concrete(0x1000 + i * 4)).unwrap();
        }
        
        assert!(mem.load_buffer_full());
        
        // Try to add one more
        let result = mem.issue_load(AbstractAddress::Concrete(0x2000));
        assert!(result.is_err());
    }
    
    #[test]
    fn test_memory_tick() {
        let config = test_config();
        let mut mem = MemorySystemState::new(&config);
        
        mem.issue_load(AbstractAddress::Concrete(0x1000)).unwrap();
        
        // Initially waiting
        assert_eq!(mem.load_buffer[0].state, LoadState::Waiting);
        
        // After one tick, should be in flight
        mem.tick();
        assert_eq!(mem.load_buffer[0].state, LoadState::InFlight);
    }
    
    #[test]
    fn test_memory_join() {
        let config = test_config();
        let mut mem1 = MemorySystemState::new(&config);
        let mut mem2 = MemorySystemState::new(&config);
        
        mem1.issue_load(AbstractAddress::Concrete(0x1000)).unwrap();
        mem2.issue_load(AbstractAddress::Concrete(0x2000)).unwrap();
        
        let joined = mem1.join(&mem2);
        
        // Should have both loads
        assert_eq!(joined.load_buffer.len(), 2);
    }
}

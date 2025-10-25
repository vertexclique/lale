use super::types::{CacheSet, MemoryBlock, AccessClassification};
use crate::microarch::state::{CacheConfig, CacheLevelConfig};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Complete cache state (instruction + data caches)
#[derive(Debug, Clone)]
pub struct CacheState {
    /// Instruction cache
    pub i_cache: Option<AbstractCache>,
    
    /// Data cache
    pub d_cache: Option<AbstractCache>,
}

impl CacheState {
    /// Create new cache state from configuration
    pub fn new(config: &CacheConfig) -> Self {
        Self {
            i_cache: config.instruction_cache.as_ref().map(AbstractCache::new),
            d_cache: config.data_cache.as_ref().map(AbstractCache::new),
        }
    }
    
    /// Compute hash for state comparison
    pub fn hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        
        if let Some(cache) = &self.i_cache {
            cache.hash(&mut hasher);
        }
        if let Some(cache) = &self.d_cache {
            cache.hash(&mut hasher);
        }
        
        hasher.finish()
    }
    
    /// Check if two cache states can be joined
    pub fn is_joinable(&self, other: &Self) -> bool {
        // Both must have same cache configuration
        match (&self.i_cache, &other.i_cache) {
            (Some(c1), Some(c2)) if !c1.is_compatible(c2) => return false,
            (None, Some(_)) | (Some(_), None) => return false,
            _ => {}
        }
        
        match (&self.d_cache, &other.d_cache) {
            (Some(c1), Some(c2)) if !c1.is_compatible(c2) => return false,
            (None, Some(_)) | (Some(_), None) => return false,
            _ => {}
        }
        
        true
    }
    
    /// Join two compatible cache states
    pub fn join(&self, other: &Self) -> Self {
        Self {
            i_cache: match (&self.i_cache, &other.i_cache) {
                (Some(c1), Some(c2)) => Some(c1.join(c2)),
                _ => None,
            },
            d_cache: match (&self.d_cache, &other.d_cache) {
                (Some(c1), Some(c2)) => Some(c1.join(c2)),
                _ => None,
            },
        }
    }
    
    /// Access instruction cache
    pub fn access_instruction(&mut self, address: u64) -> AccessClassification {
        if let Some(cache) = &mut self.i_cache {
            cache.access(address)
        } else {
            AccessClassification::AlwaysHit  // No cache = always hit
        }
    }
    
    /// Access data cache
    pub fn access_data(&mut self, address: u64) -> AccessClassification {
        if let Some(cache) = &mut self.d_cache {
            cache.access(address)
        } else {
            AccessClassification::AlwaysHit  // No cache = always hit
        }
    }
}

/// Abstract cache (single level)
#[derive(Debug, Clone)]
pub struct AbstractCache {
    /// Cache sets
    pub sets: Vec<CacheSet>,
    
    /// Configuration
    pub config: CacheLevelConfig,
}

impl AbstractCache {
    /// Create new abstract cache from configuration
    pub fn new(config: &CacheLevelConfig) -> Self {
        let num_sets = (config.size_kb * 1024) / (config.line_size_bytes * config.associativity);
        
        let sets = (0..num_sets)
            .map(|_| CacheSet::new(config.associativity))
            .collect();
        
        Self {
            sets,
            config: config.clone(),
        }
    }
    
    /// Get set index for address
    fn get_set_index(&self, address: u64) -> usize {
        let block = MemoryBlock::from_address(address, self.config.line_size_bytes);
        (block.0 as usize) % self.sets.len()
    }
    
    /// Access cache at address
    pub fn access(&mut self, address: u64) -> AccessClassification {
        let set_index = self.get_set_index(address);
        let block = MemoryBlock::from_address(address, self.config.line_size_bytes);
        
        let classification = self.sets[set_index].classify(block);
        
        // Update cache state
        self.sets[set_index].access(block);
        
        classification
    }
    
    /// Classify access without updating state
    pub fn classify(&self, address: u64) -> AccessClassification {
        let set_index = self.get_set_index(address);
        let block = MemoryBlock::from_address(address, self.config.line_size_bytes);
        
        self.sets[set_index].classify(block)
    }
    
    /// Check if two caches are compatible
    pub fn is_compatible(&self, other: &Self) -> bool {
        self.sets.len() == other.sets.len()
            && self.config.associativity == other.config.associativity
            && self.config.line_size_bytes == other.config.line_size_bytes
    }
    
    /// Join two compatible caches
    pub fn join(&self, other: &Self) -> Self {
        assert!(self.is_compatible(other), "Incompatible caches");
        
        let sets = self
            .sets
            .iter()
            .zip(other.sets.iter())
            .map(|(s1, s2)| s1.join(s2))
            .collect();
        
        Self {
            sets,
            config: self.config.clone(),
        }
    }
    
    /// Compute hash for state comparison
    pub fn hash<H: Hasher>(&self, state: &mut H) {
        for set in &self.sets {
            set.hash(state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::microarch::state::ReplacementPolicy;
    
    fn test_cache_config() -> CacheLevelConfig {
        CacheLevelConfig {
            size_kb: 16,
            line_size_bytes: 32,
            associativity: 4,
            replacement_policy: ReplacementPolicy::LRU,
        }
    }
    
    #[test]
    fn test_abstract_cache_creation() {
        let config = test_cache_config();
        let cache = AbstractCache::new(&config);
        
        // 16KB / (32 bytes * 4-way) = 128 sets
        assert_eq!(cache.sets.len(), 128);
    }
    
    #[test]
    fn test_cache_access() {
        let config = test_cache_config();
        let mut cache = AbstractCache::new(&config);
        
        let addr = 0x1000;
        
        // First access should be miss
        let result = cache.access(addr);
        assert_eq!(result, AccessClassification::AlwaysMiss);
        
        // Second access should be hit
        let result = cache.access(addr);
        assert_eq!(result, AccessClassification::AlwaysHit);
    }
    
    #[test]
    fn test_cache_state_creation() {
        let config = CacheConfig {
            instruction_cache: Some(test_cache_config()),
            data_cache: Some(test_cache_config()),
        };
        
        let state = CacheState::new(&config);
        
        assert!(state.i_cache.is_some());
        assert!(state.d_cache.is_some());
    }
    
    #[test]
    fn test_cache_state_access() {
        let config = CacheConfig {
            instruction_cache: Some(test_cache_config()),
            data_cache: Some(test_cache_config()),
        };
        
        let mut state = CacheState::new(&config);
        
        // Access instruction cache
        let result = state.access_instruction(0x1000);
        assert_eq!(result, AccessClassification::AlwaysMiss);
        
        // Access data cache
        let result = state.access_data(0x2000);
        assert_eq!(result, AccessClassification::AlwaysMiss);
    }
    
    #[test]
    fn test_cache_state_join() {
        let config = CacheConfig {
            instruction_cache: Some(test_cache_config()),
            data_cache: Some(test_cache_config()),
        };
        
        let mut state1 = CacheState::new(&config);
        let mut state2 = CacheState::new(&config);
        
        // Different accesses in each state
        state1.access_instruction(0x1000);
        state2.access_instruction(0x2000);
        
        assert!(state1.is_joinable(&state2));
        
        let joined = state1.join(&state2);
        assert!(joined.i_cache.is_some());
    }
}

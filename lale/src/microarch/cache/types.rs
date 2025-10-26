use ahash::AHashMap;
use std::hash::{Hash, Hasher};

/// Memory block identifier (cache line)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemoryBlock(pub u64);

impl MemoryBlock {
    /// Create memory block from address (simplified constructor)
    pub fn new(address: u64) -> Self {
        Self(address)
    }

    /// Create memory block from address and line size
    pub fn from_address(address: u64, line_size: usize) -> Self {
        Self(address / line_size as u64)
    }

    /// Get set index for this block
    pub fn set_index(&self, line_size: usize, num_sets: usize) -> usize {
        ((self.0 / line_size as u64) % num_sets as u64) as usize
    }
}

/// Age information for cache analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Age {
    /// Minimum age (must analysis)
    pub must_age: u32,

    /// Maximum age (may analysis)
    pub may_age: u32,
}

impl Age {
    /// Create new age with both must and may set to same value
    pub fn new(age: u32) -> Self {
        Self {
            must_age: age,
            may_age: age,
        }
    }

    /// Create age range
    pub fn range(must: u32, may: u32) -> Self {
        Self {
            must_age: must,
            may_age: may,
        }
    }

    /// Join two ages (conservative approximation)
    pub fn join(&self, other: &Self) -> Self {
        Self {
            must_age: self.must_age.min(other.must_age),
            may_age: self.may_age.max(other.may_age),
        }
    }

    /// Check if definitely in cache (age < associativity)
    pub fn is_must_hit(&self, associativity: usize) -> bool {
        (self.must_age as usize) < associativity
    }

    /// Check if block is in cache
    pub fn contains(&self, block: &MemoryBlock) -> bool {
        self.is_must_hit(4) // Simplified
    }

    /// Check if possibly not in cache
    pub fn is_may_miss(&self, associativity: usize) -> bool {
        (self.may_age as usize) >= associativity
    }
}

/// Cache set containing age information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheSet {
    /// Age map: memory block -> age
    pub ages: AHashMap<MemoryBlock, Age>,

    /// Set associativity
    pub associativity: usize,
}

impl CacheSet {
    /// Create new empty cache set
    pub fn new(associativity: usize) -> Self {
        Self {
            ages: AHashMap::new(),
            associativity,
        }
    }

    /// Access a memory block (update ages)
    pub fn access(&mut self, block: MemoryBlock) {
        // Age all other blocks
        for age in self.ages.values_mut() {
            age.must_age = age.must_age.saturating_add(1);
            age.may_age = age.may_age.saturating_add(1);
        }

        // Accessed block gets age 0
        self.ages.insert(block, Age::new(0));

        // Remove blocks that are definitely evicted
        self.ages
            .retain(|_, age| (age.must_age as usize) < self.associativity * 2);
    }

    /// Classify access as hit or miss
    pub fn classify(&self, block: MemoryBlock) -> AccessClassification {
        match self.ages.get(&block) {
            Some(age) if age.is_must_hit(self.associativity) => AccessClassification::AlwaysHit,
            Some(age) if age.is_may_miss(self.associativity) => AccessClassification::AlwaysMiss,
            Some(_) => AccessClassification::Unknown,
            None => AccessClassification::AlwaysMiss,
        }
    }

    /// Join two cache sets
    pub fn join(&self, other: &Self) -> Self {
        assert_eq!(self.associativity, other.associativity);

        let mut ages = AHashMap::new();

        // Blocks in both sets
        for (block, age1) in &self.ages {
            if let Some(age2) = other.ages.get(block) {
                ages.insert(*block, age1.join(age2));
            } else {
                // Block only in first set - conservative age
                ages.insert(*block, Age::range(age1.must_age, self.associativity as u32));
            }
        }

        // Blocks only in second set
        for (block, age2) in &other.ages {
            if !self.ages.contains_key(block) {
                ages.insert(*block, Age::range(age2.must_age, self.associativity as u32));
            }
        }

        Self {
            ages,
            associativity: self.associativity,
        }
    }

    /// Check if block is in set
    pub fn contains(&self, block: &MemoryBlock) -> bool {
        self.ages.contains_key(block)
    }

    /// Must join - intersection
    pub fn must_join(&self, other: &Self) -> Self {
        let mut ages = AHashMap::new();

        // Only keep blocks in both sets
        for (block, age1) in &self.ages {
            if let Some(age2) = other.ages.get(block) {
                ages.insert(*block, age1.join(age2));
            }
        }

        Self {
            ages,
            associativity: self.associativity,
        }
    }

    /// May join - union
    pub fn may_join(&self, other: &Self) -> Self {
        self.join(other) // Same as regular join for may analysis
    }

    /// Compute hash for state comparison
    pub fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash sorted blocks for deterministic result
        let mut blocks: Vec<_> = self.ages.keys().collect();
        blocks.sort_by_key(|b| b.0);

        for block in blocks {
            block.hash(state);
            if let Some(age) = self.ages.get(block) {
                age.must_age.hash(state);
                age.may_age.hash(state);
            }
        }
    }
}

/// Cache access classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessClassification {
    /// Guaranteed cache hit
    AlwaysHit,

    /// Guaranteed cache miss
    AlwaysMiss,

    /// Unknown (may hit or miss)
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_block() {
        let block1 = MemoryBlock::from_address(0x1000, 32);
        let block2 = MemoryBlock::from_address(0x1010, 32);
        let block3 = MemoryBlock::from_address(0x1020, 32);

        // Same cache line
        assert_eq!(block1, block2);
        // Different cache line
        assert_ne!(block1, block3);
    }

    #[test]
    fn test_age_operations() {
        let age1 = Age::new(5);
        let age2 = Age::new(10);

        let joined = age1.join(&age2);
        assert_eq!(joined.must_age, 5); // min
        assert_eq!(joined.may_age, 10); // max
    }

    #[test]
    fn test_age_classification() {
        let age_hit = Age::new(2);
        let age_miss = Age::new(10);

        assert!(age_hit.is_must_hit(4));
        assert!(!age_miss.is_must_hit(4));
        assert!(age_miss.is_may_miss(4));
    }

    #[test]
    fn test_cache_set_access() {
        let mut set = CacheSet::new(4);
        let block = MemoryBlock(100);

        set.access(block);

        // First access should be miss
        assert_eq!(set.classify(block), AccessClassification::AlwaysHit);

        // Access again
        set.access(block);
        assert_eq!(set.classify(block), AccessClassification::AlwaysHit);
    }

    #[test]
    fn test_cache_set_eviction() {
        let mut set = CacheSet::new(2); // 2-way associative

        let block1 = MemoryBlock(1);
        let block2 = MemoryBlock(2);
        let block3 = MemoryBlock(3);

        set.access(block1);
        set.access(block2);
        set.access(block3); // Should evict block1

        // block1 should be evicted
        assert_eq!(set.classify(block1), AccessClassification::AlwaysMiss);
        // block3 should be present
        assert_eq!(set.classify(block3), AccessClassification::AlwaysHit);
    }

    #[test]
    fn test_cache_set_join() {
        let mut set1 = CacheSet::new(4);
        let mut set2 = CacheSet::new(4);

        let block1 = MemoryBlock(1);
        let block2 = MemoryBlock(2);

        set1.access(block1);
        set2.access(block2);

        let joined = set1.join(&set2);

        // Both blocks should be in joined set
        assert!(joined.ages.contains_key(&block1));
        assert!(joined.ages.contains_key(&block2));
    }
}

/// Cache behavior classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheCategory {
    AlwaysHit,
    AlwaysMiss,
    Unknown,
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub size: usize,
    pub line_size: usize,
}

/// Cache model (placeholder for Phase 3)
pub struct CacheModel {
    pub instruction_cache: CacheConfig,
    pub data_cache: Option<CacheConfig>,
    pub cache_miss_penalty: u32,
}

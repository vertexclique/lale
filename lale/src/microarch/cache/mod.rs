pub mod lru;
pub mod may;
pub mod must;
pub mod persistence;
pub mod state;
pub mod types;

pub use lru::{LRUCache, LRUStack};
pub use may::{MayAnalysis, MayCacheState};
pub use must::{MustAnalysis, MustCacheState};
pub use persistence::{
    CacheAccessClass, CacheAnalysisResult, LoopPersistence, PersistenceAnalysis, PersistentBlocks,
};
pub use state::{AbstractCache, CacheState};
pub use types::{AccessClassification, Age, CacheSet, MemoryBlock};

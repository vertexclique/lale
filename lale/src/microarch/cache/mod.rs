pub mod lru;
pub mod may;
pub mod must;
pub mod persistence;
pub mod state;
pub mod types;

pub use lru::{LRUCache, LRUStack};
pub use may::{MayAnalysis, MayCacheState};
pub use must::{MustAnalysis, MustCacheState};
pub use persistence::{PersistenceAnalysis, PersistentBlocks, LoopPersistence, CacheAnalysisResult, CacheAccessClass};
pub use state::{CacheState, AbstractCache};
pub use types::{CacheSet, Age, MemoryBlock, AccessClassification};

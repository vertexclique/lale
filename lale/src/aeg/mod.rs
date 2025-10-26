pub mod builder;
pub mod compression;
pub mod types;

pub use builder::AEGBuilder;
pub use compression::{
    BlockEdge, BlockNode, CompressedAEG, Compression, CompressionMode, EfficientCompression,
    PreciseCompression,
};
pub use types::{AEGEdge, AEGNode, EdgeMetrics, AEG};

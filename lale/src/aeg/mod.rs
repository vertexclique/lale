pub mod builder;
pub mod compression;
pub mod types;

pub use builder::AEGBuilder;
pub use compression::{CompressedAEG, BlockNode, BlockEdge, CompressionMode, Compression, PreciseCompression, EfficientCompression};
pub use types::{AEG, AEGEdge, AEGNode, EdgeMetrics};

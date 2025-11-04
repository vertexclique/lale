//! High-level analyzers for WCET analysis
//!
//! This module provides convenient analyzer structs that wrap the lower-level
//! analysis components into easy-to-use interfaces.

pub mod actor_analyzer;
pub mod directory;
pub mod function;
pub mod module;

pub use actor_analyzer::ActorAnalyzer;
pub use directory::{DirectoryAnalysisResult, DirectoryAnalyzer};
pub use function::{FunctionAnalysisResult, FunctionAnalyzer};
pub use module::{FunctionTimingDetails, ModuleAnalysisResult, ModuleAnalyzer};

pub mod callgraph;
pub mod cfg;
pub mod parser;

pub use callgraph::CallGraph;
pub use cfg::{BasicBlock, EdgeType, CFG};
pub use parser::IRParser;

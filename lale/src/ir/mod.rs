pub mod callgraph;
pub mod cfg;
pub mod inkwell_parser;
pub mod parser;

pub use callgraph::CallGraph;
pub use cfg::{BasicBlock, EdgeType, CFG};
pub use inkwell_parser::{InkwellBasicBlock, InkwellFunction, InkwellParser, TerminatorKind};
pub use parser::IRParser;

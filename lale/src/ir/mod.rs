pub mod cfg;
pub mod inkwell_cfg;
pub mod inkwell_parser;

pub use cfg::{BasicBlock, EdgeType, CFG};
pub use inkwell_cfg::{InkwellBasicBlock as InkwellCFGBlock, InkwellCFG};
pub use inkwell_parser::{InkwellBasicBlock, InkwellFunction, InkwellParser, TerminatorKind};

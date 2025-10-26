pub mod cortex_ar;
pub mod cortex_m;
pub mod models;
pub mod riscv;

// ARM Cortex-M exports
pub use cortex_m::{CortexM0Model, CortexM33Model, CortexM3Model, CortexM4Model, CortexM7Model};

// ARM Cortex-R/A exports
pub use cortex_ar::{CortexA53Model, CortexA7Model, CortexR4Model, CortexR5Model};

// RISC-V exports
pub use riscv::{RV32GCModel, RV32IMACModel, RV32IModel, RV64GCModel};

// Platform model
pub use models::PlatformModel;

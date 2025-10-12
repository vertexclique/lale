pub mod cortex_ar;
pub mod cortex_m;
pub mod models;
pub mod riscv;

// ARM Cortex-M exports
pub use cortex_m::{CortexM0Model, CortexM3Model, CortexM4Model, CortexM7Model, CortexM33Model};

// ARM Cortex-R/A exports
pub use cortex_ar::{CortexR4Model, CortexR5Model, CortexA7Model, CortexA53Model};

// RISC-V exports
pub use riscv::{RV32IModel, RV32IMACModel, RV32GCModel, RV64GCModel};

// Platform model
pub use models::PlatformModel;

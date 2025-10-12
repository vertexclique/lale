/// Task annotation from IR metadata (placeholder for Phase 5)
#[derive(Debug, Clone)]
pub struct TaskAnnotation {
    pub name: String,
    pub period_us: Option<f64>,
    pub deadline_us: Option<f64>,
    pub priority: Option<u8>,
    pub preemptible: Option<bool>,
}

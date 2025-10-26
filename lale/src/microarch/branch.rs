use ahash::AHashMap;

/// Branch prediction outcome
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchPrediction {
    Taken,
    NotTaken,
}

/// Branch history for pattern-based prediction
#[derive(Debug, Clone)]
pub struct BranchHistory {
    /// History bits (1 = taken, 0 = not taken)
    history: u64,

    /// History length
    length: usize,
}

impl BranchHistory {
    pub fn new(length: usize) -> Self {
        Self { history: 0, length }
    }

    /// Update history with outcome
    pub fn update(&mut self, taken: bool) {
        self.history = (self.history << 1) | (taken as u64);
        self.history &= (1 << self.length) - 1;
    }

    /// Get history pattern
    pub fn pattern(&self) -> u64 {
        self.history
    }
}

/// Two-bit saturating counter for branch prediction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaturatingCounter {
    StronglyNotTaken = 0,
    WeaklyNotTaken = 1,
    WeaklyTaken = 2,
    StronglyTaken = 3,
}

impl SaturatingCounter {
    pub fn predict(&self) -> BranchPrediction {
        match self {
            Self::StronglyNotTaken | Self::WeaklyNotTaken => BranchPrediction::NotTaken,
            Self::WeaklyTaken | Self::StronglyTaken => BranchPrediction::Taken,
        }
    }

    pub fn update(&mut self, taken: bool) {
        *self = match (*self, taken) {
            (Self::StronglyNotTaken, false) => Self::StronglyNotTaken,
            (Self::StronglyNotTaken, true) => Self::WeaklyNotTaken,
            (Self::WeaklyNotTaken, false) => Self::StronglyNotTaken,
            (Self::WeaklyNotTaken, true) => Self::WeaklyTaken,
            (Self::WeaklyTaken, false) => Self::WeaklyNotTaken,
            (Self::WeaklyTaken, true) => Self::StronglyTaken,
            (Self::StronglyTaken, false) => Self::WeaklyTaken,
            (Self::StronglyTaken, true) => Self::StronglyTaken,
        };
    }
}

impl Default for SaturatingCounter {
    fn default() -> Self {
        Self::WeaklyNotTaken
    }
}

/// Branch Target Buffer (BTB) entry
#[derive(Debug, Clone)]
pub struct BTBEntry {
    /// Branch PC
    pub pc: u64,

    /// Target address
    pub target: u64,

    /// Prediction counter
    pub counter: SaturatingCounter,
}

/// Branch Target Buffer
pub struct BranchTargetBuffer {
    /// Entries indexed by PC
    entries: AHashMap<u64, BTBEntry>,

    /// Maximum size
    max_size: usize,
}

impl BranchTargetBuffer {
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: AHashMap::new(),
            max_size,
        }
    }

    /// Lookup branch prediction
    pub fn lookup(&self, pc: u64) -> Option<(u64, BranchPrediction)> {
        self.entries
            .get(&pc)
            .map(|entry| (entry.target, entry.counter.predict()))
    }

    /// Update BTB with branch outcome
    pub fn update(&mut self, pc: u64, target: u64, taken: bool) {
        if let Some(entry) = self.entries.get_mut(&pc) {
            entry.counter.update(taken);
            entry.target = target;
        } else if self.entries.len() < self.max_size {
            self.entries.insert(
                pc,
                BTBEntry {
                    pc,
                    target,
                    counter: if taken {
                        SaturatingCounter::WeaklyTaken
                    } else {
                        SaturatingCounter::WeaklyNotTaken
                    },
                },
            );
        }
    }
}

/// Global History Register for gshare predictor
pub struct GlobalHistoryRegister {
    /// History bits
    history: u64,

    /// History length
    length: usize,
}

impl GlobalHistoryRegister {
    pub fn new(length: usize) -> Self {
        Self { history: 0, length }
    }

    /// Update with branch outcome
    pub fn update(&mut self, taken: bool) {
        self.history = (self.history << 1) | (taken as u64);
        self.history &= (1 << self.length) - 1;
    }

    /// Get history value
    pub fn value(&self) -> u64 {
        self.history
    }
}

/// Gshare branch predictor
pub struct GsharePredictor {
    /// Pattern History Table
    pht: Vec<SaturatingCounter>,

    /// Global history register
    ghr: GlobalHistoryRegister,

    /// Index bits
    index_bits: usize,
}

impl GsharePredictor {
    pub fn new(index_bits: usize) -> Self {
        let table_size = 1 << index_bits;
        Self {
            pht: vec![SaturatingCounter::default(); table_size],
            ghr: GlobalHistoryRegister::new(index_bits),
            index_bits,
        }
    }

    /// Predict branch
    pub fn predict(&self, pc: u64) -> BranchPrediction {
        let index = self.get_index(pc);
        self.pht[index].predict()
    }

    /// Update predictor
    pub fn update(&mut self, pc: u64, taken: bool) {
        let index = self.get_index(pc);
        self.pht[index].update(taken);
        self.ghr.update(taken);
    }

    /// Get PHT index using XOR of PC and GHR
    fn get_index(&self, pc: u64) -> usize {
        let pc_bits = pc & ((1 << self.index_bits) - 1);
        (pc_bits ^ self.ghr.value()) as usize
    }
}

/// Speculative execution state
#[derive(Debug, Clone)]
pub struct SpeculativeState {
    /// Branch PC that caused speculation
    pub branch_pc: u64,

    /// Predicted target
    pub predicted_target: u64,

    /// Prediction (taken/not taken)
    pub prediction: BranchPrediction,

    /// Instructions speculatively executed
    pub speculative_instructions: Vec<usize>,
}

/// Speculation manager
pub struct SpeculationManager {
    /// Active speculative states (stack for nested speculation)
    speculative_stack: Vec<SpeculativeState>,

    /// Maximum speculation depth
    max_depth: usize,
}

impl SpeculationManager {
    pub fn new(max_depth: usize) -> Self {
        Self {
            speculative_stack: Vec::new(),
            max_depth,
        }
    }

    /// Start speculative execution
    pub fn speculate(
        &mut self,
        branch_pc: u64,
        predicted_target: u64,
        prediction: BranchPrediction,
    ) -> Result<(), String> {
        if self.speculative_stack.len() >= self.max_depth {
            return Err("Maximum speculation depth reached".to_string());
        }

        self.speculative_stack.push(SpeculativeState {
            branch_pc,
            predicted_target,
            prediction,
            speculative_instructions: Vec::new(),
        });

        Ok(())
    }

    /// Add speculatively executed instruction
    pub fn add_speculative_instruction(&mut self, instr_id: usize) {
        if let Some(state) = self.speculative_stack.last_mut() {
            state.speculative_instructions.push(instr_id);
        }
    }

    /// Resolve speculation (correct prediction)
    pub fn resolve_correct(&mut self) -> Option<SpeculativeState> {
        self.speculative_stack.pop()
    }

    /// Resolve speculation (misprediction - need to flush)
    pub fn resolve_misprediction(&mut self) -> Vec<usize> {
        let mut flushed = Vec::new();

        while let Some(state) = self.speculative_stack.pop() {
            flushed.extend(state.speculative_instructions);
        }

        flushed
    }

    /// Check if currently speculating
    pub fn is_speculating(&self) -> bool {
        !self.speculative_stack.is_empty()
    }

    /// Get speculation depth
    pub fn depth(&self) -> usize {
        self.speculative_stack.len()
    }
}

/// Combined branch prediction and speculation unit
pub struct BranchPredictionUnit {
    /// Branch target buffer
    btb: BranchTargetBuffer,

    /// Gshare predictor
    gshare: GsharePredictor,

    /// Speculation manager
    speculation: SpeculationManager,
}

impl BranchPredictionUnit {
    pub fn new(btb_size: usize, gshare_bits: usize, max_speculation_depth: usize) -> Self {
        Self {
            btb: BranchTargetBuffer::new(btb_size),
            gshare: GsharePredictor::new(gshare_bits),
            speculation: SpeculationManager::new(max_speculation_depth),
        }
    }

    /// Predict branch and start speculation
    pub fn predict_and_speculate(&mut self, pc: u64) -> Option<(u64, BranchPrediction)> {
        // Try BTB first
        if let Some((target, prediction)) = self.btb.lookup(pc) {
            let _ = self.speculation.speculate(pc, target, prediction);
            return Some((target, prediction));
        }

        // Fall back to gshare
        let prediction = self.gshare.predict(pc);
        None
    }

    /// Update with branch outcome
    pub fn update(&mut self, pc: u64, target: u64, taken: bool) {
        self.btb.update(pc, target, taken);
        self.gshare.update(pc, taken);

        // Resolve speculation
        if self.speculation.is_speculating() {
            // Check if prediction was correct
            let correct = match self.speculation.speculative_stack.last() {
                Some(state) => {
                    (taken
                        && state.prediction == BranchPrediction::Taken
                        && state.predicted_target == target)
                        || (!taken && state.prediction == BranchPrediction::NotTaken)
                }
                None => false,
            };

            if correct {
                self.speculation.resolve_correct();
            } else {
                self.speculation.resolve_misprediction();
            }
        }
    }

    /// Get speculation manager
    pub fn speculation_manager(&mut self) -> &mut SpeculationManager {
        &mut self.speculation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_saturating_counter() {
        let mut counter = SaturatingCounter::WeaklyNotTaken;
        assert_eq!(counter.predict(), BranchPrediction::NotTaken);

        counter.update(true);
        assert_eq!(counter.predict(), BranchPrediction::Taken);
    }

    #[test]
    fn test_btb() {
        let mut btb = BranchTargetBuffer::new(16);
        btb.update(0x1000, 0x2000, true);

        let result = btb.lookup(0x1000);
        assert!(result.is_some());

        let (target, prediction) = result.unwrap();
        assert_eq!(target, 0x2000);
        assert_eq!(prediction, BranchPrediction::Taken);
    }

    #[test]
    fn test_gshare() {
        let mut gshare = GsharePredictor::new(10);

        let prediction = gshare.predict(0x1000);
        gshare.update(0x1000, true);

        // After update, prediction may change
        let _ = gshare.predict(0x1000);
    }

    #[test]
    fn test_speculation_manager() {
        let mut manager = SpeculationManager::new(4);

        assert!(!manager.is_speculating());

        manager
            .speculate(0x1000, 0x2000, BranchPrediction::Taken)
            .unwrap();
        assert!(manager.is_speculating());
        assert_eq!(manager.depth(), 1);

        manager.add_speculative_instruction(0);
        manager.add_speculative_instruction(1);

        let flushed = manager.resolve_misprediction();
        assert_eq!(flushed.len(), 2);
        assert!(!manager.is_speculating());
    }
}

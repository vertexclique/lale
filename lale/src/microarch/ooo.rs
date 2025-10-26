use super::hazards::{HazardType, InstructionDependency, Register};
use ahash::{AHashMap, AHashSet};
use std::collections::VecDeque;

/// Reorder Buffer (ROB) entry
#[derive(Debug, Clone)]
pub struct ROBEntry {
    /// Instruction ID
    pub id: usize,

    /// Destination register
    pub dest: Option<Register>,

    /// Whether instruction has completed execution
    pub completed: bool,

    /// Result value (simplified)
    pub value: Option<u64>,
}

/// Reorder Buffer for out-of-order execution
pub struct ReorderBuffer {
    /// Buffer entries (FIFO order)
    entries: VecDeque<ROBEntry>,

    /// Maximum size
    max_size: usize,

    /// Head pointer (next to commit)
    head: usize,

    /// Tail pointer (next to allocate)
    tail: usize,
}

impl ReorderBuffer {
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(max_size),
            max_size,
            head: 0,
            tail: 0,
        }
    }

    /// Check if ROB is full
    pub fn is_full(&self) -> bool {
        self.entries.len() >= self.max_size
    }

    /// Check if ROB is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Allocate entry for new instruction
    pub fn allocate(&mut self, id: usize, dest: Option<Register>) -> Result<usize, String> {
        if self.is_full() {
            return Err("ROB full".to_string());
        }

        let entry = ROBEntry {
            id,
            dest,
            completed: false,
            value: None,
        };

        self.entries.push_back(entry);
        let index = self.tail;
        self.tail += 1;

        Ok(index)
    }

    /// Mark instruction as completed
    pub fn complete(&mut self, rob_index: usize, value: Option<u64>) {
        if let Some(entry) = self.entries.get_mut(rob_index - self.head) {
            entry.completed = true;
            entry.value = value;
        }
    }

    /// Commit head instruction if completed
    pub fn commit(&mut self) -> Option<ROBEntry> {
        if let Some(entry) = self.entries.front() {
            if entry.completed {
                self.head += 1;
                return self.entries.pop_front();
            }
        }
        None
    }

    /// Flush ROB (e.g., on branch misprediction)
    pub fn flush(&mut self) {
        self.entries.clear();
        self.head = 0;
        self.tail = 0;
    }
}

/// Reservation Station for instruction scheduling
#[derive(Debug, Clone)]
pub struct ReservationStation {
    /// Instruction ID
    pub id: usize,

    /// Operation type
    pub op: OperationType,

    /// Source operands
    pub src1: Operand,
    pub src2: Operand,

    /// Destination register
    pub dest: Option<Register>,

    /// ROB entry index
    pub rob_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationType {
    ALU,
    Load,
    Store,
    Branch,
}

#[derive(Debug, Clone)]
pub enum Operand {
    /// Ready value
    Ready(u64),

    /// Waiting for ROB entry
    Waiting(usize),

    /// Register value
    Register(Register),
}

impl Operand {
    pub fn is_ready(&self) -> bool {
        matches!(self, Operand::Ready(_))
    }
}

/// Reservation Station pool
pub struct ReservationStationPool {
    /// Stations
    stations: Vec<Option<ReservationStation>>,

    /// Maximum size
    max_size: usize,
}

impl ReservationStationPool {
    pub fn new(max_size: usize) -> Self {
        Self {
            stations: vec![None; max_size],
            max_size,
        }
    }

    /// Allocate station
    pub fn allocate(&mut self, station: ReservationStation) -> Result<usize, String> {
        for (i, slot) in self.stations.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(station);
                return Ok(i);
            }
        }
        Err("No free reservation stations".to_string())
    }

    /// Get ready instructions
    pub fn get_ready(&self) -> Vec<usize> {
        self.stations
            .iter()
            .enumerate()
            .filter_map(|(i, station)| {
                if let Some(s) = station {
                    if s.src1.is_ready() && s.src2.is_ready() {
                        return Some(i);
                    }
                }
                None
            })
            .collect()
    }

    /// Remove station
    pub fn remove(&mut self, index: usize) -> Option<ReservationStation> {
        self.stations.get_mut(index).and_then(|s| s.take())
    }

    /// Update operands when value becomes available
    pub fn broadcast(&mut self, rob_index: usize, value: u64) {
        for station in self.stations.iter_mut().flatten() {
            if matches!(station.src1, Operand::Waiting(idx) if idx == rob_index) {
                station.src1 = Operand::Ready(value);
            }
            if matches!(station.src2, Operand::Waiting(idx) if idx == rob_index) {
                station.src2 = Operand::Ready(value);
            }
        }
    }
}

/// Register Alias Table (RAT) for register renaming
pub struct RegisterAliasTable {
    /// Mapping from architectural register to ROB entry
    mappings: AHashMap<Register, usize>,
}

impl RegisterAliasTable {
    pub fn new() -> Self {
        Self {
            mappings: AHashMap::new(),
        }
    }

    /// Map register to ROB entry
    pub fn map(&mut self, register: Register, rob_index: usize) {
        self.mappings.insert(register, rob_index);
    }

    /// Get ROB entry for register
    pub fn get(&self, register: Register) -> Option<usize> {
        self.mappings.get(&register).copied()
    }

    /// Clear mapping
    pub fn clear(&mut self, register: Register) {
        self.mappings.remove(&register);
    }

    /// Flush all mappings
    pub fn flush(&mut self) {
        self.mappings.clear();
    }
}

impl Default for RegisterAliasTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Out-of-order execution engine
pub struct OOOEngine {
    /// Reorder buffer
    rob: ReorderBuffer,

    /// Reservation stations
    rs_pool: ReservationStationPool,

    /// Register alias table
    rat: RegisterAliasTable,

    /// Issue width (instructions per cycle)
    issue_width: usize,
}

impl OOOEngine {
    pub fn new(rob_size: usize, rs_size: usize, issue_width: usize) -> Self {
        Self {
            rob: ReorderBuffer::new(rob_size),
            rs_pool: ReservationStationPool::new(rs_size),
            rat: RegisterAliasTable::new(),
            issue_width,
        }
    }

    /// Issue instruction
    pub fn issue(&mut self, instr: &InstructionDependency) -> Result<(), String> {
        // Allocate ROB entry
        let dest = instr.writes.first().copied();
        let rob_index = self.rob.allocate(instr.id, dest)?;

        // Update RAT if writing to register
        if let Some(reg) = dest {
            self.rat.map(reg, rob_index);
        }

        // Create reservation station entry
        let station = ReservationStation {
            id: instr.id,
            op: OperationType::ALU, // Simplified
            src1: self.resolve_operand(instr.reads.get(0).copied()),
            src2: self.resolve_operand(instr.reads.get(1).copied()),
            dest,
            rob_index,
        };

        self.rs_pool.allocate(station)?;

        Ok(())
    }

    /// Resolve operand using RAT
    fn resolve_operand(&self, reg: Option<Register>) -> Operand {
        if let Some(r) = reg {
            if let Some(rob_idx) = self.rat.get(r) {
                Operand::Waiting(rob_idx)
            } else {
                Operand::Register(r)
            }
        } else {
            Operand::Ready(0)
        }
    }

    /// Execute ready instructions
    pub fn execute(&mut self) -> Vec<usize> {
        let ready = self.rs_pool.get_ready();
        let mut executed = Vec::new();

        for &rs_idx in ready.iter().take(self.issue_width) {
            if let Some(station) = self.rs_pool.remove(rs_idx) {
                // Simplified execution
                let value = 42; // Placeholder
                self.rob.complete(station.rob_index, Some(value));
                self.rs_pool.broadcast(station.rob_index, value);
                executed.push(station.id);
            }
        }

        executed
    }

    /// Commit instructions
    pub fn commit(&mut self) -> Vec<usize> {
        let mut committed = Vec::new();

        while let Some(entry) = self.rob.commit() {
            if let Some(reg) = entry.dest {
                // Clear RAT mapping if this was the latest write
                if self.rat.get(reg) == Some(entry.id) {
                    self.rat.clear(reg);
                }
            }
            committed.push(entry.id);
        }

        committed
    }

    /// Flush pipeline (e.g., on branch misprediction)
    pub fn flush(&mut self) {
        self.rob.flush();
        self.rat.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rob_creation() {
        let rob = ReorderBuffer::new(16);
        assert!(rob.is_empty());
        assert!(!rob.is_full());
    }

    #[test]
    fn test_rob_allocate() {
        let mut rob = ReorderBuffer::new(16);
        let result = rob.allocate(0, Some(Register(1)));
        assert!(result.is_ok());
    }

    #[test]
    fn test_rob_commit() {
        let mut rob = ReorderBuffer::new(16);
        let idx = rob.allocate(0, Some(Register(1))).unwrap();
        rob.complete(idx, Some(42));

        let committed = rob.commit();
        assert!(committed.is_some());
    }

    #[test]
    fn test_rat() {
        let mut rat = RegisterAliasTable::new();
        rat.map(Register(1), 5);

        assert_eq!(rat.get(Register(1)), Some(5));
        assert_eq!(rat.get(Register(2)), None);
    }

    #[test]
    fn test_ooo_engine() {
        let mut engine = OOOEngine::new(16, 8, 2);

        let instr = InstructionDependency {
            id: 0,
            reads: vec![],
            writes: vec![Register(1)],
            stage: 0,
        };

        assert!(engine.issue(&instr).is_ok());
    }
}

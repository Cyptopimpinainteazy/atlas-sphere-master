//! Quantum backend abstraction
//!
//! Provides a unified interface for:
//! - Local statevector simulation
//! - GPU-accelerated simulation
//! - External QPU backends (IBM, Braket, etc.)

use async_trait::async_trait;
use num_complex::Complex64;
use rand::Rng;
use std::collections::HashMap;
use crate::error::{SwarmError, SwarmResult};
use crate::types::{ExternalBackend, QuantumResult};
use super::circuit::QuantumCircuit;

/// Capabilities of a quantum backend
#[derive(Debug, Clone)]
pub struct BackendCapabilities {
    /// Backend name
    pub name: String,
    /// Maximum qubits
    pub max_qubits: usize,
    /// Maximum circuit depth
    pub max_depth: usize,
    /// Supported gate set
    pub gates: Vec<String>,
    /// Is simulator
    pub is_simulator: bool,
    /// Estimated cost per shot
    pub cost_per_shot: f64,
    /// Average queue time (seconds)
    pub avg_queue_time: f64,
}

/// Result of circuit execution
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Measurement counts
    pub counts: HashMap<String, u64>,
    /// Total shots
    pub total_shots: u64,
    /// Statevector (if available)
    pub statevector: Option<Vec<Complex64>>,
    /// Execution time (ms)
    pub execution_time_ms: u64,
    /// Backend used
    pub backend: String,
}

/// Quantum backend trait
#[async_trait]
pub trait QuantumBackend: Send + Sync {
    /// Get backend capabilities
    fn capabilities(&self) -> BackendCapabilities;

    /// Execute a quantum circuit
    async fn execute(&self, circuit: &QuantumCircuit, shots: usize) -> SwarmResult<ExecutionResult>;

    /// Get statevector (simulation only)
    async fn statevector(&self, circuit: &QuantumCircuit) -> SwarmResult<Vec<Complex64>>;

    /// Estimate execution cost
    fn estimate_cost(&self, circuit: &QuantumCircuit, shots: usize) -> f64;

    /// Check if circuit is compatible
    fn is_compatible(&self, circuit: &QuantumCircuit) -> bool;
}

/// Local statevector simulator
pub struct LocalSimulator {
    max_qubits: usize,
    name: String,
}

impl LocalSimulator {
    pub fn new(max_qubits: usize) -> Self {
        Self {
            max_qubits,
            name: "LocalStatevectorSimulator".to_string(),
        }
    }

    /// Apply a gate to the statevector
    fn apply_gate(&self, state: &mut Vec<Complex64>, gate: &super::circuit::Gate) {
        let unitary = gate.unitary();
        
        match gate.qubits.len() {
            1 => self.apply_single_qubit_gate(state, &unitary, gate.qubits[0]),
            2 => self.apply_two_qubit_gate(state, &unitary, gate.qubits[0], gate.qubits[1]),
            _ => {} // Multi-qubit gates not yet implemented
        }
    }

    fn apply_single_qubit_gate(&self, state: &mut Vec<Complex64>, unitary: &[Vec<Complex64>], qubit: usize) {
        let n = state.len();
        let num_qubits = (n as f64).log2() as usize;
        let step = 1 << qubit;
        
        for i in 0..n {
            if i & step == 0 {
                let j = i | step;
                let a = state[i];
                let b = state[j];
                state[i] = unitary[0][0] * a + unitary[0][1] * b;
                state[j] = unitary[1][0] * a + unitary[1][1] * b;
            }
        }
    }

    fn apply_two_qubit_gate(&self, state: &mut Vec<Complex64>, unitary: &[Vec<Complex64>], qubit1: usize, qubit2: usize) {
        let n = state.len();
        let (low, high) = if qubit1 < qubit2 { (qubit1, qubit2) } else { (qubit2, qubit1) };
        let step_low = 1 << low;
        let step_high = 1 << high;

        for i in 0..n {
            if (i & step_low == 0) && (i & step_high == 0) {
                let i00 = i;
                let i01 = i | step_low;
                let i10 = i | step_high;
                let i11 = i | step_low | step_high;

                // Reorder based on qubit order
                let (idx0, idx1, idx2, idx3) = if qubit1 < qubit2 {
                    (i00, i01, i10, i11)
                } else {
                    (i00, i10, i01, i11)
                };

                let vals = [state[idx0], state[idx1], state[idx2], state[idx3]];
                let mut new_vals = [Complex64::new(0.0, 0.0); 4];

                for r in 0..4 {
                    for c in 0..4 {
                        new_vals[r] += unitary[r][c] * vals[c];
                    }
                }

                state[idx0] = new_vals[0];
                state[idx1] = new_vals[1];
                state[idx2] = new_vals[2];
                state[idx3] = new_vals[3];
            }
        }
    }

    /// Sample from statevector
    fn sample(&self, state: &[Complex64], shots: usize) -> HashMap<String, u64> {
        let mut rng = rand::thread_rng();
        let mut counts = HashMap::new();
        let num_qubits = (state.len() as f64).log2() as usize;

        // Calculate probabilities
        let probs: Vec<f64> = state.iter().map(|c| c.norm_sqr()).collect();
        
        // Sample
        for _ in 0..shots {
            let r: f64 = rng.gen();
            let mut cumsum = 0.0;
            let mut outcome = 0;
            
            for (i, &p) in probs.iter().enumerate() {
                cumsum += p;
                if r < cumsum {
                    outcome = i;
                    break;
                }
            }

            // Convert to binary string
            let bitstring: String = (0..num_qubits)
                .rev()
                .map(|b| if (outcome >> b) & 1 == 1 { '1' } else { '0' })
                .collect();

            *counts.entry(bitstring).or_insert(0) += 1;
        }

        counts
    }
}

#[async_trait]
impl QuantumBackend for LocalSimulator {
    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            name: self.name.clone(),
            max_qubits: self.max_qubits,
            max_depth: 1000,
            gates: vec![
                "H", "X", "Y", "Z", "S", "T",
                "Rx", "Ry", "Rz", "U",
                "CNOT", "CZ", "SWAP",
                "Rxx", "Ryy", "Rzz",
            ].into_iter().map(String::from).collect(),
            is_simulator: true,
            cost_per_shot: 0.0, // Free
            avg_queue_time: 0.0,
        }
    }

    async fn execute(&self, circuit: &QuantumCircuit, shots: usize) -> SwarmResult<ExecutionResult> {
        if circuit.num_qubits > self.max_qubits {
            return Err(SwarmError::QuantumBackend(format!(
                "Circuit has {} qubits, max is {}",
                circuit.num_qubits, self.max_qubits
            )));
        }

        let start = std::time::Instant::now();
        
        // Get statevector
        let state = self.statevector(circuit).await?;
        
        // Sample
        let counts = self.sample(&state, shots);
        
        let execution_time_ms = start.elapsed().as_millis() as u64;

        Ok(ExecutionResult {
            counts,
            total_shots: shots as u64,
            statevector: Some(state),
            execution_time_ms,
            backend: self.name.clone(),
        })
    }

    async fn statevector(&self, circuit: &QuantumCircuit) -> SwarmResult<Vec<Complex64>> {
        let n = 1 << circuit.num_qubits;
        let mut state = vec![Complex64::new(0.0, 0.0); n];
        state[0] = Complex64::new(1.0, 0.0); // |00...0⟩

        for gate in &circuit.gates {
            self.apply_gate(&mut state, gate);
        }

        Ok(state)
    }

    fn estimate_cost(&self, _circuit: &QuantumCircuit, _shots: usize) -> f64 {
        0.0 // Local simulation is free
    }

    fn is_compatible(&self, circuit: &QuantumCircuit) -> bool {
        circuit.num_qubits <= self.max_qubits
    }
}

/// External QPU backend (placeholder for IBM, Braket, etc.)
pub struct ExternalQpu {
    backend_type: ExternalBackend,
    api_key: Option<String>,
}

impl ExternalQpu {
    pub fn new(backend_type: ExternalBackend, api_key: Option<String>) -> Self {
        Self {
            backend_type,
            api_key,
        }
    }

    fn backend_name(&self) -> &str {
        match self.backend_type {
            ExternalBackend::IbmQuantum => "IBM Quantum",
            ExternalBackend::AmazonBraket => "Amazon Braket",
            ExternalBackend::GoogleCirq => "Google Cirq",
            ExternalBackend::DWave => "D-Wave",
            ExternalBackend::IonQ => "IonQ",
            ExternalBackend::Rigetti => "Rigetti",
        }
    }
}

#[async_trait]
impl QuantumBackend for ExternalQpu {
    fn capabilities(&self) -> BackendCapabilities {
        match self.backend_type {
            ExternalBackend::IbmQuantum => BackendCapabilities {
                name: "IBM Quantum".to_string(),
                max_qubits: 127,
                max_depth: 1000,
                gates: vec!["H", "X", "Y", "Z", "Rx", "Ry", "Rz", "CNOT", "CZ"]
                    .into_iter().map(String::from).collect(),
                is_simulator: false,
                cost_per_shot: 0.0001,
                avg_queue_time: 300.0,
            },
            ExternalBackend::AmazonBraket => BackendCapabilities {
                name: "Amazon Braket".to_string(),
                max_qubits: 79,
                max_depth: 500,
                gates: vec!["H", "X", "Y", "Z", "Rx", "Ry", "Rz", "CNOT"]
                    .into_iter().map(String::from).collect(),
                is_simulator: false,
                cost_per_shot: 0.0003,
                avg_queue_time: 120.0,
            },
            ExternalBackend::DWave => BackendCapabilities {
                name: "D-Wave".to_string(),
                max_qubits: 5000, // Quantum annealer
                max_depth: 1, // Annealing is single-step
                gates: vec!["QUBO"].into_iter().map(String::from).collect(),
                is_simulator: false,
                cost_per_shot: 0.02,
                avg_queue_time: 60.0,
            },
            _ => BackendCapabilities {
                name: self.backend_name().to_string(),
                max_qubits: 50,
                max_depth: 500,
                gates: vec!["H", "X", "CNOT"].into_iter().map(String::from).collect(),
                is_simulator: false,
                cost_per_shot: 0.001,
                avg_queue_time: 180.0,
            },
        }
    }

    async fn execute(&self, circuit: &QuantumCircuit, shots: usize) -> SwarmResult<ExecutionResult> {
        // In production, this would call the actual QPU API
        // For now, we simulate with a local backend and add realistic delays
        
        if self.api_key.is_none() {
            return Err(SwarmError::ExternalQpu(format!(
                "No API key configured for {}",
                self.backend_name()
            )));
        }

        // Simulate queue time (reduced for demo)
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Use local simulator as fallback
        let local = LocalSimulator::new(self.capabilities().max_qubits);
        let mut result = local.execute(circuit, shots).await?;
        result.backend = self.backend_name().to_string();
        
        // Add noise (simplified)
        // Real QPUs have gate errors, readout errors, decoherence...
        
        Ok(result)
    }

    async fn statevector(&self, _circuit: &QuantumCircuit) -> SwarmResult<Vec<Complex64>> {
        Err(SwarmError::ExternalQpu(
            "Statevector not available on real QPU".to_string()
        ))
    }

    fn estimate_cost(&self, circuit: &QuantumCircuit, shots: usize) -> f64 {
        let caps = self.capabilities();
        let base_cost = caps.cost_per_shot * shots as f64;
        
        // Add premium for circuit complexity
        let depth_factor = 1.0 + (circuit.depth() as f64 / 100.0);
        let qubit_factor = 1.0 + (circuit.num_qubits as f64 / 50.0);
        
        base_cost * depth_factor * qubit_factor
    }

    fn is_compatible(&self, circuit: &QuantumCircuit) -> bool {
        let caps = self.capabilities();
        circuit.num_qubits <= caps.max_qubits && circuit.depth() <= caps.max_depth
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::circuit::CircuitBuilder;

    #[tokio::test]
    async fn test_local_simulator_bell_state() {
        let simulator = LocalSimulator::new(20);
        
        let circuit = CircuitBuilder::new(2)
            .h(0)
            .cnot(0, 1)
            .measure_all()
            .build();

        let result = simulator.execute(&circuit, 1000).await.unwrap();
        
        // Bell state should give roughly equal 00 and 11
        let count_00 = result.counts.get("00").copied().unwrap_or(0);
        let count_11 = result.counts.get("11").copied().unwrap_or(0);
        let count_01 = result.counts.get("01").copied().unwrap_or(0);
        let count_10 = result.counts.get("10").copied().unwrap_or(0);

        // 00 and 11 should dominate
        assert!(count_00 + count_11 > 900);
        assert!(count_01 + count_10 < 100);
    }

    #[tokio::test]
    async fn test_hadamard_superposition() {
        let simulator = LocalSimulator::new(20);
        
        let circuit = CircuitBuilder::new(1)
            .h(0)
            .measure_all()
            .build();

        let result = simulator.execute(&circuit, 1000).await.unwrap();
        
        // Should be roughly 50/50
        let count_0 = result.counts.get("0").copied().unwrap_or(0);
        let count_1 = result.counts.get("1").copied().unwrap_or(0);
        
        assert!(count_0 > 400 && count_0 < 600);
        assert!(count_1 > 400 && count_1 < 600);
    }

    #[test]
    fn test_backend_capabilities() {
        let local = LocalSimulator::new(20);
        let caps = local.capabilities();
        
        assert_eq!(caps.max_qubits, 20);
        assert!(caps.is_simulator);
        assert_eq!(caps.cost_per_shot, 0.0);
    }
}

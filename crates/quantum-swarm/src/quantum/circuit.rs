//! Quantum circuit representation and construction

use crate::error::{SwarmError, SwarmResult};
use crate::types::{GateType, SerializableCircuit, SerializableGate};
use num_complex::Complex64;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Qubit index
pub type Qubit = usize;

/// Quantum gate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gate {
    /// Gate type
    pub gate_type: GateType,
    /// Target qubits
    pub qubits: Vec<Qubit>,
    /// Gate parameters (for parametric gates)
    pub parameters: Vec<f64>,
}

impl Gate {
    /// Create a Hadamard gate
    pub fn h(qubit: Qubit) -> Self {
        Self {
            gate_type: GateType::H,
            qubits: vec![qubit],
            parameters: vec![],
        }
    }

    /// Create a Pauli-X gate
    pub fn x(qubit: Qubit) -> Self {
        Self {
            gate_type: GateType::X,
            qubits: vec![qubit],
            parameters: vec![],
        }
    }

    /// Create a Pauli-Y gate
    pub fn y(qubit: Qubit) -> Self {
        Self {
            gate_type: GateType::Y,
            qubits: vec![qubit],
            parameters: vec![],
        }
    }

    /// Create a Pauli-Z gate
    pub fn z(qubit: Qubit) -> Self {
        Self {
            gate_type: GateType::Z,
            qubits: vec![qubit],
            parameters: vec![],
        }
    }

    /// Create an Rx (rotation around X) gate
    pub fn rx(qubit: Qubit, theta: f64) -> Self {
        Self {
            gate_type: GateType::Rx,
            qubits: vec![qubit],
            parameters: vec![theta],
        }
    }

    /// Create an Ry (rotation around Y) gate
    pub fn ry(qubit: Qubit, theta: f64) -> Self {
        Self {
            gate_type: GateType::Ry,
            qubits: vec![qubit],
            parameters: vec![theta],
        }
    }

    /// Create an Rz (rotation around Z) gate
    pub fn rz(qubit: Qubit, theta: f64) -> Self {
        Self {
            gate_type: GateType::Rz,
            qubits: vec![qubit],
            parameters: vec![theta],
        }
    }

    /// Create a CNOT gate
    pub fn cnot(control: Qubit, target: Qubit) -> Self {
        Self {
            gate_type: GateType::Cnot,
            qubits: vec![control, target],
            parameters: vec![],
        }
    }

    /// Create a CZ gate
    pub fn cz(control: Qubit, target: Qubit) -> Self {
        Self {
            gate_type: GateType::Cz,
            qubits: vec![control, target],
            parameters: vec![],
        }
    }

    /// Create an RZZ gate
    pub fn rzz(qubit1: Qubit, qubit2: Qubit, theta: f64) -> Self {
        Self {
            gate_type: GateType::Rzz,
            qubits: vec![qubit1, qubit2],
            parameters: vec![theta],
        }
    }

    /// Create an RXX gate
    pub fn rxx(qubit1: Qubit, qubit2: Qubit, theta: f64) -> Self {
        Self {
            gate_type: GateType::Rxx,
            qubits: vec![qubit1, qubit2],
            parameters: vec![theta],
        }
    }

    /// Get the unitary matrix for this gate
    pub fn unitary(&self) -> Vec<Vec<Complex64>> {
        use std::f64::consts::PI;
        let i = Complex64::i();
        let zero = Complex64::new(0.0, 0.0);
        let one = Complex64::new(1.0, 0.0);
        let half = Complex64::new(0.5, 0.0);
        let inv_sqrt2 = Complex64::new(1.0 / 2.0_f64.sqrt(), 0.0);

        match self.gate_type {
            GateType::H => vec![vec![inv_sqrt2, inv_sqrt2], vec![inv_sqrt2, -inv_sqrt2]],
            GateType::X => vec![vec![zero, one], vec![one, zero]],
            GateType::Y => vec![vec![zero, -i], vec![i, zero]],
            GateType::Z => vec![vec![one, zero], vec![zero, -one]],
            GateType::S => vec![vec![one, zero], vec![zero, i]],
            GateType::T => vec![
                vec![one, zero],
                vec![zero, Complex64::from_polar(1.0, PI / 4.0)],
            ],
            GateType::Rx => {
                let theta = self.parameters.get(0).copied().unwrap_or(0.0);
                let cos = Complex64::new((theta / 2.0).cos(), 0.0);
                let sin = Complex64::new(0.0, -(theta / 2.0).sin());
                vec![vec![cos, sin], vec![sin, cos]]
            }
            GateType::Ry => {
                let theta = self.parameters.get(0).copied().unwrap_or(0.0);
                let cos = Complex64::new((theta / 2.0).cos(), 0.0);
                let sin = Complex64::new((theta / 2.0).sin(), 0.0);
                vec![vec![cos, -sin], vec![sin, cos]]
            }
            GateType::Rz => {
                let theta = self.parameters.get(0).copied().unwrap_or(0.0);
                vec![
                    vec![Complex64::from_polar(1.0, -theta / 2.0), zero],
                    vec![zero, Complex64::from_polar(1.0, theta / 2.0)],
                ]
            }
            GateType::Cnot => vec![
                vec![one, zero, zero, zero],
                vec![zero, one, zero, zero],
                vec![zero, zero, zero, one],
                vec![zero, zero, one, zero],
            ],
            GateType::Cz => vec![
                vec![one, zero, zero, zero],
                vec![zero, one, zero, zero],
                vec![zero, zero, one, zero],
                vec![zero, zero, zero, -one],
            ],
            _ => vec![vec![one]], // Identity fallback
        }
    }
}

/// Quantum circuit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumCircuit {
    /// Number of qubits
    pub num_qubits: usize,
    /// Gates in the circuit
    pub gates: Vec<Gate>,
    /// Qubits to measure (None = measure all)
    pub measurements: Option<Vec<Qubit>>,
    /// Parameter names for parametric circuits
    pub parameter_names: Vec<String>,
}

impl QuantumCircuit {
    /// Create a new empty circuit
    pub fn new(num_qubits: usize) -> Self {
        Self {
            num_qubits,
            gates: Vec::new(),
            measurements: None,
            parameter_names: Vec::new(),
        }
    }

    /// Add a gate to the circuit
    pub fn add_gate(&mut self, gate: Gate) -> SwarmResult<()> {
        // Validate qubit indices
        for &q in &gate.qubits {
            if q >= self.num_qubits {
                return Err(SwarmError::QuantumCircuit(format!(
                    "Qubit index {} out of bounds for {}-qubit circuit",
                    q, self.num_qubits
                )));
            }
        }
        self.gates.push(gate);
        Ok(())
    }

    /// Set measurement qubits
    pub fn measure(&mut self, qubits: Vec<Qubit>) -> SwarmResult<()> {
        for &q in &qubits {
            if q >= self.num_qubits {
                return Err(SwarmError::QuantumCircuit(format!(
                    "Measurement qubit {} out of bounds",
                    q
                )));
            }
        }
        self.measurements = Some(qubits);
        Ok(())
    }

    /// Measure all qubits
    pub fn measure_all(&mut self) {
        self.measurements = Some((0..self.num_qubits).collect());
    }

    /// Get circuit depth (number of layers)
    pub fn depth(&self) -> usize {
        if self.gates.is_empty() {
            return 0;
        }

        // Track the current depth for each qubit
        let mut qubit_depths = vec![0usize; self.num_qubits];

        for gate in &self.gates {
            // Find max depth among involved qubits
            let max_depth = gate
                .qubits
                .iter()
                .map(|&q| qubit_depths[q])
                .max()
                .unwrap_or(0);

            // Update all involved qubits to max_depth + 1
            for &q in &gate.qubits {
                qubit_depths[q] = max_depth + 1;
            }
        }

        qubit_depths.into_iter().max().unwrap_or(0)
    }

    /// Get total gate count
    pub fn gate_count(&self) -> usize {
        self.gates.len()
    }

    /// Bind parameters to create a concrete circuit
    pub fn bind_parameters(&self, params: &[f64]) -> SwarmResult<Self> {
        let mut bound = self.clone();
        let mut param_idx = 0;

        for gate in &mut bound.gates {
            for p in &mut gate.parameters {
                if param_idx < params.len() {
                    *p = params[param_idx];
                    param_idx += 1;
                }
            }
        }

        Ok(bound)
    }

    /// Convert to serializable format
    pub fn to_serializable(&self) -> SerializableCircuit {
        SerializableCircuit {
            num_qubits: self.num_qubits,
            gates: self
                .gates
                .iter()
                .map(|g| SerializableGate {
                    gate_type: g.gate_type,
                    qubits: g.qubits.clone(),
                    parameters: g.parameters.clone(),
                })
                .collect(),
            measurements: self
                .measurements
                .clone()
                .unwrap_or_else(|| (0..self.num_qubits).collect()),
        }
    }

    /// Create from serializable format
    pub fn from_serializable(s: SerializableCircuit) -> Self {
        Self {
            num_qubits: s.num_qubits,
            gates: s
                .gates
                .into_iter()
                .map(|g| Gate {
                    gate_type: g.gate_type,
                    qubits: g.qubits,
                    parameters: g.parameters,
                })
                .collect(),
            measurements: Some(s.measurements),
            parameter_names: Vec::new(),
        }
    }
}

/// Builder for constructing quantum circuits fluently
pub struct CircuitBuilder {
    circuit: QuantumCircuit,
}

impl CircuitBuilder {
    /// Create a new circuit builder
    pub fn new(num_qubits: usize) -> Self {
        Self {
            circuit: QuantumCircuit::new(num_qubits),
        }
    }

    /// Add Hadamard gate
    pub fn h(mut self, qubit: Qubit) -> Self {
        self.circuit.add_gate(Gate::h(qubit)).unwrap();
        self
    }

    /// Add Pauli-X gate
    pub fn x(mut self, qubit: Qubit) -> Self {
        self.circuit.add_gate(Gate::x(qubit)).unwrap();
        self
    }

    /// Add Pauli-Y gate
    pub fn y(mut self, qubit: Qubit) -> Self {
        self.circuit.add_gate(Gate::y(qubit)).unwrap();
        self
    }

    /// Add Pauli-Z gate
    pub fn z(mut self, qubit: Qubit) -> Self {
        self.circuit.add_gate(Gate::z(qubit)).unwrap();
        self
    }

    /// Add Rx gate
    pub fn rx(mut self, qubit: Qubit, theta: f64) -> Self {
        self.circuit.add_gate(Gate::rx(qubit, theta)).unwrap();
        self
    }

    /// Add Ry gate
    pub fn ry(mut self, qubit: Qubit, theta: f64) -> Self {
        self.circuit.add_gate(Gate::ry(qubit, theta)).unwrap();
        self
    }

    /// Add Rz gate
    pub fn rz(mut self, qubit: Qubit, theta: f64) -> Self {
        self.circuit.add_gate(Gate::rz(qubit, theta)).unwrap();
        self
    }

    /// Add CNOT gate
    pub fn cnot(mut self, control: Qubit, target: Qubit) -> Self {
        self.circuit.add_gate(Gate::cnot(control, target)).unwrap();
        self
    }

    /// Add CZ gate
    pub fn cz(mut self, control: Qubit, target: Qubit) -> Self {
        self.circuit.add_gate(Gate::cz(control, target)).unwrap();
        self
    }

    /// Add RZZ gate
    pub fn rzz(mut self, qubit1: Qubit, qubit2: Qubit, theta: f64) -> Self {
        self.circuit
            .add_gate(Gate::rzz(qubit1, qubit2, theta))
            .unwrap();
        self
    }

    /// Add barrier (for visualization, no-op in simulation)
    pub fn barrier(self) -> Self {
        // Barriers are implicit in our representation
        self
    }

    /// Measure specific qubits
    pub fn measure(mut self, qubits: Vec<Qubit>) -> Self {
        self.circuit.measure(qubits).unwrap();
        self
    }

    /// Measure all qubits
    pub fn measure_all(mut self) -> Self {
        self.circuit.measure_all();
        self
    }

    /// Apply Hadamard to all qubits (uniform superposition)
    pub fn hadamard_layer(mut self) -> Self {
        for q in 0..self.circuit.num_qubits {
            self.circuit.add_gate(Gate::h(q)).unwrap();
        }
        self
    }

    /// Apply parametric rotation layer
    pub fn rotation_layer(mut self, params: &[f64]) -> Self {
        for (i, &theta) in params.iter().enumerate() {
            if i < self.circuit.num_qubits {
                self.circuit.add_gate(Gate::ry(i, theta)).unwrap();
            }
        }
        self
    }

    /// Apply entangling layer (linear connectivity)
    pub fn entangling_layer(mut self) -> Self {
        for i in 0..self.circuit.num_qubits - 1 {
            self.circuit.add_gate(Gate::cnot(i, i + 1)).unwrap();
        }
        self
    }

    /// Build the circuit
    pub fn build(self) -> QuantumCircuit {
        self.circuit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_builder() {
        let circuit = CircuitBuilder::new(3)
            .h(0)
            .cnot(0, 1)
            .cnot(1, 2)
            .measure_all()
            .build();

        assert_eq!(circuit.num_qubits, 3);
        assert_eq!(circuit.gate_count(), 3);
        assert_eq!(circuit.depth(), 3);
    }

    #[test]
    fn test_hadamard_layer() {
        let circuit = CircuitBuilder::new(4).hadamard_layer().build();

        assert_eq!(circuit.gate_count(), 4);
        assert_eq!(circuit.depth(), 1);
    }

    #[test]
    fn test_ghz_state() {
        // GHZ state: H(0), CNOT(0,1), CNOT(1,2), CNOT(2,3)
        let circuit = CircuitBuilder::new(4)
            .h(0)
            .cnot(0, 1)
            .cnot(1, 2)
            .cnot(2, 3)
            .measure_all()
            .build();

        assert_eq!(circuit.depth(), 4);
    }

    #[test]
    fn test_bell_pair() {
        let circuit = CircuitBuilder::new(2).h(0).cnot(0, 1).measure_all().build();

        assert_eq!(circuit.num_qubits, 2);
        assert_eq!(circuit.gate_count(), 2);
    }
}

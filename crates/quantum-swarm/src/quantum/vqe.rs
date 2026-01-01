//! VQE (Variational Quantum Eigensolver) optimizer
//!
//! VQE is used for finding ground states of quantum systems,
//! applicable to molecular simulation and optimization problems.

use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use crate::error::{SwarmError, SwarmResult};
use super::backend::QuantumBackend;
use super::circuit::{CircuitBuilder, Gate, QuantumCircuit};

/// VQE configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VqeConfig {
    /// Ansatz type
    pub ansatz: Ansatz,
    /// Ansatz depth (layers)
    pub depth: usize,
    /// Number of shots per evaluation
    pub shots: usize,
    /// Maximum optimizer iterations
    pub max_iterations: usize,
    /// Convergence threshold
    pub convergence_threshold: f64,
    /// Learning rate
    pub learning_rate: f64,
    /// Optimizer type
    pub optimizer: OptimizerType,
}

impl Default for VqeConfig {
    fn default() -> Self {
        Self {
            ansatz: Ansatz::EfficientSU2,
            depth: 2,
            shots: 1024,
            max_iterations: 200,
            convergence_threshold: 1e-6,
            learning_rate: 0.1,
            optimizer: OptimizerType::Adam,
        }
    }
}

/// Ansatz (variational form) types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ansatz {
    /// RY ansatz (simple rotation only)
    RY,
    /// RY-RZ ansatz
    RYRZ,
    /// Efficient SU(2) ansatz
    EfficientSU2,
    /// Hardware-efficient ansatz
    HardwareEfficient,
    /// UCCSD (for chemistry)
    UCCSD,
}

/// Classical optimizer type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizerType {
    /// Gradient descent
    GradientDescent,
    /// Adam optimizer
    Adam,
    /// SPSA (Simultaneous Perturbation Stochastic Approximation)
    Spsa,
    /// COBYLA (Constrained Optimization BY Linear Approximation)
    Cobyla,
}

/// Result of VQE optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VqeResult {
    /// Optimal parameters
    pub optimal_params: Vec<f64>,
    /// Ground state energy estimate
    pub energy: f64,
    /// Convergence history
    pub history: Vec<f64>,
    /// Total iterations
    pub iterations: usize,
    /// Converged flag
    pub converged: bool,
}

/// Hamiltonian representation (Pauli decomposition)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hamiltonian {
    /// Number of qubits
    pub num_qubits: usize,
    /// Pauli terms: (coefficient, pauli_string)
    /// Pauli string: "IXYZ" means I⊗X⊗Y⊗Z
    pub terms: Vec<(f64, String)>,
}

impl Hamiltonian {
    /// Create a simple Heisenberg model
    pub fn heisenberg(n: usize, j: f64) -> Self {
        let mut terms = Vec::new();
        
        for i in 0..n - 1 {
            // XX interaction
            let mut pauli_xx = vec!['I'; n];
            pauli_xx[i] = 'X';
            pauli_xx[i + 1] = 'X';
            terms.push((j, pauli_xx.iter().collect()));

            // YY interaction
            let mut pauli_yy = vec!['I'; n];
            pauli_yy[i] = 'Y';
            pauli_yy[i + 1] = 'Y';
            terms.push((j, pauli_yy.iter().collect()));

            // ZZ interaction
            let mut pauli_zz = vec!['I'; n];
            pauli_zz[i] = 'Z';
            pauli_zz[i + 1] = 'Z';
            terms.push((j, pauli_zz.iter().collect()));
        }

        Self { num_qubits: n, terms }
    }

    /// Create transverse field Ising model
    pub fn transverse_ising(n: usize, j: f64, h: f64) -> Self {
        let mut terms = Vec::new();

        // ZZ interactions
        for i in 0..n - 1 {
            let mut pauli = vec!['I'; n];
            pauli[i] = 'Z';
            pauli[i + 1] = 'Z';
            terms.push((j, pauli.iter().collect()));
        }

        // Transverse field (X terms)
        for i in 0..n {
            let mut pauli = vec!['I'; n];
            pauli[i] = 'X';
            terms.push((h, pauli.iter().collect()));
        }

        Self { num_qubits: n, terms }
    }
}

/// VQE optimizer
pub struct VqeOptimizer {
    config: VqeConfig,
}

impl VqeOptimizer {
    pub fn new(config: VqeConfig) -> Self {
        Self { config }
    }

    /// Get number of parameters for the ansatz
    pub fn num_parameters(&self, num_qubits: usize) -> usize {
        match self.config.ansatz {
            Ansatz::RY => num_qubits * self.config.depth,
            Ansatz::RYRZ => 2 * num_qubits * self.config.depth,
            Ansatz::EfficientSU2 => 2 * num_qubits * self.config.depth,
            Ansatz::HardwareEfficient => 3 * num_qubits * self.config.depth,
            Ansatz::UCCSD => num_qubits * (num_qubits - 1) * self.config.depth,
        }
    }

    /// Build variational circuit
    pub fn build_circuit(&self, num_qubits: usize, params: &[f64]) -> SwarmResult<QuantumCircuit> {
        let mut circuit = QuantumCircuit::new(num_qubits);
        let mut param_idx = 0;

        for _layer in 0..self.config.depth {
            match self.config.ansatz {
                Ansatz::RY => {
                    // Single-qubit RY rotations
                    for q in 0..num_qubits {
                        if param_idx < params.len() {
                            circuit.add_gate(Gate::ry(q, params[param_idx]))?;
                            param_idx += 1;
                        }
                    }
                    // Entangling layer
                    for q in 0..num_qubits - 1 {
                        circuit.add_gate(Gate::cnot(q, q + 1))?;
                    }
                }
                Ansatz::RYRZ | Ansatz::EfficientSU2 => {
                    // RY rotations
                    for q in 0..num_qubits {
                        if param_idx < params.len() {
                            circuit.add_gate(Gate::ry(q, params[param_idx]))?;
                            param_idx += 1;
                        }
                    }
                    // RZ rotations
                    for q in 0..num_qubits {
                        if param_idx < params.len() {
                            circuit.add_gate(Gate::rz(q, params[param_idx]))?;
                            param_idx += 1;
                        }
                    }
                    // Entangling layer
                    for q in 0..num_qubits - 1 {
                        circuit.add_gate(Gate::cnot(q, q + 1))?;
                    }
                }
                Ansatz::HardwareEfficient => {
                    // Full rotation layer
                    for q in 0..num_qubits {
                        if param_idx + 2 < params.len() {
                            circuit.add_gate(Gate::rx(q, params[param_idx]))?;
                            circuit.add_gate(Gate::ry(q, params[param_idx + 1]))?;
                            circuit.add_gate(Gate::rz(q, params[param_idx + 2]))?;
                            param_idx += 3;
                        }
                    }
                    // Linear entangling layer
                    for q in 0..num_qubits - 1 {
                        circuit.add_gate(Gate::cz(q, q + 1))?;
                    }
                }
                Ansatz::UCCSD => {
                    // Simplified UCCSD-like ansatz
                    for i in 0..num_qubits {
                        for j in i + 1..num_qubits {
                            if param_idx < params.len() {
                                let theta = params[param_idx];
                                circuit.add_gate(Gate::cnot(i, j))?;
                                circuit.add_gate(Gate::ry(j, theta))?;
                                circuit.add_gate(Gate::cnot(i, j))?;
                                param_idx += 1;
                            }
                        }
                    }
                }
            }
        }

        Ok(circuit)
    }

    /// Measure Pauli expectation value
    async fn measure_pauli(
        &self,
        circuit: &QuantumCircuit,
        pauli_string: &str,
        backend: &dyn QuantumBackend,
    ) -> SwarmResult<f64> {
        let mut measure_circuit = circuit.clone();
        
        // Add basis rotation for measurement
        for (i, p) in pauli_string.chars().enumerate() {
            match p {
                'X' => {
                    measure_circuit.add_gate(Gate::h(i))?;
                }
                'Y' => {
                    // Rotate to Y basis: S†H
                    measure_circuit.add_gate(Gate::rz(i, -PI / 2.0))?;
                    measure_circuit.add_gate(Gate::h(i))?;
                }
                'Z' | 'I' => {} // No rotation needed
                _ => {}
            }
        }
        measure_circuit.measure_all();

        let result = backend.execute(&measure_circuit, self.config.shots).await?;

        // Calculate expectation value
        let mut exp_val = 0.0;
        let mut total = 0u64;

        for (bitstring, count) in &result.counts {
            let mut parity = 1.0;
            for (i, (bit, p)) in bitstring.chars().zip(pauli_string.chars()).enumerate() {
                if p != 'I' && bit == '1' {
                    parity *= -1.0;
                }
            }
            exp_val += parity * (*count as f64);
            total += count;
        }

        Ok(exp_val / total as f64)
    }

    /// Evaluate Hamiltonian expectation value
    async fn evaluate_energy(
        &self,
        circuit: &QuantumCircuit,
        hamiltonian: &Hamiltonian,
        backend: &dyn QuantumBackend,
    ) -> SwarmResult<f64> {
        let mut energy = 0.0;

        for (coeff, pauli_string) in &hamiltonian.terms {
            let exp_val = self.measure_pauli(circuit, pauli_string, backend).await?;
            energy += coeff * exp_val;
        }

        Ok(energy)
    }

    /// Run VQE optimization
    pub async fn optimize(
        &self,
        hamiltonian: &Hamiltonian,
        backend: &dyn QuantumBackend,
    ) -> SwarmResult<VqeResult> {
        let num_params = self.num_parameters(hamiltonian.num_qubits);
        
        // Initialize parameters randomly
        let mut params: Vec<f64> = (0..num_params)
            .map(|_| rand::random::<f64>() * PI)
            .collect();

        let mut history = Vec::new();
        let mut best_energy = f64::MAX;
        let mut best_params = params.clone();

        // Adam optimizer state
        let mut m = vec![0.0; num_params];
        let mut v = vec![0.0; num_params];
        let beta1 = 0.9;
        let beta2 = 0.999;
        let epsilon = 1e-8;

        for iteration in 0..self.config.max_iterations {
            // Build circuit and evaluate
            let circuit = self.build_circuit(hamiltonian.num_qubits, &params)?;
            let energy = self.evaluate_energy(&circuit, hamiltonian, backend).await?;
            history.push(energy);

            if energy < best_energy {
                best_energy = energy;
                best_params = params.clone();
            }

            // Check convergence
            if history.len() > 1 {
                let improvement = (history[history.len() - 2] - energy).abs();
                if improvement < self.config.convergence_threshold {
                    return Ok(VqeResult {
                        optimal_params: best_params,
                        energy: best_energy,
                        history,
                        iterations: iteration + 1,
                        converged: true,
                    });
                }
            }

            // Compute gradient using parameter shift
            let gradient = self.compute_gradient(hamiltonian, &params, backend).await?;

            // Update parameters using Adam
            for i in 0..num_params {
                m[i] = beta1 * m[i] + (1.0 - beta1) * gradient[i];
                v[i] = beta2 * v[i] + (1.0 - beta2) * gradient[i].powi(2);
                
                let m_hat = m[i] / (1.0 - beta1.powi(iteration as i32 + 1));
                let v_hat = v[i] / (1.0 - beta2.powi(iteration as i32 + 1));
                
                params[i] -= self.config.learning_rate * m_hat / (v_hat.sqrt() + epsilon);
            }
        }

        Ok(VqeResult {
            optimal_params: best_params,
            energy: best_energy,
            history,
            iterations: self.config.max_iterations,
            converged: false,
        })
    }

    /// Compute gradient using parameter shift rule
    async fn compute_gradient(
        &self,
        hamiltonian: &Hamiltonian,
        params: &[f64],
        backend: &dyn QuantumBackend,
    ) -> SwarmResult<Vec<f64>> {
        let shift = PI / 2.0;
        let mut gradient = vec![0.0; params.len()];

        for i in 0..params.len() {
            // Forward shift
            let mut params_plus = params.to_vec();
            params_plus[i] += shift;
            let circuit_plus = self.build_circuit(hamiltonian.num_qubits, &params_plus)?;
            let energy_plus = self.evaluate_energy(&circuit_plus, hamiltonian, backend).await?;

            // Backward shift
            let mut params_minus = params.to_vec();
            params_minus[i] -= shift;
            let circuit_minus = self.build_circuit(hamiltonian.num_qubits, &params_minus)?;
            let energy_minus = self.evaluate_energy(&circuit_minus, hamiltonian, backend).await?;

            gradient[i] = (energy_plus - energy_minus) / 2.0;
        }

        Ok(gradient)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::backend::LocalSimulator;

    #[tokio::test]
    async fn test_vqe_transverse_ising() {
        let hamiltonian = Hamiltonian::transverse_ising(2, -1.0, 0.5);
        
        let config = VqeConfig {
            ansatz: Ansatz::EfficientSU2,
            depth: 1,
            shots: 512,
            max_iterations: 10,
            ..Default::default()
        };

        let optimizer = VqeOptimizer::new(config);
        let backend = LocalSimulator::new(10);

        let result = optimizer.optimize(&hamiltonian, &backend).await.unwrap();
        
        println!("Energy: {}", result.energy);
        println!("Converged: {}", result.converged);
        assert!(result.history.len() > 0);
    }

    #[test]
    fn test_heisenberg_hamiltonian() {
        let h = Hamiltonian::heisenberg(3, 1.0);
        assert_eq!(h.num_qubits, 3);
        assert!(!h.terms.is_empty());
    }

    #[test]
    fn test_num_parameters() {
        let config = VqeConfig {
            ansatz: Ansatz::RYRZ,
            depth: 2,
            ..Default::default()
        };
        let optimizer = VqeOptimizer::new(config);
        
        // RYRZ: 2 * n * depth params
        assert_eq!(optimizer.num_parameters(4), 2 * 4 * 2);
    }
}

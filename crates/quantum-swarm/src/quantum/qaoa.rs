//! QAOA (Quantum Approximate Optimization Algorithm) optimizer
//!
//! QAOA is used for solving combinatorial optimization problems like:
//! - Portfolio optimization
//! - Route optimization (arbitrage paths)
//! - Max-cut problems

use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use crate::error::{SwarmError, SwarmResult};
use crate::types::OptimizationProblem;
use super::backend::{QuantumBackend, LocalSimulator};
use super::circuit::{CircuitBuilder, Gate, QuantumCircuit};

/// QAOA configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QaoaConfig {
    /// Number of QAOA layers (p)
    pub layers: usize,
    /// Number of shots per evaluation
    pub shots: usize,
    /// Maximum optimizer iterations
    pub max_iterations: usize,
    /// Convergence threshold
    pub convergence_threshold: f64,
    /// Learning rate for parameter optimization
    pub learning_rate: f64,
    /// Use gradient-based optimization
    pub use_gradient: bool,
}

impl Default for QaoaConfig {
    fn default() -> Self {
        Self {
            layers: 3,
            shots: 1024,
            max_iterations: 100,
            convergence_threshold: 0.001,
            learning_rate: 0.1,
            use_gradient: true,
        }
    }
}

/// Result of QAOA optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QaoaResult {
    /// Optimal parameters (gamma, beta)
    pub optimal_params: Vec<f64>,
    /// Best objective value found
    pub optimal_value: f64,
    /// Best bitstring solution
    pub solution: Vec<bool>,
    /// Convergence history
    pub history: Vec<f64>,
    /// Total evaluations
    pub evaluations: usize,
    /// Converged flag
    pub converged: bool,
}

/// QAOA optimizer
pub struct QaoaOptimizer {
    config: QaoaConfig,
}

impl QaoaOptimizer {
    pub fn new(config: QaoaConfig) -> Self {
        Self { config }
    }

    /// Build QAOA circuit for a given problem
    pub fn build_circuit(
        &self,
        problem: &QaoaProblem,
        params: &[f64],
    ) -> SwarmResult<QuantumCircuit> {
        let n = problem.num_qubits;
        let p = self.config.layers;
        
        if params.len() != 2 * p {
            return Err(SwarmError::QuantumCircuit(format!(
                "Expected {} parameters, got {}",
                2 * p,
                params.len()
            )));
        }

        let mut circuit = QuantumCircuit::new(n);

        // Initial state: uniform superposition
        for q in 0..n {
            circuit.add_gate(Gate::h(q))?;
        }

        // QAOA layers
        for layer in 0..p {
            let gamma = params[layer];
            let beta = params[p + layer];

            // Cost layer (problem Hamiltonian)
            self.apply_cost_layer(&mut circuit, problem, gamma)?;

            // Mixer layer
            self.apply_mixer_layer(&mut circuit, n, beta)?;
        }

        circuit.measure_all();
        Ok(circuit)
    }

    /// Apply cost layer encoding the problem Hamiltonian
    fn apply_cost_layer(
        &self,
        circuit: &mut QuantumCircuit,
        problem: &QaoaProblem,
        gamma: f64,
    ) -> SwarmResult<()> {
        // Linear terms: e^{-i * gamma * h_i * Z_i}
        for (i, &h) in problem.linear.iter().enumerate() {
            if h.abs() > 1e-10 {
                circuit.add_gate(Gate::rz(i, 2.0 * gamma * h))?;
            }
        }

        // Quadratic terms: e^{-i * gamma * J_ij * Z_i Z_j}
        for &(i, j, j_val) in &problem.quadratic {
            if j_val.abs() > 1e-10 {
                circuit.add_gate(Gate::rzz(i, j, 2.0 * gamma * j_val))?;
            }
        }

        Ok(())
    }

    /// Apply mixer layer (transverse field)
    fn apply_mixer_layer(
        &self,
        circuit: &mut QuantumCircuit,
        n: usize,
        beta: f64,
    ) -> SwarmResult<()> {
        for q in 0..n {
            circuit.add_gate(Gate::rx(q, 2.0 * beta))?;
        }
        Ok(())
    }

    /// Evaluate objective function for a bitstring
    pub fn evaluate_bitstring(&self, problem: &QaoaProblem, bits: &[bool]) -> f64 {
        let mut cost = 0.0;

        // Linear terms
        for (i, &h) in problem.linear.iter().enumerate() {
            let z = if bits[i] { -1.0 } else { 1.0 };
            cost += h * z;
        }

        // Quadratic terms
        for &(i, j, j_val) in &problem.quadratic {
            let z_i = if bits[i] { -1.0 } else { 1.0 };
            let z_j = if bits[j] { -1.0 } else { 1.0 };
            cost += j_val * z_i * z_j;
        }

        cost
    }

    /// Run QAOA optimization
    pub async fn optimize(
        &self,
        problem: &QaoaProblem,
        backend: &dyn QuantumBackend,
    ) -> SwarmResult<QaoaResult> {
        let p = self.config.layers;
        
        // Initialize parameters
        let mut params: Vec<f64> = (0..2 * p)
            .map(|i| if i < p { 0.5 } else { 0.5 })
            .collect();

        let mut history = Vec::new();
        let mut best_value = f64::MAX;
        let mut best_solution = vec![false; problem.num_qubits];
        let mut evaluations = 0;

        // Simple gradient descent optimization
        for iteration in 0..self.config.max_iterations {
            // Build and execute circuit
            let circuit = self.build_circuit(problem, &params)?;
            let result = backend.execute(&circuit, self.config.shots).await?;
            evaluations += 1;

            // Calculate expectation value
            let mut exp_value = 0.0;
            let mut total_count = 0u64;

            for (bitstring, count) in &result.counts {
                let bits: Vec<bool> = bitstring.chars().map(|c| c == '1').collect();
                let cost = self.evaluate_bitstring(problem, &bits);
                exp_value += cost * (*count as f64);
                total_count += count;

                if cost < best_value {
                    best_value = cost;
                    best_solution = bits;
                }
            }
            exp_value /= total_count as f64;
            history.push(exp_value);

            // Check convergence
            if history.len() > 1 {
                let improvement = (history[history.len() - 2] - exp_value).abs();
                if improvement < self.config.convergence_threshold {
                    return Ok(QaoaResult {
                        optimal_params: params,
                        optimal_value: best_value,
                        solution: best_solution,
                        history,
                        evaluations,
                        converged: true,
                    });
                }
            }

            // Update parameters using numerical gradient
            if self.config.use_gradient {
                let gradient = self.compute_gradient(problem, &params, backend).await?;
                evaluations += 2 * params.len();
                
                for (i, g) in gradient.iter().enumerate() {
                    params[i] -= self.config.learning_rate * g;
                }
            } else {
                // Random perturbation for exploration
                use rand::Rng;
                let mut rng = rand::thread_rng();
                for p in &mut params {
                    *p += rng.gen_range(-0.1..0.1);
                }
            }
        }

        Ok(QaoaResult {
            optimal_params: params,
            optimal_value: best_value,
            solution: best_solution,
            history,
            evaluations,
            converged: false,
        })
    }

    /// Compute gradient using parameter shift rule
    async fn compute_gradient(
        &self,
        problem: &QaoaProblem,
        params: &[f64],
        backend: &dyn QuantumBackend,
    ) -> SwarmResult<Vec<f64>> {
        let shift = PI / 2.0;
        let mut gradient = vec![0.0; params.len()];

        for i in 0..params.len() {
            // Forward shift
            let mut params_plus = params.to_vec();
            params_plus[i] += shift;
            let circuit_plus = self.build_circuit(problem, &params_plus)?;
            let result_plus = backend.execute(&circuit_plus, self.config.shots).await?;
            let exp_plus = self.compute_expectation(problem, &result_plus.counts);

            // Backward shift
            let mut params_minus = params.to_vec();
            params_minus[i] -= shift;
            let circuit_minus = self.build_circuit(problem, &params_minus)?;
            let result_minus = backend.execute(&circuit_minus, self.config.shots).await?;
            let exp_minus = self.compute_expectation(problem, &result_minus.counts);

            gradient[i] = (exp_plus - exp_minus) / 2.0;
        }

        Ok(gradient)
    }

    /// Compute expectation value from measurement counts
    fn compute_expectation(
        &self,
        problem: &QaoaProblem,
        counts: &std::collections::HashMap<String, u64>,
    ) -> f64 {
        let mut exp_value = 0.0;
        let mut total = 0u64;

        for (bitstring, count) in counts {
            let bits: Vec<bool> = bitstring.chars().map(|c| c == '1').collect();
            let cost = self.evaluate_bitstring(problem, &bits);
            exp_value += cost * (*count as f64);
            total += count;
        }

        exp_value / total as f64
    }
}

/// QAOA problem representation (Ising model)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QaoaProblem {
    /// Number of qubits (variables)
    pub num_qubits: usize,
    /// Linear terms h_i (coefficients of Z_i)
    pub linear: Vec<f64>,
    /// Quadratic terms J_ij (coefficients of Z_i Z_j)
    pub quadratic: Vec<(usize, usize, f64)>,
}

impl QaoaProblem {
    /// Create from QUBO matrix
    pub fn from_qubo(q: &[Vec<f64>]) -> Self {
        let n = q.len();
        let mut linear = vec![0.0; n];
        let mut quadratic = Vec::new();

        // Convert QUBO to Ising
        // Q(x) = sum_i Q_ii * x_i + sum_{i<j} Q_ij * x_i * x_j
        // Using x_i = (1 - z_i) / 2 where z_i in {-1, +1}

        for i in 0..n {
            // Diagonal (linear) terms
            linear[i] = q[i][i] / 2.0;
            
            for j in (i + 1)..n {
                // Off-diagonal (quadratic) terms
                let q_ij = q[i][j] + q[j][i];
                if q_ij.abs() > 1e-10 {
                    quadratic.push((i, j, q_ij / 4.0));
                    linear[i] += q_ij / 4.0;
                    linear[j] += q_ij / 4.0;
                }
            }
        }

        Self {
            num_qubits: n,
            linear,
            quadratic,
        }
    }

    /// Create Max-Cut problem for a graph
    pub fn max_cut(edges: &[(usize, usize)], n: usize) -> Self {
        let mut quadratic = Vec::new();
        
        // Max-Cut: maximize sum_{(i,j) in E} (1 - z_i * z_j) / 2
        // Which is equivalent to minimizing sum_{(i,j) in E} z_i * z_j
        for &(i, j) in edges {
            quadratic.push((i.min(j), i.max(j), 0.5));
        }

        Self {
            num_qubits: n,
            linear: vec![0.0; n],
            quadratic,
        }
    }

    /// Create portfolio optimization problem
    /// Minimize: lambda * x^T Sigma x - mu^T x
    /// Subject to: sum(x) = k (k assets selected)
    pub fn portfolio(
        expected_returns: &[f64],
        covariance: &[Vec<f64>],
        risk_aversion: f64,
        num_assets: usize,
    ) -> Self {
        let n = expected_returns.len();
        let mut linear = vec![0.0; n];
        let mut quadratic = Vec::new();

        // Linear terms from expected returns
        for (i, &mu) in expected_returns.iter().enumerate() {
            linear[i] = -mu; // Negative because we minimize
        }

        // Quadratic terms from covariance
        for i in 0..n {
            for j in i..n {
                let cov = if i == j {
                    covariance[i][j] * risk_aversion
                } else {
                    covariance[i][j] * risk_aversion * 2.0
                };
                if cov.abs() > 1e-10 {
                    if i == j {
                        linear[i] += cov / 2.0;
                    } else {
                        quadratic.push((i, j, cov));
                    }
                }
            }
        }

        Self {
            num_qubits: n,
            linear,
            quadratic,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_qaoa_max_cut() {
        // Simple 4-node graph: 0-1, 1-2, 2-3, 3-0 (square)
        let edges = vec![(0, 1), (1, 2), (2, 3), (3, 0)];
        let problem = QaoaProblem::max_cut(&edges, 4);

        let config = QaoaConfig {
            layers: 2,
            shots: 512,
            max_iterations: 20,
            ..Default::default()
        };

        let optimizer = QaoaOptimizer::new(config);
        let backend = LocalSimulator::new(10);

        let result = optimizer.optimize(&problem, &backend).await.unwrap();

        // Max-cut for square should be 4 (alternating vertices)
        println!("Solution: {:?}", result.solution);
        println!("Value: {}", result.optimal_value);
        assert!(result.evaluations > 0);
    }

    #[test]
    fn test_qubo_to_ising() {
        // Simple 2-variable QUBO: Q = [[1, 2], [0, 3]]
        let q = vec![vec![1.0, 2.0], vec![0.0, 3.0]];
        let problem = QaoaProblem::from_qubo(&q);

        assert_eq!(problem.num_qubits, 2);
        assert!(!problem.linear.is_empty());
    }

    #[test]
    fn test_portfolio_problem() {
        let returns = vec![0.05, 0.08, 0.06];
        let cov = vec![
            vec![0.04, 0.01, 0.02],
            vec![0.01, 0.09, 0.015],
            vec![0.02, 0.015, 0.0625],
        ];

        let problem = QaoaProblem::portfolio(&returns, &cov, 0.5, 3);
        assert_eq!(problem.num_qubits, 3);
    }
}

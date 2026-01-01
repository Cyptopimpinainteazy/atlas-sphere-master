//! QUBO (Quadratic Unconstrained Binary Optimization) solver
//!
//! Provides simulated annealing and quantum-inspired optimization
//! for combinatorial problems common in finance:
//! - Portfolio selection
//! - Route optimization
//! - Resource allocation

use crate::error::{SwarmError, SwarmResult};
use rand::Rng;
use serde::{Deserialize, Serialize};

/// QUBO matrix representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuboMatrix {
    /// Size of the matrix
    pub size: usize,
    /// Upper triangular matrix (including diagonal)
    pub coefficients: Vec<Vec<f64>>,
}

impl QuboMatrix {
    /// Create a new QUBO matrix
    pub fn new(size: usize) -> Self {
        Self {
            size,
            coefficients: vec![vec![0.0; size]; size],
        }
    }

    /// Set a coefficient
    pub fn set(&mut self, i: usize, j: usize, value: f64) {
        if i <= j {
            self.coefficients[i][j] = value;
        } else {
            self.coefficients[j][i] = value;
        }
    }

    /// Get a coefficient
    pub fn get(&self, i: usize, j: usize) -> f64 {
        if i <= j {
            self.coefficients[i][j]
        } else {
            self.coefficients[j][i]
        }
    }

    /// Evaluate the QUBO objective for a binary vector
    pub fn evaluate(&self, x: &[bool]) -> f64 {
        let mut cost = 0.0;
        for i in 0..self.size {
            if x[i] {
                cost += self.coefficients[i][i];
                for j in i + 1..self.size {
                    if x[j] {
                        cost += self.coefficients[i][j];
                    }
                }
            }
        }
        cost
    }

    /// Convert to Ising model
    pub fn to_ising(&self) -> IsingModel {
        let n = self.size;
        let mut h = vec![0.0; n];
        let mut j_matrix = vec![vec![0.0; n]; n];
        let mut offset = 0.0;

        // x_i = (1 - s_i) / 2 where s_i in {-1, +1}
        for i in 0..n {
            // Diagonal terms
            let q_ii = self.coefficients[i][i];
            h[i] = -q_ii / 2.0;
            offset += q_ii / 2.0;

            for j in i + 1..n {
                let q_ij = self.coefficients[i][j];
                j_matrix[i][j] = q_ij / 4.0;
                h[i] -= q_ij / 4.0;
                h[j] -= q_ij / 4.0;
                offset += q_ij / 4.0;
            }
        }

        IsingModel {
            size: n,
            h,
            j: j_matrix,
            offset,
        }
    }

    /// Create portfolio selection QUBO
    /// Objective: Maximize returns while minimizing risk
    /// x_i = 1 if asset i is selected
    pub fn portfolio_selection(
        returns: &[f64],
        covariance: &[Vec<f64>],
        risk_aversion: f64,
        budget: usize,
        penalty: f64,
    ) -> Self {
        let n = returns.len();
        let mut qubo = QuboMatrix::new(n);

        // Returns term (negative because QUBO minimizes)
        for i in 0..n {
            qubo.coefficients[i][i] -= returns[i];
        }

        // Risk term
        for i in 0..n {
            qubo.coefficients[i][i] += risk_aversion * covariance[i][i];
            for j in i + 1..n {
                qubo.coefficients[i][j] += risk_aversion * covariance[i][j];
            }
        }

        // Budget constraint: (sum x_i - budget)^2
        // Expanded: sum x_i + 2*sum_{i<j} x_i*x_j - 2*budget*sum x_i + budget^2
        for i in 0..n {
            qubo.coefficients[i][i] += penalty * (1.0 - 2.0 * budget as f64);
            for j in i + 1..n {
                qubo.coefficients[i][j] += penalty * 2.0;
            }
        }

        qubo
    }

    /// Create route optimization QUBO for TSP-like problems
    pub fn route_optimization(distances: &[Vec<f64>], penalty: f64) -> Self {
        let n = distances.len();
        let num_vars = n * n; // x_it = 1 if city i is visited at time t
        let mut qubo = QuboMatrix::new(num_vars);

        // Distance cost
        for t in 0..n - 1 {
            for i in 0..n {
                for j in 0..n {
                    if i != j {
                        let var_it = i * n + t;
                        let var_jt1 = j * n + (t + 1);
                        qubo.set(var_it.min(var_jt1), var_it.max(var_jt1), distances[i][j]);
                    }
                }
            }
        }

        // Constraint: each city visited exactly once
        for i in 0..n {
            // sum_t x_it = 1
            for t in 0..n {
                let var = i * n + t;
                qubo.coefficients[var][var] -= penalty;
                for t2 in t + 1..n {
                    let var2 = i * n + t2;
                    qubo.set(var.min(var2), var.max(var2), 2.0 * penalty);
                }
            }
        }

        // Constraint: one city at each time
        for t in 0..n {
            for i in 0..n {
                let var = i * n + t;
                qubo.coefficients[var][var] -= penalty;
                for i2 in i + 1..n {
                    let var2 = i2 * n + t;
                    qubo.set(var.min(var2), var.max(var2), 2.0 * penalty);
                }
            }
        }

        qubo
    }
}

/// Ising model representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsingModel {
    /// Number of spins
    pub size: usize,
    /// Linear terms (local fields)
    pub h: Vec<f64>,
    /// Coupling matrix (upper triangular)
    pub j: Vec<Vec<f64>>,
    /// Constant offset
    pub offset: f64,
}

impl IsingModel {
    /// Evaluate energy for a spin configuration
    pub fn energy(&self, spins: &[i8]) -> f64 {
        let mut e = self.offset;

        for i in 0..self.size {
            e += self.h[i] * spins[i] as f64;
            for j in i + 1..self.size {
                e += self.j[i][j] * spins[i] as f64 * spins[j] as f64;
            }
        }

        e
    }

    /// Convert spin configuration to binary
    pub fn spins_to_binary(&self, spins: &[i8]) -> Vec<bool> {
        spins.iter().map(|&s| s == -1).collect()
    }
}

/// Annealing result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnealingResult {
    /// Best solution found
    pub solution: Vec<bool>,
    /// Best energy/cost
    pub energy: f64,
    /// Number of sweeps performed
    pub sweeps: usize,
    /// Final temperature
    pub final_temperature: f64,
    /// Energy history (sampled)
    pub history: Vec<f64>,
}

/// QUBO solver using simulated annealing
pub struct QuboSolver {
    /// Initial temperature
    pub initial_temp: f64,
    /// Final temperature
    pub final_temp: f64,
    /// Number of sweeps
    pub num_sweeps: usize,
    /// Schedule type
    pub schedule: AnnealingSchedule,
    /// Number of reads (independent runs)
    pub num_reads: usize,
}

/// Annealing schedule
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnnealingSchedule {
    /// Linear temperature decrease
    Linear,
    /// Exponential temperature decrease
    Exponential,
    /// Geometric schedule
    Geometric,
}

impl Default for QuboSolver {
    fn default() -> Self {
        Self {
            initial_temp: 100.0,
            final_temp: 0.01,
            num_sweeps: 1000,
            schedule: AnnealingSchedule::Geometric,
            num_reads: 10,
        }
    }
}

impl QuboSolver {
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure the solver
    pub fn with_temp(mut self, initial: f64, final_t: f64) -> Self {
        self.initial_temp = initial;
        self.final_temp = final_t;
        self
    }

    pub fn with_sweeps(mut self, sweeps: usize) -> Self {
        self.num_sweeps = sweeps;
        self
    }

    pub fn with_reads(mut self, reads: usize) -> Self {
        self.num_reads = reads;
        self
    }

    /// Solve QUBO using simulated annealing
    pub fn solve(&self, qubo: &QuboMatrix) -> SwarmResult<AnnealingResult> {
        let ising = qubo.to_ising();
        let result = self.anneal_ising(&ising)?;

        Ok(AnnealingResult {
            solution: ising.spins_to_binary(&result.0),
            energy: qubo.evaluate(&ising.spins_to_binary(&result.0)),
            sweeps: self.num_sweeps,
            final_temperature: self.final_temp,
            history: result.1,
        })
    }

    /// Solve Ising model using simulated annealing
    pub fn solve_ising(&self, ising: &IsingModel) -> SwarmResult<AnnealingResult> {
        let result = self.anneal_ising(ising)?;

        Ok(AnnealingResult {
            solution: ising.spins_to_binary(&result.0),
            energy: result.2,
            sweeps: self.num_sweeps,
            final_temperature: self.final_temp,
            history: result.1,
        })
    }

    /// Run simulated annealing on Ising model
    fn anneal_ising(&self, ising: &IsingModel) -> SwarmResult<(Vec<i8>, Vec<f64>, f64)> {
        let mut rng = rand::thread_rng();
        let n = ising.size;

        // Run multiple reads and keep best
        let mut best_spins = vec![1i8; n];
        let mut best_energy = f64::MAX;
        let mut best_history = Vec::new();

        for _ in 0..self.num_reads {
            // Initialize random spin configuration
            let mut spins: Vec<i8> = (0..n)
                .map(|_| if rng.gen::<bool>() { 1 } else { -1 })
                .collect();

            let mut energy = ising.energy(&spins);
            let mut history = Vec::new();

            // Temperature schedule
            let temp_ratio = (self.final_temp / self.initial_temp).ln();

            for sweep in 0..self.num_sweeps {
                let progress = sweep as f64 / self.num_sweeps as f64;
                let temp = match self.schedule {
                    AnnealingSchedule::Linear => {
                        self.initial_temp * (1.0 - progress) + self.final_temp * progress
                    }
                    AnnealingSchedule::Exponential => {
                        self.initial_temp * (temp_ratio * progress).exp()
                    }
                    AnnealingSchedule::Geometric => {
                        self.initial_temp * (self.final_temp / self.initial_temp).powf(progress)
                    }
                };

                // Single sweep: try flipping each spin
                for i in 0..n {
                    // Calculate energy change from flipping spin i
                    let mut delta_e = 2.0 * ising.h[i] * spins[i] as f64;
                    for j in 0..n {
                        if i != j {
                            let jij = if i < j { ising.j[i][j] } else { ising.j[j][i] };
                            delta_e += 2.0 * jij * spins[i] as f64 * spins[j] as f64;
                        }
                    }

                    // Metropolis criterion
                    if delta_e < 0.0 || rng.gen::<f64>() < (-delta_e / temp).exp() {
                        spins[i] *= -1;
                        energy += delta_e;
                    }
                }

                // Sample history
                if sweep % 100 == 0 {
                    history.push(energy);
                }
            }

            if energy < best_energy {
                best_energy = energy;
                best_spins = spins;
                best_history = history;
            }
        }

        Ok((best_spins, best_history, best_energy))
    }

    /// Quantum-inspired parallel tempering
    pub fn parallel_tempering(&self, qubo: &QuboMatrix) -> SwarmResult<AnnealingResult> {
        let ising = qubo.to_ising();
        let mut rng = rand::thread_rng();
        let n = ising.size;

        // Number of replicas
        let num_replicas = 8;
        let temps: Vec<f64> = (0..num_replicas)
            .map(|i| {
                self.initial_temp
                    * (self.final_temp / self.initial_temp)
                        .powf(i as f64 / (num_replicas - 1) as f64)
            })
            .collect();

        // Initialize replicas
        let mut replicas: Vec<Vec<i8>> = (0..num_replicas)
            .map(|_| {
                (0..n)
                    .map(|_| if rng.gen::<bool>() { 1 } else { -1 })
                    .collect()
            })
            .collect();
        let mut energies: Vec<f64> = replicas.iter().map(|s| ising.energy(s)).collect();

        let mut best_spins = replicas[0].clone();
        let mut best_energy = energies[0];
        let mut history = Vec::new();

        for sweep in 0..self.num_sweeps {
            // Standard Monte Carlo sweeps for each replica
            for (r, replica) in replicas.iter_mut().enumerate() {
                let temp = temps[r];
                for i in 0..n {
                    let mut delta_e = 2.0 * ising.h[i] * replica[i] as f64;
                    for j in 0..n {
                        if i != j {
                            let jij = if i < j { ising.j[i][j] } else { ising.j[j][i] };
                            delta_e += 2.0 * jij * replica[i] as f64 * replica[j] as f64;
                        }
                    }

                    if delta_e < 0.0 || rng.gen::<f64>() < (-delta_e / temp).exp() {
                        replica[i] *= -1;
                        energies[r] += delta_e;
                    }
                }

                if energies[r] < best_energy {
                    best_energy = energies[r];
                    best_spins = replica.clone();
                }
            }

            // Replica exchange moves
            for i in 0..num_replicas - 1 {
                let delta = (1.0 / temps[i] - 1.0 / temps[i + 1]) * (energies[i] - energies[i + 1]);
                if rng.gen::<f64>() < (-delta).exp().min(1.0) {
                    replicas.swap(i, i + 1);
                    energies.swap(i, i + 1);
                }
            }

            if sweep % 100 == 0 {
                history.push(best_energy);
            }
        }

        Ok(AnnealingResult {
            solution: ising.spins_to_binary(&best_spins),
            energy: qubo.evaluate(&ising.spins_to_binary(&best_spins)),
            sweeps: self.num_sweeps,
            final_temperature: self.final_temp,
            history,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qubo_evaluation() {
        let mut qubo = QuboMatrix::new(3);
        qubo.set(0, 0, 1.0);
        qubo.set(1, 1, 2.0);
        qubo.set(0, 1, -1.0);

        // x = [1, 0, 0]
        assert_eq!(qubo.evaluate(&[true, false, false]), 1.0);

        // x = [1, 1, 0]
        assert_eq!(qubo.evaluate(&[true, true, false]), 1.0 + 2.0 - 1.0);
    }

    #[test]
    fn test_portfolio_qubo() {
        let returns = vec![0.1, 0.15, 0.08];
        let cov = vec![
            vec![0.04, 0.01, 0.02],
            vec![0.01, 0.09, 0.015],
            vec![0.02, 0.015, 0.0625],
        ];

        let qubo = QuboMatrix::portfolio_selection(&returns, &cov, 0.5, 2, 10.0);
        assert_eq!(qubo.size, 3);
    }

    #[test]
    fn test_simulated_annealing() {
        // Simple QUBO: minimize x0 + x1 - 2*x0*x1 (optimal: x0=1, x1=1 or x0=0, x1=0)
        let mut qubo = QuboMatrix::new(2);
        qubo.set(0, 0, 1.0);
        qubo.set(1, 1, 1.0);
        qubo.set(0, 1, -2.0);

        let solver = QuboSolver::default().with_sweeps(500).with_reads(5);

        let result = solver.solve(&qubo).unwrap();

        // Optimal solutions: [0,0] with cost 0, or [1,1] with cost 0
        assert!(result.energy <= 0.0 + 1e-6);
    }

    #[test]
    fn test_parallel_tempering() {
        let mut qubo = QuboMatrix::new(2);
        qubo.set(0, 0, 1.0);
        qubo.set(1, 1, 1.0);
        qubo.set(0, 1, -2.0);

        let solver = QuboSolver::default().with_sweeps(300);
        let result = solver.parallel_tempering(&qubo).unwrap();

        assert!(result.energy <= 0.0 + 1e-6);
    }
}

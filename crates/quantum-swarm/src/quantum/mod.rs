//! Quantum computing layer
//!
//! Provides abstractions for:
//! - Quantum circuit construction
//! - QAOA optimization
//! - VQE ground state finding
//! - QUBO/Ising annealing
//! - Backend routing (local sim vs external QPU)

mod backend;
mod circuit;
mod qaoa;
mod qubo;
mod vqe;

pub use backend::{BackendCapabilities, ExternalQpu, LocalSimulator, QuantumBackend};
pub use circuit::{CircuitBuilder, Gate, QuantumCircuit, Qubit};
pub use qaoa::{QaoaConfig, QaoaOptimizer, QaoaResult};
pub use qubo::{AnnealingResult, IsingModel, QuboMatrix, QuboSolver};
pub use vqe::{Ansatz, VqeConfig, VqeOptimizer, VqeResult};

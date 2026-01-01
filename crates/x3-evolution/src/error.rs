//! Error types for evolution operations

use std::fmt;

/// Evolution error types
#[derive(Debug, Clone)]
pub enum EvolutionError {
    /// Empty population
    EmptyPopulation,

    /// Invalid chromosome
    InvalidChromosome(String),

    /// Invalid bytecode
    InvalidBytecode(String),

    /// Bytecode too large
    BytecodeTooLarge { size: usize, max: usize },

    /// Gene index out of bounds
    GeneIndexOutOfBounds { index: usize, length: usize },

    /// Mutation failed
    MutationFailed(String),

    /// Crossover failed
    CrossoverFailed(String),

    /// Selection failed
    SelectionFailed(String),

    /// Fitness evaluation failed
    FitnessEvaluationFailed(String),

    /// Simulation failed
    SimulationFailed(String),

    /// VM execution error
    VmExecutionError(String),

    /// Configuration error
    ConfigurationError(String),

    /// Population size mismatch
    PopulationSizeMismatch { expected: usize, actual: usize },

    /// Generation limit reached
    GenerationLimitReached(usize),

    /// Stagnation detected
    StagnationDetected { generations: usize },

    /// Constraint violation
    ConstraintViolation(String),

    /// Invalid parameters
    InvalidParameters(String),

    /// Resource exhausted
    ResourceExhausted(String),

    /// Timeout
    Timeout { elapsed_ms: u64, limit_ms: u64 },

    /// IO error
    IoError(String),

    /// Serialization error
    SerializationError(String),
}

impl fmt::Display for EvolutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvolutionError::EmptyPopulation => {
                write!(f, "Empty population")
            }
            EvolutionError::InvalidChromosome(msg) => {
                write!(f, "Invalid chromosome: {}", msg)
            }
            EvolutionError::InvalidBytecode(msg) => {
                write!(f, "Invalid bytecode: {}", msg)
            }
            EvolutionError::BytecodeTooLarge { size, max } => {
                write!(f, "Bytecode too large: {} bytes (max {})", size, max)
            }
            EvolutionError::GeneIndexOutOfBounds { index, length } => {
                write!(f, "Gene index {} out of bounds (length {})", index, length)
            }
            EvolutionError::MutationFailed(msg) => {
                write!(f, "Mutation failed: {}", msg)
            }
            EvolutionError::CrossoverFailed(msg) => {
                write!(f, "Crossover failed: {}", msg)
            }
            EvolutionError::SelectionFailed(msg) => {
                write!(f, "Selection failed: {}", msg)
            }
            EvolutionError::FitnessEvaluationFailed(msg) => {
                write!(f, "Fitness evaluation failed: {}", msg)
            }
            EvolutionError::SimulationFailed(msg) => {
                write!(f, "Simulation failed: {}", msg)
            }
            EvolutionError::VmExecutionError(msg) => {
                write!(f, "VM execution error: {}", msg)
            }
            EvolutionError::ConfigurationError(msg) => {
                write!(f, "Configuration error: {}", msg)
            }
            EvolutionError::PopulationSizeMismatch { expected, actual } => {
                write!(
                    f,
                    "Population size mismatch: expected {}, got {}",
                    expected, actual
                )
            }
            EvolutionError::GenerationLimitReached(gen) => {
                write!(f, "Generation limit reached: {}", gen)
            }
            EvolutionError::StagnationDetected { generations } => {
                write!(f, "Stagnation detected after {} generations", generations)
            }
            EvolutionError::ConstraintViolation(msg) => {
                write!(f, "Constraint violation: {}", msg)
            }
            EvolutionError::InvalidParameters(msg) => {
                write!(f, "Invalid parameters: {}", msg)
            }
            EvolutionError::ResourceExhausted(msg) => {
                write!(f, "Resource exhausted: {}", msg)
            }
            EvolutionError::Timeout {
                elapsed_ms,
                limit_ms,
            } => {
                write!(f, "Timeout: {}ms elapsed, {}ms limit", elapsed_ms, limit_ms)
            }
            EvolutionError::IoError(msg) => {
                write!(f, "IO error: {}", msg)
            }
            EvolutionError::SerializationError(msg) => {
                write!(f, "Serialization error: {}", msg)
            }
        }
    }
}

impl std::error::Error for EvolutionError {}

/// Result type for evolution operations
pub type Result<T> = std::result::Result<T, EvolutionError>;

/// Convert from std::io::Error
impl From<std::io::Error> for EvolutionError {
    fn from(e: std::io::Error) -> Self {
        EvolutionError::IoError(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = EvolutionError::EmptyPopulation;
        assert_eq!(err.to_string(), "Empty population");

        let err = EvolutionError::GeneIndexOutOfBounds {
            index: 5,
            length: 3,
        };
        assert_eq!(err.to_string(), "Gene index 5 out of bounds (length 3)");

        let err = EvolutionError::Timeout {
            elapsed_ms: 1500,
            limit_ms: 1000,
        };
        assert_eq!(err.to_string(), "Timeout: 1500ms elapsed, 1000ms limit");
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let evo_err: EvolutionError = io_err.into();
        assert!(matches!(evo_err, EvolutionError::IoError(_)));
    }
}

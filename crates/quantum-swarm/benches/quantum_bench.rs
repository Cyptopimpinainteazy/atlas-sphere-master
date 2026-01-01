//! Quantum Swarm benchmarks
//!
//! Run with: cargo bench -p quantum-swarm

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use quantum_swarm::quantum::{
    Circuit, CircuitBuilder, QuantumBackend, QuantumGate, StatevectorBackend,
};

/// Benchmark quantum circuit creation
fn bench_circuit_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("circuit_creation");

    for num_qubits in [2, 4, 8, 12].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_qubits),
            num_qubits,
            |b, &qubits| {
                b.iter(|| {
                    let circuit = CircuitBuilder::new(qubits).h(0).cx(0, 1).build();
                    black_box(circuit)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark statevector simulation
fn bench_statevector_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("statevector_simulation");

    for num_qubits in [2, 4, 6, 8].iter() {
        let circuit = CircuitBuilder::new(*num_qubits).h(0).cx(0, 1).build();

        let backend = StatevectorBackend::new(*num_qubits);

        group.bench_with_input(
            BenchmarkId::from_parameter(num_qubits),
            &circuit,
            |b, circuit| {
                b.iter(|| {
                    let result = backend.execute(circuit, 1);
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark Bell state preparation
fn bench_bell_state(c: &mut Criterion) {
    c.bench_function("bell_state_2_qubits", |b| {
        let circuit = CircuitBuilder::new(2).h(0).cx(0, 1).build();

        let backend = StatevectorBackend::new(2);

        b.iter(|| {
            let result = backend.execute(&circuit, 100);
            black_box(result)
        });
    });
}

/// Benchmark GHZ state preparation
fn bench_ghz_state(c: &mut Criterion) {
    let mut group = c.benchmark_group("ghz_state");

    for num_qubits in [3, 4, 5, 6].iter() {
        let mut builder = CircuitBuilder::new(*num_qubits);
        builder = builder.h(0);
        for i in 0..(*num_qubits - 1) {
            builder = builder.cx(i, i + 1);
        }
        let circuit = builder.build();

        let backend = StatevectorBackend::new(*num_qubits);

        group.bench_with_input(
            BenchmarkId::from_parameter(num_qubits),
            &circuit,
            |b, circuit| {
                b.iter(|| {
                    let result = backend.execute(circuit, 100);
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_circuit_creation,
    bench_statevector_simulation,
    bench_bell_state,
    bench_ghz_state,
);

criterion_main!(benches);

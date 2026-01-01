# GPU Swarm

Distributed GPU compute network for the X3 Atlas Sphere blockchain.

## Overview

The GPU Swarm provides a decentralized network of GPU compute nodes that can execute various workloads:

- **X3 Bytecode Execution**: Run X3 virtual machine programs on GPU
- **Mempool Simulation**: Parallel simulation of transaction outcomes
- **Route Optimization**: DEX arbitrage and routing calculations
- **ML Training/Inference**: Distributed AI model training and inference
- **Proof Generation**: Zero-knowledge proof generation
- **Custom Workloads**: Arbitrary GPU compute tasks

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Coordinator                               │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────────────────┐ │
│  │  Scheduler   │  │  Verifier    │  │  Node Registry        │ │
│  │              │  │              │  │                        │ │
│  │ • RoundRobin │  │ • Consensus  │  │ • Node tracking       │ │
│  │ • LeastLoad  │  │ • Re-execute │  │ • Reputation         │ │
│  │ • BestFit    │  │ • Compare    │  │ • Capabilities       │ │
│  │ • Reputation │  │              │  │                        │ │
│  └──────────────┘  └──────────────┘  └────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
                              │
                    P2P Network (libp2p)
                              │
       ┌──────────────────────┼──────────────────────┐
       │                      │                      │
┌──────▼──────┐       ┌───────▼──────┐       ┌──────▼───────┐
│  GPU Node   │       │   GPU Node   │       │   GPU Node   │
│             │       │              │       │              │
│ CUDA/OpenCL │       │    Vulkan    │       │    Metal     │
│ RTX 4090    │       │   RX 7900    │       │   M3 Max     │
└─────────────┘       └──────────────┘       └──────────────┘
```

## Features

- **Multi-Backend GPU Support**: CUDA, OpenCL, Vulkan, Metal, WebGPU
- **Task Scheduling**: Multiple strategies (round-robin, best-fit, reputation-weighted)
- **Verification**: Deterministic re-execution and consensus verification
- **Reputation System**: Track node reliability and performance
- **Economics**: Token rewards for compute provision
- **P2P Networking**: Decentralized peer discovery and communication

## Quick Start

### Running a Coordinator

```bash
# Build the coordinator
cargo build --release -p gpu-swarm --bin swarm-coordinator

# Run with default config
./target/release/swarm-coordinator

# Or with custom config
./target/release/swarm-coordinator --config coordinator-config.toml
```

### Running a Compute Node

```bash
# Build the node
cargo build --release -p gpu-swarm --bin swarm-node

# Run with default config
./target/release/swarm-node

# Or with custom config
./target/release/swarm-node --config node-config.toml
```

### Environment Variables

```bash
# Logging level
export RUST_LOG=info  # trace, debug, info, warn, error

# GPU Backend preference
export GPU_BACKEND=vulkan  # cuda, opencl, vulkan, metal
```

## Configuration

### Node Configuration (`node-config.toml`)

```toml
[node]
keypair_path = "node-keypair.json"
listen_addresses = ["/ip4/0.0.0.0/tcp/9100"]
min_stake = 1000
accepted_task_types = ["X3Bytecode", "MempoolSimulation", "ProofGeneration"]

[scheduler]
strategy = "BestFit"
max_concurrent_tasks = 4
task_timeout_secs = 300

[gpu]
enabled_backends = ["vulkan"]
max_vram_bytes = 0  # 0 = unlimited
```

### Coordinator Configuration (`coordinator-config.toml`)

```toml
[coordinator]
keypair_path = "coordinator-keypair.json"
listen_addresses = ["/ip4/0.0.0.0/tcp/9100"]

[scheduler]
strategy = "ReputationWeighted"
max_queue_size = 10000

[verification]
min_verifiers = 2
verification_timeout_secs = 60

[reputation]
initial_score = 100
success_reward = 1
failure_penalty = 5
```

## Task Types

| Type                | Description                   | GPU Intensity |
| ------------------- | ----------------------------- | ------------- |
| `X3Bytecode`        | Execute X3 VM programs        | High          |
| `MempoolSimulation` | Simulate transaction outcomes | Medium        |
| `RouteOptimization` | DEX routing calculations      | High          |
| `MLTraining`        | Distributed model training    | Very High     |
| `MLInference`       | Model inference               | Medium        |
| `ProofGeneration`   | ZK proof generation           | Very High     |
| `ArbitrageSearch`   | Find arbitrage opportunities  | Medium        |
| `Custom`            | User-defined workloads        | Variable      |

## Verification Flow

1. **Task Submitted**: Client submits task with inputs and expected outputs
2. **Primary Execution**: Scheduler assigns to best-fit node
3. **Result Produced**: Node executes and returns result with execution proof
4. **Verification**: Multiple verifiers re-execute deterministically
5. **Consensus**: If results match, task marked complete; else, penalty applied
6. **Reward Distribution**: Tokens distributed to executor and verifiers

## API

### Submitting Tasks (Programmatic)

```rust
use gpu_swarm::{Task, TaskType, SwarmClient};

let client = SwarmClient::connect("ws://coordinator:9100").await?;

let task = Task::new(
    TaskType::X3Bytecode,
    bytecode_payload,
)
.with_priority(TaskPriority::High)
.with_timeout(Duration::from_secs(300));

let result = client.submit_task(task).await?;
println!("Result: {:?}", result.output);
```

## Development

### Building

```bash
# Build all
cargo build -p gpu-swarm

# Build with specific backend
cargo build -p gpu-swarm --features cuda

# Run tests
cargo test -p gpu-swarm
```

### Module Structure

```
crates/gpu-swarm/
├── src/
│   ├── lib.rs           # Crate entry point
│   ├── config.rs        # Configuration types
│   ├── error.rs         # Error types
│   ├── task.rs          # Task definitions
│   ├── node.rs          # GPU node representation
│   ├── protocol.rs      # P2P message protocol
│   ├── scheduler.rs     # Task scheduling
│   ├── verification.rs  # Execution verification
│   ├── coordinator.rs   # Central coordinator
│   ├── network.rs       # P2P networking
│   └── bin/
│       ├── swarm-node.rs       # Node binary
│       └── swarm-coordinator.rs # Coordinator binary
└── config/
    ├── node-config.toml
    └── coordinator-config.toml
```

## Roadmap

- [x] Core task and node types
- [x] Scheduler with multiple strategies
- [x] Verification pipeline
- [x] Coordinator architecture
- [ ] Full libp2p P2P networking
- [ ] X3-VM integration for bytecode execution
- [ ] CUDA backend implementation
- [ ] Vulkan compute shaders
- [ ] WebGPU support for browsers
- [ ] Economic incentive model
- [ ] Slashing conditions
- [ ] Blockchain integration for rewards

## License

MIT License - See LICENSE file for details.

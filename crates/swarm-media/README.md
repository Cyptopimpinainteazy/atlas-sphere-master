# Swarm Media Orchestration System

A distributed media toolchain for autonomous swarms. Local-first design with GPU scaling, abstracted tool implementations, and complete reproducibility.

## Philosophy

The swarm **never hardcodes** a tool provider. Instead:

- **Abstraction**: Call `ToolAdapter` trait methods, not specific tools
- **Local-first**: Run LLM, image, video, audio locally. Cloud for burst only.
- **GPU scaled**: Contributors provide compute. You provide jobs.
- **Reproducible**: Every output has a hash. Regenerate or iterate anytime.

## Architecture

### Layer 1: Tool Adapter (The Abstraction)

```rust
#[async_trait]
pub trait ToolAdapter: Send + Sync {
    async fn invoke(&self, params: ToolParams) -> Result<JobId, String>;
    async fn get_status(&self, job_id: JobId) -> Result<JobStatus, String>;
    async fn get_result(&self, job_id: JobId) -> Result<ToolResult, String>;
    async fn cancel_job(&self, job_id: JobId) -> Result<(), String>;
}
```

The swarm calls this. It doesn't care what's inside.

### Layer 2: Job Queue & Dispatcher

Central routing:
- Priority queue (Critical > High > Normal > Low)
- Smart node selection (VRAM, latency, capability matching)
- Load balancing across GPU nodes
- Job status tracking and failure recovery

```rust
let job_id = dispatcher.submit_job(job);
let (next_job, node_id) = dispatcher.get_next_job();  // Auto-routes to best node
dispatcher.mark_job_completed(job_id);
```

### Layer 3: GPU Node Manager

GPU nodes register capabilities:
```rust
manager.register_node(GpuNodeCapabilities {
    vram_gb: 24,
    supported_tools: vec![ToolType::ImageGeneration, ToolType::VideoProcessing],
    latency_ms: 450,
    ...
});
```

Keep-alive via heartbeat:
```rust
manager.heartbeat(node_id, available_vram_gb)?;
```

### Layer 4: Tool Implementations

Example: Mock adapter for testing
```rust
pub struct MockAdapter { ... }

#[async_trait]
impl ToolAdapter for MockAdapter {
    fn tool_type(&self) -> ToolType { ToolType::TextGeneration }
    async fn invoke(&self, params) -> Result<JobId, String> { ... }
    ...
}
```

See [TOOL_ADAPTER_IMPLEMENTATION_GUIDE.txt](../TOOL_ADAPTER_IMPLEMENTATION_GUIDE.txt) for detailed patterns for:
- LLM (LLaMA/DeepSeek)
- Images (SDXL + LoRA)
- Video (FFmpeg)
- TTS (XTTS/Piper)
- And more...

## Usage Example

### 1. Set up dispatcher and GPU nodes

```rust
use swarm_media::*;

// Create dispatcher
let mut dispatcher = JobDispatcher::new();

// Register GPU node
let node = GpuNodeCapabilities {
    node_id: Uuid::new_v4(),
    name: "gpu-west-1".into(),
    vram_gb: 24,
    available_vram_gb: 20,
    supported_tools: vec![ToolType::ImageGeneration],
    latency_ms: 450,
    online: true,
    last_heartbeat: Utc::now().timestamp(),
    jobs_completed: 0,
    compute_contributed: 0.0,
};
dispatcher.register_node(node.clone());

// Create node manager
let mut node_manager = GpuNodeManager::default();
node_manager.register_node(node)?;
```

### 2. Submit a job

```rust
let params = ToolParams::new(serde_json::json!({
    "prompt": "A sleek GPU card surrounded by digital art",
    "num_images": 4,
    "style": "professional_product_photography"
}));

let job = MediaJob::new(ToolType::ImageGeneration, params)
    .with_priority(Priority::High)
    .with_min_vram(16);

let job_id = dispatcher.submit_job(job);
```

### 3. Dispatcher routes to best node

```rust
if let Some((job, node_id)) = dispatcher.get_next_job() {
    println!("Assigned {:?} to {}", job.tool_type, node_id);
    dispatcher.mark_job_running(job.job_id, node_id);
}
```

### 4. Get results

```rust
let status = dispatcher.get_assignment(job_id);
// Poll until Completed
// Then retrieve result from adapter
```

## Smart Job Routing

When `dispatcher.get_next_job()` is called:

1. **Find candidates**: Nodes that are online, support the tool, have enough VRAM
2. **Rank by**: Latency (prefer local) → Available VRAM (more headroom)
3. **Assign**: Best candidate gets the job
4. **Fallback**: If no suitable node, wait for next heartbeat

Example: Job needs 16GB, ImageGeneration
- Node 1: 24GB ✓, ImageGen ✓, latency 100ms → WINNER
- Node 2: 12GB ✗ (need 16)
- Node 3: 20GB ✓, VideoOnly ✗ (doesn't support ImageGen)

## Running Tests

```bash
cargo test -p swarm-media --lib
```

Integration tests that exercise the Postgres-backed `PgRepo` will only run when `DATABASE_URL` is set in the environment. To run them locally, start a Postgres instance (Docker):

```bash
# start a local postgres container for tests
docker run --rm -e POSTGRES_PASSWORD=postgres -e POSTGRES_DB=atlas_test -p 5432:5432 -d postgres:15

export DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/atlas_test
cargo test -p swarm-media --lib
```

Currently passing tests:
- ✅ Job creation and priority ordering
- ✅ Priority queue behavior (FIFO within priority)
- ✅ Job assignment lifecycle (Queued → Assigned → Running → Completed)
- ✅ Node registration and heartbeat
- ✅ Node status transitions (Online → Stale → Offline)
- ✅ Network statistics and VRAM tracking
- ✅ Mock adapter behavior

## Modules

- **`tool_adapter`**: Core abstraction (ToolAdapter trait, ToolType, MediaJob, Priority)
- **`job_queue`**: JobDispatcher with smart routing
- **`gpu_nodes`**: GpuNodeManager with registration and heartbeat
- **`adapters`**: Example implementations (MockAdapter, template for real tools)

## What's Next

See [SWARM_MEDIA_ARCHITECTURE.txt](../SWARM_MEDIA_ARCHITECTURE.txt) for:
- Complete system diagram
- Request flow examples
- Fail-over and recovery strategies
- Cost analysis (local vs cloud)
- Implementation checklist for remaining tasks

## Key Design Decisions

### Why abstraction?

Without it, your swarm code would be tied to specific tools. Switching from Stable Diffusion to SDXL-Turbo would require code changes. Adding cloud fallback would require rewrites.

With ToolAdapter, you implement once, swap forever.

### Why BinaryHeap for the queue?

Fast O(log n) insertion and guaranteed priority ordering. Jobs don't starve (lower priority will eventually execute). FIFO within same priority ensures fairness.

### Why heartbeats?

Nodes can go offline unexpectedly (power loss, network failure). Regular heartbeats detect this quickly. Stale nodes are marked offline after 2x heartbeat_timeout, allowing jobs to be reassigned.

### Why content hashing?

Every result can be cached by hash(params + seed + model). Regenerating the exact same output is instant. Variations (different seed) are quick. This is how you iterate fast.

## Monitoring

Track these metrics:

```rust
let stats = dispatcher.get_stats();
println!("Queue depth: {}", dispatcher.queue_length());
println!("Nodes online: {}", dispatcher.node_count());
```

For production, integrate with:
- Prometheus (metrics export)
- Grafana (dashboards)
- CloudWatch / DataDog (cloud monitoring)

## Dependencies

Minimal:
- `tokio`: async runtime
- `async-trait`: for trait definitions
- `serde`: serialization
- `sqlx`: database (future: persistent queue)
- `uuid`: job/node IDs
- `chrono`: timestamps
- `thiserror`: error handling

## Future Enhancements

- Persistent job queue (PostgreSQL backend)
- Job retry logic with exponential backoff
- Reputation system for nodes (penalize failures)
- Batch job submission and grouping
- Cost tracking and billing per node
- Web RPC API for remote job submission
- Dashboard for monitoring
- AI agent support (agents call swarm as a service)

## References

- Architecture: [SWARM_MEDIA_ARCHITECTURE.txt](../SWARM_MEDIA_ARCHITECTURE.txt)
- Implementation Guide: [TOOL_ADAPTER_IMPLEMENTATION_GUIDE.txt](../TOOL_ADAPTER_IMPLEMENTATION_GUIDE.txt)
- Atlas Sphere: [Copilot Instructions](../.github/copilot-instructions.md)

## License

MIT

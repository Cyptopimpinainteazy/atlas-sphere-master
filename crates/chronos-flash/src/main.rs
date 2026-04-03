use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use chronos_flash::config::ChainConfig;
use chronos_flash::oracle::ChronosOracleBuilder;
use chronos_flash::timewarp::Signer;
use chronos_flash::{ChronosConfig, ChronosResult};

struct DevSigner;

#[async_trait::async_trait]
impl Signer for DevSigner {
    async fn sign(
        &self,
        chain_id: chronos_flash::types::ChainId,
        _data: &[u8],
    ) -> ChronosResult<chronos_flash::types::Signature> {
        Ok(chronos_flash::types::Signature {
            chain_id,
            signer: self.address(),
            signature: vec![0u8; 65],
            recovery_id: 0,
        })
    }

    fn address(&self) -> chronos_flash::types::Address {
        [0u8; 32]
    }
}

fn parse_chains_env() -> Vec<String> {
    std::env::var("CHRONOS_CHAINS")
        .ok()
        .unwrap_or_else(|| "ethereum,solana".to_string())
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

fn read_u64_env(var: &str) -> Option<u64> {
    std::env::var(var)
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
}

#[derive(Default)]
struct CliOverrides {
    chains: Option<Vec<String>>,
    scan_interval_ms: Option<u64>,
    duration_secs: Option<u64>,
    health_addr: Option<SocketAddr>,
}

fn parse_cli_overrides() -> anyhow::Result<CliOverrides> {
    let mut out = CliOverrides::default();
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!(
                    "ChronosFlash daemon\n\n  --chains <csv>            Override CHRONOS_CHAINS (e.g. ethereum,solana)\n  --scan-interval-ms <ms>  Override CHRONOS_SCAN_INTERVAL_MS\n  --duration-secs <secs>  Run for N seconds then exit\n  --health-addr <addr>    Serve /healthz on addr (e.g. 127.0.0.1:9191)\n\nEnv defaults are loaded from crates/chronos-flash/.env if present.\nHealth can also be enabled via CHRONOS_HEALTH_ADDR."
                );
                return Ok(out);
            }
            "--chains" => {
                let v = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--chains requires a value"))?;
                out.chains = Some(
                    v.split(',')
                        .map(|s| s.trim().to_lowercase())
                        .filter(|s| !s.is_empty())
                        .collect(),
                );
            }
            "--scan-interval-ms" => {
                let v = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--scan-interval-ms requires a value"))?;
                out.scan_interval_ms = Some(v.trim().parse::<u64>().map_err(|e| {
                    anyhow::anyhow!("invalid --scan-interval-ms '{}': {}", v.trim(), e)
                })?);
            }
            "--duration-secs" => {
                let v = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--duration-secs requires a value"))?;
                out.duration_secs = Some(v.trim().parse::<u64>().map_err(|e| {
                    anyhow::anyhow!("invalid --duration-secs '{}': {}", v.trim(), e)
                })?);
            }
            "--health-addr" => {
                let v = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--health-addr requires a value"))?;
                out.health_addr =
                    Some(v.trim().parse::<SocketAddr>().map_err(|e| {
                        anyhow::anyhow!("invalid --health-addr '{}': {}", v.trim(), e)
                    })?);
            }
            _ if arg.starts_with("--chains=") => {
                let v = arg.splitn(2, '=').nth(1).unwrap_or("");
                out.chains = Some(
                    v.split(',')
                        .map(|s| s.trim().to_lowercase())
                        .filter(|s| !s.is_empty())
                        .collect(),
                );
            }
            _ if arg.starts_with("--scan-interval-ms=") => {
                let v = arg.splitn(2, '=').nth(1).unwrap_or("");
                out.scan_interval_ms = Some(v.trim().parse::<u64>().map_err(|e| {
                    anyhow::anyhow!("invalid --scan-interval-ms '{}': {}", v.trim(), e)
                })?);
            }
            _ if arg.starts_with("--duration-secs=") => {
                let v = arg.splitn(2, '=').nth(1).unwrap_or("");
                out.duration_secs = Some(v.trim().parse::<u64>().map_err(|e| {
                    anyhow::anyhow!("invalid --duration-secs '{}': {}", v.trim(), e)
                })?);
            }
            _ if arg.starts_with("--health-addr=") => {
                let v = arg.splitn(2, '=').nth(1).unwrap_or("");
                out.health_addr =
                    Some(v.trim().parse::<SocketAddr>().map_err(|e| {
                        anyhow::anyhow!("invalid --health-addr '{}': {}", v.trim(), e)
                    })?);
            }
            _ => {
                return Err(anyhow::anyhow!("unknown arg: {} (try --help)", arg));
            }
        }
    }

    Ok(out)
}

fn read_socket_addr_env(var: &str) -> Option<SocketAddr> {
    std::env::var(var)
        .ok()
        .and_then(|v| v.trim().parse::<SocketAddr>().ok())
}

async fn serve_healthz(
    addr: SocketAddr,
    statuses: Arc<tokio::sync::RwLock<HashMap<u64, chronos_flash::types::ChainStatus>>>,
    mempool_stats: Arc<tokio::sync::RwLock<chronos_flash::types::MempoolStats>>,
    oracle_metrics: Arc<tokio::sync::RwLock<chronos_flash::types::OracleMetrics>>,
) -> anyhow::Result<()> {
    use hyper::service::{make_service_fn, service_fn};
    use hyper::{Body, Request, Response, Server, StatusCode};

    let make = make_service_fn(move |_conn| {
        let statuses = statuses.clone();
        let mempool_stats = mempool_stats.clone();
        let oracle_metrics = oracle_metrics.clone();

        async move {
            Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| {
                let statuses = statuses.clone();
                let mempool_stats = mempool_stats.clone();
                let oracle_metrics = oracle_metrics.clone();

                async move {
                    let path = req.uri().path();
                    if path != "/healthz" {
                        let mut resp = Response::new(Body::from("not found"));
                        *resp.status_mut() = StatusCode::NOT_FOUND;
                        return Ok::<_, hyper::Error>(resp);
                    }

                    let now_ms = chrono::Utc::now().timestamp_millis() as u64;
                    let statuses_map = statuses.read().await;
                    let mut chains: Vec<chronos_flash::types::ChainStatus> =
                        statuses_map.values().cloned().collect();
                    chains.sort_by_key(|c| c.chain_id);

                    let ok = !chains.is_empty() && chains.iter().all(|c| c.is_connected);

                    let mempool = mempool_stats.read().await.clone();
                    let metrics = oracle_metrics.read().await.clone();

                    let body = serde_json::json!({
                        "ok": ok,
                        "ts_ms": now_ms,
                        "chains": chains,
                        "mempool": mempool,
                        "oracle": metrics,
                    });

                    let mut resp = Response::new(Body::from(body.to_string()));
                    resp.headers_mut().insert(
                        hyper::header::CONTENT_TYPE,
                        hyper::header::HeaderValue::from_static("application/json"),
                    );
                    Ok::<_, hyper::Error>(resp)
                }
            }))
        }
    });

    log::info!("Health endpoint listening on http://{}{}", addr, "/healthz");
    let server = Server::bind(&addr).serve(make);
    server.await.map_err(|e| anyhow::anyhow!(e))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load local env (optional). This will not affect production deployments.
    let _ = dotenvy::dotenv();

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    let cli = parse_cli_overrides()?;
    if std::env::args().any(|a| a == "--help" || a == "-h") {
        return Ok(());
    }

    let chains = cli.chains.unwrap_or_else(parse_chains_env);
    log::info!("Starting ChronosFlash with chains: {:?}", chains);

    let mut config = ChronosConfig::default();

    // Slower default scan interval for real RPC usage (the library default is extremely aggressive).
    config.mempool.scan_interval_ms = cli
        .scan_interval_ms
        .or_else(|| read_u64_env("CHRONOS_SCAN_INTERVAL_MS"))
        .unwrap_or(250);

    let mut map: HashMap<u64, ChainConfig> = HashMap::new();
    for c in chains {
        match c.as_str() {
            "ethereum" | "eth" => {
                let cfg = ChainConfig::ethereum();
                map.insert(cfg.chain_id, cfg);
            }
            "polygon" => {
                let cfg = ChainConfig::polygon();
                map.insert(cfg.chain_id, cfg);
            }
            "arbitrum" | "arb" => {
                let cfg = ChainConfig::arbitrum();
                map.insert(cfg.chain_id, cfg);
            }
            "solana" | "sol" => {
                let cfg = ChainConfig::solana();
                map.insert(cfg.chain_id, cfg);
            }
            "atlas" | "atlas-sphere" | "atlas_sphere" => {
                let cfg = ChainConfig::atlas_sphere();
                map.insert(cfg.chain_id, cfg);
            }
            other => {
                log::warn!("Unknown chain '{}'; skipping", other);
            }
        }
    }

    config.chains = map;

    let signer: Arc<dyn Signer + Send + Sync> = Arc::new(DevSigner);

    let mut oracle = ChronosOracleBuilder::new()
        .with_config(config)
        .with_signer(signer)
        .build()
        .await?;

    let health_addr = cli
        .health_addr
        .or_else(|| read_socket_addr_env("CHRONOS_HEALTH_ADDR"));
    if let Some(addr) = health_addr {
        let statuses = oracle.shared_chain_statuses();
        let mempool_stats = oracle.shared_mempool_stats();
        let oracle_metrics = oracle.shared_oracle_metrics();
        tokio::spawn(async move {
            if let Err(e) = serve_healthz(addr, statuses, mempool_stats, oracle_metrics).await {
                log::error!("Health server error: {}", e);
            }
        });
    }

    log::info!("ChronosFlash is live. Press Ctrl+C to stop.");

    match cli.duration_secs {
        Some(secs) if secs > 0 => {
            log::info!("Run duration set to {}s", secs);
            tokio::select! {
                r = oracle.start() => {
                    if let Err(e) = r {
                        log::error!("Oracle exited with error: {}", e);
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    log::info!("Ctrl+C received; exiting.");
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(secs)) => {
                    log::info!("Duration elapsed; exiting.");
                }
            }
        }
        _ => {
            tokio::select! {
                r = oracle.start() => {
                    if let Err(e) = r {
                        log::error!("Oracle exited with error: {}", e);
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    log::info!("Ctrl+C received; exiting.");
                }
            }
        }
    }

    Ok(())
}

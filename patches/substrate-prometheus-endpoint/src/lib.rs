// This file is part of Substrate.
//
// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0
//
// Patched for compatibility with prometheus 0.13 / protobuf RepeatedField API

mod sourced;

pub use sourced::*;

use hyper::{
    http::StatusCode,
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use prometheus::{core::Collector, Encoder, TextEncoder};
use std::net::SocketAddr;

// Re-export prometheus for downstream crates (e.g., sc-proposer-metrics)
pub use prometheus;

/// PrometheusError alias for compatibility
pub type PrometheusError = prometheus::Error;

/// Start a prometheus metrics server.
/// Returns a Future that completes when the server encounters an error.
/// This matches the original substrate-prometheus-endpoint API.
pub async fn init_prometheus(
    prometheus_addr: SocketAddr,
    registry: prometheus::Registry,
) -> Result<(), Error> {
    run_server(prometheus_addr, registry).await
}

async fn run_server(addr: SocketAddr, registry: prometheus::Registry) -> Result<(), Error> {
    let make_svc = make_service_fn(move |_| {
        let registry = registry.clone();
        async move {
            Ok::<_, hyper::Error>(service_fn(move |_req: Request<Body>| {
                let registry = registry.clone();
                async move {
                    let metric_families = registry.gather();
                    let encoder = TextEncoder::new();
                    let mut buffer = vec![];
                    encoder.encode(&metric_families, &mut buffer).unwrap();

                    Response::builder()
                        .status(StatusCode::OK)
                        .header("Content-Type", encoder.format_type())
                        .body(Body::from(buffer))
                }
            }))
        }
    });

    Server::bind(&addr)
        .serve(make_svc)
        .await
        .map_err(|_| Error::Internal)
}

/// Errors that can occur during prometheus initialization.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to start the prometheus server.
    #[error("Prometheus error: {0}")]
    Prometheus(#[from] prometheus::Error),
    /// Port already in use.
    #[error("Port already in use")]
    PortInUse(std::io::Error),
    /// Internal error.
    #[error("Internal error")]
    Internal,
}

// Implement From<Error> for prometheus::Error to allow ? conversion in upstream code
impl From<Error> for prometheus::Error {
    fn from(e: Error) -> Self {
        match e {
            Error::Prometheus(p) => p,
            Error::PortInUse(io) => prometheus::Error::Msg(format!("Port in use: {}", io)),
            Error::Internal => prometheus::Error::Msg("Internal prometheus endpoint error".into()),
        }
    }
}

/// Register a prometheus collector.
pub fn register<C: Collector + Clone + 'static>(
    collector: C,
    registry: &prometheus::Registry,
) -> prometheus::Result<C> {
    registry.register(Box::new(collector.clone()))?;
    Ok(collector)
}

/// Re-export prometheus types for convenience.
pub use prometheus::{
    core::{
        AtomicF64 as F64, AtomicU64 as U64, GenericCounter, GenericCounterVec, GenericGauge,
        GenericGaugeVec,
    },
    exponential_buckets, linear_buckets, Histogram, HistogramOpts, HistogramVec, Opts, Registry,
};

/// Counter type alias.
pub type Counter<T> = GenericCounter<T>;
/// CounterVec type alias.
pub type CounterVec<T> = GenericCounterVec<T>;
/// Gauge type alias.
pub type Gauge<T> = GenericGauge<T>;
/// GaugeVec type alias.
pub type GaugeVec<T> = GenericGaugeVec<T>;

/// Creates a new prometheus registry.
pub fn new_registry() -> Registry {
    prometheus::Registry::new()
}

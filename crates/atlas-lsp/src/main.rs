//! Atlas LSP - Language Server for Atlas Sphere development.
//!
//! Provides IDE features for:
//! - `.comit` transaction definition files
//! - Substrate pallet development
//! - Cross-VM bridge configuration

mod backend;
mod completion;
mod diagnostics;
mod document;
mod hover;
mod semantic;

use backend::Backend;
use tower_lsp::{LspService, Server};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

fn init_logging() {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();
}

#[tokio::main]
async fn main() {
    init_logging();
    info!("Atlas LSP starting...");

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);

    Server::new(stdin, stdout, socket).serve(service).await;
}

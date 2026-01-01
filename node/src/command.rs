use crate::{
    cli::{Cli, Commands},
    service,
};
use clap::Parser;
use log::{error, info};
use sc_cli::{Error as CliError, Result as CliResult, SubstrateCli};

/// Run the Atlas Sphere node with the specified configuration and commands
pub fn run() -> CliResult<()> {
    let cli = Cli::parse();

    match &cli.subcommand {
        Some(Commands::BuildSpec(cmd)) => {
            let runner = cli.create_runner(cmd).map_err(|e| {
                error!("Failed to initialize runner for `build-spec`: {e}");
                e
            })?;

            runner.sync_run(|config| {
                info!("Building Atlas Sphere chain specification (raw: {})", cmd.raw);
                cmd.run(config.chain_spec, config.network).map_err(|e| {
                    error!("`build-spec` command failed: {e}");
                    e
                })
            })
        }
        Some(Commands::CheckBlock(cmd)) => {
            let runner = cli.create_runner(cmd).map_err(|e| {
                error!("Failed to initialize runner for `check-block`: {e}");
                e
            })?;

            runner.async_run(|config| {
                info!("Checking blocks with the current runtime logic");
                let partial = service::new_partial(&config).map_err(|e| {
                    error!("Unable to build partial components for `check-block`: {e}");
                    CliError::Service(e)
                })?;

                let sc_service::PartialComponents {
                    client,
                    task_manager,
                    import_queue,
                    ..
                } = partial;

                Ok((cmd.run(client, import_queue), task_manager))
            })
        }
        Some(Commands::ExportBlocks(cmd)) => {
            let runner = cli.create_runner(cmd).map_err(|e| {
                error!("Failed to initialize runner for `export-blocks`: {e}");
                e
            })?;

            runner.async_run(|config| {
                info!("Exporting blocks to file");
                let partial = service::new_partial(&config).map_err(|e| {
                    error!("Unable to build partial components for `export-blocks`: {e}");
                    CliError::Service(e)
                })?;

                let sc_service::PartialComponents {
                    client,
                    task_manager,
                    ..
                } = partial;

                Ok((cmd.run(client, config.database), task_manager))
            })
        }
        Some(Commands::ExportState(cmd)) => {
            let runner = cli.create_runner(cmd).map_err(|e| {
                error!("Failed to initialize runner for `export-state`: {e}");
                e
            })?;

            runner.async_run(|config| {
                info!("Exporting full runtime state snapshot");
                let partial = service::new_partial(&config).map_err(|e| {
                    error!("Unable to build partial components for `export-state`: {e}");
                    CliError::Service(e)
                })?;

                let sc_service::PartialComponents {
                    client,
                    task_manager,
                    ..
                } = partial;

                Ok((cmd.run(client, config.chain_spec), task_manager))
            })
        }
        Some(Commands::ImportBlocks(cmd)) => {
            let runner = cli.create_runner(cmd).map_err(|e| {
                error!("Failed to initialize runner for `import-blocks`: {e}");
                e
            })?;

            runner.async_run(|config| {
                info!("Importing blocks into the local database");
                let partial = service::new_partial(&config).map_err(|e| {
                    error!("Unable to build partial components for `import-blocks`: {e}");
                    CliError::Service(e)
                })?;

                let sc_service::PartialComponents {
                    client,
                    task_manager,
                    import_queue,
                    ..
                } = partial;

                Ok((cmd.run(client, import_queue), task_manager))
            })
        }
        Some(Commands::PurgeChain(cmd)) => {
            let runner = cli.create_runner(cmd).map_err(|e| {
                error!("Failed to initialize runner for `purge-chain`: {e}");
                e
            })?;

            runner.sync_run(|config| {
                info!("Purging local database for Atlas Sphere");
                cmd.run(config.database).map_err(|e| {
                    error!("`purge-chain` command failed: {e}");
                    e
                })
            })
        }
        Some(Commands::Revert(cmd)) => {
            let runner = cli.create_runner(cmd).map_err(|e| {
                error!("Failed to initialize runner for `revert`: {e}");
                e
            })?;

            runner.async_run(|config| {
                info!("Reverting chain state by {:?} blocks", cmd.num);
                let partial = service::new_partial(&config).map_err(|e| {
                    error!("Unable to build partial components for `revert`: {e}");
                    CliError::Service(e)
                })?;

                let sc_service::PartialComponents {
                    client,
                    task_manager,
                    backend,
                    ..
                } = partial;

                Ok((cmd.run(client, backend, None), task_manager))
            })
        }
        #[cfg(feature = "runtime-benchmarks")]
        Some(Commands::Benchmark(cmd)) => {
            let runner = cli.create_runner(cmd).map_err(|e| {
                error!("Failed to initialize runner for `benchmark`: {e}");
                e
            })?;

            runner.sync_run(|config| {
                info!("Executing runtime benchmarks");
                cmd.run::<Block, AtlasSphereExecutorDispatch>(config).map_err(|e| {
                    error!("`benchmark` command failed: {e}");
                    e
                })
            })
        }
        #[cfg(feature = "try-runtime")]
        Some(Commands::TryRuntime(_cmd)) => {
            error!("`try-runtime` is not yet supported for Atlas Sphere");
            Err(CliError::Other(
                "try-runtime subcommand is not yet supported for Atlas Sphere".into(),
            ))
        }
        None => {
            let runner = cli.create_runner(&cli.run).map_err(|e| {
                error!("Failed to initialize runner for node execution: {e}");
                e
            })?;

            runner.run_node_until_exit(|config| async move {
                let role = config.role.clone();
                info!("Starting Atlas Sphere node as {:?}", role);
                service::new_full(config).map_err(|e| {
                    error!("Atlas Sphere node terminated with an error: {e}");
                    CliError::Service(e)
                })
            })
        }
    }
}
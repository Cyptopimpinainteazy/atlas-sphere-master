/// Atlas Sphere node service module
///
/// Provides node initialization, partial components, and full service setup with:
/// - Aura (Authority Round) block authoring consensus
/// - GRANDPA finality gadget
/// - libp2p networking with peer discovery
/// - Proper block import queue with consensus verification

use atlas_sphere_runtime::{opaque::Block, RuntimeApi};
use sc_client_api::BlockBackend;
use sc_consensus_aura::{ImportQueueParams, SlotProportion, StartAuraParams};
use sc_consensus_grandpa::SharedVoterState;
pub use sc_executor::NativeElseWasmExecutor;
use sc_service::{Configuration, Error as ServiceError, PartialComponents, TaskManager};
use sc_telemetry::{Telemetry, TelemetryWorker};
use sp_consensus_aura::sr25519::AuthorityPair as AuraPair;
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;

/// Atlas Sphere native executor implementation
pub struct AtlasSphereExecutorDispatch;

impl sc_executor::NativeExecutionDispatch for AtlasSphereExecutorDispatch {
	/// Only enable the benchmarking host functions when we actually want to benchmark.
	#[cfg(feature = "runtime-benchmarks")]
	type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;
	/// Otherwise we only use the default Substrate host functions.
	#[cfg(not(feature = "runtime-benchmarks"))]
	type ExtendHostFunctions = ();

	fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
		atlas_sphere_runtime::api::dispatch(method, data)
	}

	fn native_version() -> sc_executor::NativeVersion {
		atlas_sphere_runtime::native_version()
	}
}

/// Executor for Atlas Sphere - native where possible, otherwise WASM.
pub type Executor = NativeElseWasmExecutor<AtlasSphereExecutorDispatch>;

/// Full client type alias
pub type FullClient = sc_service::TFullClient<Block, RuntimeApi, Executor>;

/// Full backend type alias
pub type FullBackend = sc_service::TFullBackend<Block>;

/// Type alias for select chain implementation
pub type SelectChain = sc_consensus::LongestChain<FullBackend, Block>;

/// Create partial components for Atlas Sphere node
///
/// Returns the common components needed by various subcommands (benchmarking, export, etc.)
pub fn new_partial(
	config: &Configuration,
) -> Result<
	PartialComponents<
		FullClient,
		FullBackend,
		SelectChain,
		sc_consensus::DefaultImportQueue<Block>,
		sc_transaction_pool::TransactionPoolHandle<Block, FullClient>,
		(
			sc_consensus_grandpa::GrandpaBlockImport<FullBackend, Block, FullClient, SelectChain>,
			sc_consensus_grandpa::LinkHalf<Block, FullClient, SelectChain>,
			Option<Telemetry>,
		),
	>,
	ServiceError,
> {
	// Set up telemetry if endpoints are configured
	let telemetry = config
		.telemetry_endpoints
		.clone()
		.filter(|x| !x.is_empty())
		.map(|endpoints| -> Result<_, sc_telemetry::Error> {
			let worker = TelemetryWorker::new(16)?;
			let telemetry = worker.handle().new_telemetry(endpoints);
			Ok((worker, telemetry))
		})
		.transpose()?;

	// Create executor - uses native executor when possible, otherwise wasm.
	let executor = sc_service::new_native_or_wasm_executor(config);

	// Build partial components
	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<Block, RuntimeApi, _>(
			config,
			telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
		)?;

	let client = Arc::new(client);

	let telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager
			.spawn_handle()
			.spawn("telemetry", None, worker.run());
		telemetry
	});

	// Select chain implementation (longest chain rule)
	let select_chain = sc_consensus::LongestChain::new(backend.clone());

	// Create transaction pool
	let transaction_pool = sc_transaction_pool::Builder::new(
		task_manager.spawn_essential_handle(),
		client.clone(),
		config.role.is_authority().into(),
	)
	.with_options(config.transaction_pool.clone())
	.with_prometheus(config.prometheus_registry())
	.build();

	let transaction_pool = Arc::new(transaction_pool);

	// Create GRANDPA block import wrapper
	let (grandpa_block_import, grandpa_link) = sc_consensus_grandpa::block_import(
		client.clone(),
		512,
		&client,
		select_chain.clone(),
		telemetry.as_ref().map(|x| x.handle()),
	)?;

	// Create Aura import queue with proper block verification
	let slot_duration = sc_consensus_aura::slot_duration(&*client)?;

	let import_queue = sc_consensus_aura::import_queue::<AuraPair, _, _, _, _, _>(
		ImportQueueParams {
			block_import: grandpa_block_import.clone(),
			justification_import: Some(Box::new(grandpa_block_import.clone())),
			client: client.clone(),
			create_inherent_data_providers: move |_, ()| async move {
				let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

				let slot =
					sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
						*timestamp,
						slot_duration,
					);

				Ok((slot, timestamp))
			},
			spawner: &task_manager.spawn_essential_handle(),
			registry: config.prometheus_registry(),
			check_for_equivocation: Default::default(),
			telemetry: telemetry.as_ref().map(|x| x.handle()),
			compatibility_mode: Default::default(),
		},
	)?;

	Ok(PartialComponents {
		client,
		backend,
		task_manager,
		keystore_container,
		select_chain,
		import_queue,
		transaction_pool,
		other: (grandpa_block_import, grandpa_link, telemetry),
	})
}

/// Start a new Atlas Sphere full node with complete consensus and networking
pub fn new_full(config: Configuration) -> Result<TaskManager, ServiceError> {
	let sc_service::PartialComponents {
		client,
		backend,
		mut task_manager,
		keystore_container,
		select_chain,
		import_queue,
		transaction_pool,
		other: (grandpa_block_import, grandpa_link, mut telemetry),
	} = new_partial(&config)?;

	let metrics = sc_network::service::NotificationMetrics::new(config.prometheus_registry());
	let mut net_config = sc_network::config::FullNetworkConfiguration::<
		Block,
		<Block as BlockT>::Hash,
		sc_network::Litep2pNetworkBackend,
	>::new(&config.network, config.prometheus_registry().cloned());

	let genesis_hash = client.block_hash(0)?.expect("Genesis block exists; qed");
	let peer_store_handle = net_config.peer_store_handle();

	let grandpa_protocol_name = sc_consensus_grandpa::protocol_standard_name(
		&genesis_hash,
		&config.chain_spec,
	);

	let (grandpa_protocol_config, grandpa_notification_service) = sc_consensus_grandpa::grandpa_peers_set_config::<
		Block,
		sc_network::Litep2pNetworkBackend,
	>(
		grandpa_protocol_name.clone(),
		sc_network::service::NotificationMetrics::new(config.prometheus_registry()),
		Arc::clone(&peer_store_handle),
	);

	net_config.add_notification_protocol(grandpa_protocol_config);

	let warp_sync = Arc::new(sc_consensus_grandpa::warp_proof::NetworkProvider::new(
		backend.clone(),
		grandpa_link.shared_authority_set().clone(),
		Vec::default(),
	));

	// Build networking service
	let (network, system_rpc_tx, tx_handler_controller, network_starter, sync_service) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
			net_config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			block_announce_validator_builder: None,
			warp_sync_config: Some(sc_service::WarpSyncConfig::WithProvider(warp_sync)),
			block_relay: None,
			metrics,
		})?;

	let rpc_extensions_builder = {
		let client = client.clone();
		let pool = transaction_pool.clone();

		Box::new(move |_| {
			let deps = crate::rpc::FullDeps {
				client: client.clone(),
				pool: pool.clone(),
				deny_unsafe: sc_rpc_api::DenyUnsafe::No,
			};
			crate::rpc::create_full(deps).map_err(Into::into)
		})
	};

	let role = config.role.clone();
	let force_authoring = config.force_authoring;
	let backoff_authoring_blocks: Option<()> = None;
	let name = config.network.node_name.clone();
	let enable_grandpa = !config.disable_grandpa;
	let prometheus_registry = config.prometheus_registry().cloned();
	let role_for_grandpa = role.clone();
	let chain_name = config.chain_spec.name().to_string();
	let node_name = config.network.node_name.clone();

	let _rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		network: network.clone(),
		client: client.clone(),
		keystore: keystore_container.keystore(),
		task_manager: &mut task_manager,
		transaction_pool: transaction_pool.clone(),
		rpc_builder: rpc_extensions_builder,
		backend: backend.clone(),
		system_rpc_tx,
		tx_handler_controller,
		sync_service: sync_service.clone(),
		config,
		telemetry: telemetry.as_mut(),
	})?;


	// Start Aura block authoring if this is an authority node
	if role.is_authority() {
		let proposer_factory = sc_basic_authorship::ProposerFactory::new(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool.clone(),
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|x: &Telemetry| x.handle()),
		);

		let slot_duration = sc_consensus_aura::slot_duration(&*client)?;

		let aura = sc_consensus_aura::start_aura::<AuraPair, _, _, _, _, _, _, _, _, _, _>(
			StartAuraParams {
				slot_duration,
				client: client.clone(),
				select_chain,
				block_import: grandpa_block_import,
				proposer_factory,
				create_inherent_data_providers: move |_, ()| async move {
					let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

					let slot =
						sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
							*timestamp,
							slot_duration,
						);

					Ok((slot, timestamp))
				},
				force_authoring,
				backoff_authoring_blocks,
				keystore: keystore_container.keystore(),
				sync_oracle: sync_service.clone(),
				justification_sync_link: sync_service.clone(),
				block_proposal_slot_portion: SlotProportion::new(2f32 / 3f32),
				max_block_proposal_slot_portion: None,
				telemetry: telemetry.as_ref().map(|x: &Telemetry| x.handle()),
				compatibility_mode: Default::default(),
			},
		)?;

		task_manager.spawn_essential_handle().spawn_blocking(
			"aura",
			Some("block-authoring"),
			aura,
		);
	}

	// Start GRANDPA finality gadget
	if enable_grandpa {
		let grandpa_config = sc_consensus_grandpa::Config {
			gossip_duration: std::time::Duration::from_millis(333),
			justification_generation_period: 512,
			name: Some(name),
			observer_enabled: false,
			keystore: Some(keystore_container.keystore()),
			local_role: role_for_grandpa,
			telemetry: telemetry.as_ref().map(|x: &Telemetry| x.handle()),
			protocol_name: grandpa_protocol_name.clone(),
		};

		// Create GRANDPA parameters with offchain transaction pool
		let offchain_tx_pool_factory =
			sc_transaction_pool_api::OffchainTransactionPoolFactory::new(transaction_pool.clone());

		let grandpa_params = sc_consensus_grandpa::GrandpaParams {
			config: grandpa_config,
			link: grandpa_link,
			network,
			sync: Arc::new(sync_service.clone()),
			voting_rule: sc_consensus_grandpa::VotingRulesBuilder::default().build(),
			prometheus_registry,
			shared_voter_state: SharedVoterState::empty(),
			telemetry: telemetry.as_ref().map(|x: &Telemetry| x.handle()),
			offchain_tx_pool_factory,
			notification_service: grandpa_notification_service,
		};

		task_manager.spawn_essential_handle().spawn_blocking(
			"grandpa-voter",
			None,
			sc_consensus_grandpa::run_grandpa_voter(grandpa_params)?,
		);
	}

	// Start the network
	network_starter.start_network();

	log::info!("✨ Atlas Sphere node started successfully");
	log::info!("🔗 Network: {}", chain_name);
	log::info!("👤 Node name: {}", node_name);
	log::info!("📋 Role: {:?}", role);

	Ok(task_manager)
}

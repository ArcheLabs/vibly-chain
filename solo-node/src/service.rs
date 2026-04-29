use std::{sync::Arc, time::Duration};

use polkadot_sdk::*;
use prometheus_endpoint::Registry;
use sc_consensus::{DefaultImportQueue, LongestChain};
use sc_consensus_aura::{ImportQueueParams, SlotProportion, StartAuraParams};
use sc_executor::{HeapAllocStrategy, WasmExecutor, DEFAULT_HEAP_ALLOC_STRATEGY};
use sc_network::{NetworkBackend, NetworkWorker};
use sc_service::{Configuration, PartialComponents, TFullBackend, TFullClient, TaskManager};
use sc_telemetry::{Telemetry, TelemetryHandle, TelemetryWorker, TelemetryWorkerHandle};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sp_consensus_aura::sr25519::AuthorityPair as AuraPair;
use sp_keystore::KeystorePtr;
use vibly_solo_runtime::{
    opaque::{Block, Hash},
    RuntimeApi,
};

type FullClient = TFullClient<Block, RuntimeApi, WasmExecutor<sp_io::SubstrateHostFunctions>>;
type FullBackend = TFullBackend<Block>;
type FullSelectChain = LongestChain<FullBackend, Block>;
type GrandpaBlockImport =
    sc_consensus_grandpa::GrandpaBlockImport<FullBackend, Block, FullClient, FullSelectChain>;

pub type Service = PartialComponents<
    FullClient,
    FullBackend,
    FullSelectChain,
    DefaultImportQueue<Block>,
    sc_transaction_pool::TransactionPoolHandle<Block, FullClient>,
    (
        GrandpaBlockImport,
        sc_consensus_grandpa::LinkHalf<Block, FullClient, FullSelectChain>,
        Option<Telemetry>,
        Option<TelemetryWorkerHandle>,
    ),
>;

fn telemetry(
    config: &Configuration,
) -> Result<(Option<(TelemetryWorker, Telemetry)>, Option<TelemetryWorkerHandle>), sc_service::Error> {
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

    let telemetry_worker_handle = telemetry.as_ref().map(|(worker, _)| worker.handle());
    Ok((telemetry, telemetry_worker_handle))
}

pub fn new_partial(config: &Configuration) -> Result<Service, sc_service::Error> {
    let (telemetry_pair, telemetry_worker_handle) = telemetry(config)?;

    let heap_pages = config
        .executor
        .default_heap_pages
        .map_or(DEFAULT_HEAP_ALLOC_STRATEGY, |h| HeapAllocStrategy::Static {
            extra_pages: h as _,
        });

    let executor = WasmExecutor::<sp_io::SubstrateHostFunctions>::builder()
        .with_execution_method(config.executor.wasm_method)
        .with_onchain_heap_alloc_strategy(heap_pages)
        .with_offchain_heap_alloc_strategy(heap_pages)
        .with_max_runtime_instances(config.executor.max_runtime_instances)
        .with_runtime_cache_size(config.executor.runtime_cache_size)
        .build();

    let (client, backend, keystore_container, task_manager) =
        sc_service::new_full_parts::<Block, RuntimeApi, _>(
            config,
            telemetry_pair.as_ref().map(|(_, telemetry)| telemetry.handle()),
            executor,
        )?;
    let client = Arc::new(client);
    let select_chain = sc_consensus::LongestChain::new(backend.clone());

    let telemetry = telemetry_pair.map(|(worker, telemetry)| {
        task_manager.spawn_handle().spawn("telemetry", None, worker.run());
        telemetry
    });

    let transaction_pool = Arc::from(
        sc_transaction_pool::Builder::new(
            task_manager.spawn_essential_handle(),
            client.clone(),
            config.role.is_authority().into(),
        )
        .with_options(config.transaction_pool.clone())
        .with_prometheus(config.prometheus_registry())
        .build(),
    );

    let grandpa_authority_provider = client.clone();
    let (grandpa_block_import, grandpa_link) = sc_consensus_grandpa::block_import(
        client.clone(),
        512,
        &grandpa_authority_provider,
        select_chain.clone(),
        telemetry.as_ref().map(|telemetry| telemetry.handle()),
    )?;

    let import_queue = build_import_queue(
        client.clone(),
        grandpa_block_import.clone(),
        config,
        telemetry.as_ref().map(|telemetry| telemetry.handle()),
        &task_manager,
    )?;

    Ok(PartialComponents {
        backend,
        client,
        import_queue,
        keystore_container,
        task_manager,
        transaction_pool,
        select_chain,
        other: (grandpa_block_import, grandpa_link, telemetry, telemetry_worker_handle),
    })
}

fn build_import_queue(
    client: Arc<FullClient>,
    block_import: GrandpaBlockImport,
    config: &Configuration,
    telemetry: Option<TelemetryHandle>,
    task_manager: &TaskManager,
) -> Result<DefaultImportQueue<Block>, sp_consensus::Error> {
    sc_consensus_aura::import_queue::<AuraPair, _, _, _, _, _>(ImportQueueParams {
        block_import,
        justification_import: None,
        client,
        create_inherent_data_providers: move |_, ()| async move {
            let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
            let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                *timestamp,
                sp_consensus_aura::SlotDuration::from_millis(vibly_solo_runtime::SLOT_DURATION),
            );
            Ok((slot, timestamp))
        },
        spawner: &task_manager.spawn_essential_handle(),
        registry: config.prometheus_registry(),
        check_for_equivocation: Default::default(),
        telemetry,
        compatibility_mode: Default::default(),
    })
}

#[allow(clippy::too_many_arguments)]
fn start_aura(
    client: Arc<FullClient>,
    select_chain: FullSelectChain,
    block_import: GrandpaBlockImport,
    transaction_pool: Arc<sc_transaction_pool::TransactionPoolHandle<Block, FullClient>>,
    keystore: KeystorePtr,
    telemetry: Option<TelemetryHandle>,
    prometheus_registry: Option<&Registry>,
    task_manager: &TaskManager,
    sync_oracle: Arc<sc_network_sync::SyncingService<Block>>,
    force_authoring: bool,
) -> Result<(), sc_service::Error> {
    let proposer_factory = sc_basic_authorship::ProposerFactory::new(
        task_manager.spawn_handle(),
        client.clone(),
        transaction_pool,
        prometheus_registry,
        telemetry.clone(),
    );

    let aura = sc_consensus_aura::start_aura::<AuraPair, _, _, _, _, _, _, _, _, _, _>(
        StartAuraParams {
            slot_duration: sc_consensus_aura::slot_duration(&*client)?,
            client,
            select_chain,
            block_import,
            proposer_factory,
            sync_oracle,
            justification_sync_link: (),
            create_inherent_data_providers: move |_, ()| async move {
                let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
                let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                    *timestamp,
                    sp_consensus_aura::SlotDuration::from_millis(vibly_solo_runtime::SLOT_DURATION),
                );
                Ok((slot, timestamp))
            },
            force_authoring,
            backoff_authoring_blocks: Some(sc_consensus_slots::BackoffAuthoringOnFinalizedHeadLagging::default()),
            keystore,
            block_proposal_slot_portion: SlotProportion::new(2f32 / 3f32),
            max_block_proposal_slot_portion: None,
            telemetry,
            compatibility_mode: Default::default(),
        },
    )?;

    task_manager.spawn_essential_handle().spawn_blocking("aura", None, aura);
    Ok(())
}

pub fn start_node(config: Configuration) -> sc_service::error::Result<(TaskManager, Arc<FullClient>)> {
    let sc_service::PartialComponents {
        client,
        backend,
        mut task_manager,
        import_queue,
        keystore_container,
        transaction_pool,
        select_chain,
        other: (block_import, grandpa_link, mut telemetry, _telemetry_worker_handle),
    } = new_partial(&config)?;

    let prometheus_registry = config.prometheus_registry().cloned();
    let metrics = NetworkWorker::<Block, Hash>::register_notification_metrics(
        config.prometheus_config.as_ref().map(|cfg| &cfg.registry),
    );

    let mut net_config =
        sc_network::config::FullNetworkConfiguration::<Block, Hash, NetworkWorker<Block, Hash>>::new(
            &config.network,
            prometheus_registry.clone(),
        );
    let genesis_hash = client.chain_info().genesis_hash;
    let grandpa_protocol_name =
        sc_consensus_grandpa::protocol_standard_name(&genesis_hash, &config.chain_spec);
    let (grandpa_protocol_config, grandpa_notification_service) =
        sc_consensus_grandpa::grandpa_peers_set_config::<_, NetworkWorker<Block, Hash>>(
            grandpa_protocol_name.clone(),
            metrics.clone(),
            net_config.peer_store_handle(),
        );
    net_config.add_notification_protocol(grandpa_protocol_config);

    let (network, system_rpc_tx, tx_handler_controller, sync_service) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &config,
            net_config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            block_announce_validator_builder: None,
            warp_sync_config: None,
            block_relay: None,
            metrics,
        })?;

    if config.role.is_authority() {
        start_aura(
            client.clone(),
            select_chain,
            block_import,
            transaction_pool.clone(),
            keystore_container.keystore(),
            telemetry.as_ref().map(|telemetry| telemetry.handle()),
            prometheus_registry.as_ref(),
            &task_manager,
            sync_service.clone(),
            config.force_authoring,
        )?;
    }

    let local_role = config.role;
    let rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        network: network.clone(),
        client: client.clone(),
        keystore: keystore_container.keystore(),
        task_manager: &mut task_manager,
        transaction_pool: transaction_pool.clone(),
        rpc_builder: Box::new(|_| Ok(jsonrpsee::RpcModule::new(()))),
        backend: backend.clone(),
        system_rpc_tx,
        tx_handler_controller,
        sync_service: sync_service.clone(),
        config,
        telemetry: telemetry.as_mut(),
        tracing_execute_block: None,
    })?;
    drop(rpc_handlers);

    if let Some(grandpa_config) = Some(sc_consensus_grandpa::Config {
        gossip_duration: Duration::from_millis(1000),
        justification_generation_period: 512,
        name: Some(network.local_peer_id().to_string()),
        observer_enabled: false,
        keystore: Some(keystore_container.keystore()),
        local_role,
        telemetry: telemetry.as_ref().map(|x| x.handle()),
        protocol_name: grandpa_protocol_name,
    }) {
        task_manager.spawn_essential_handle().spawn_blocking(
            "grandpa-voter",
            None,
            sc_consensus_grandpa::run_grandpa_voter(sc_consensus_grandpa::GrandpaParams {
                config: grandpa_config,
                link: grandpa_link,
                network,
                sync: sync_service,
                voting_rule: sc_consensus_grandpa::VotingRulesBuilder::default().build(),
                prometheus_registry,
                shared_voter_state: sc_consensus_grandpa::SharedVoterState::empty(),
                telemetry: telemetry.as_ref().map(|x| x.handle()),
                notification_service: grandpa_notification_service,
                offchain_tx_pool_factory: OffchainTransactionPoolFactory::new(transaction_pool),
            })?,
        );
    }

    Ok((task_manager, client))
}

#![no_std]

pub mod monitoring;

pub use monitoring::{
    ChainFinalityConfig, ChainId, MonitoredTransaction, MonitoringStatus, VerificationMethod,
    BridgeTransfer, TransferStatus,
    monitor_source_transaction, get_monitored_tx, check_for_reorg, handle_reorg,
    update_transaction_confirmation_count, mark_transaction_failed, create_bridge_transfer,
    add_validator_signature, approve_transfer_for_minting, complete_transfer,
    get_chain_finality_config, set_chain_finality_config,
};

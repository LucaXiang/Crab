//! Order Event Sourcing Module
//!
//! This module provides types for the order event sourcing system:
//! - Commands: Requests from clients to modify orders
//! - Events: Immutable facts recorded after command processing
//! - Snapshots: Computed order state from event stream

pub mod applied_mg_rule;
pub mod applied_rule;
pub mod canonical;
pub mod command;
pub mod event;
pub mod snapshot;
pub mod types;
pub mod verifactu;

// Re-exports
pub use applied_mg_rule::AppliedMgRule;
pub use applied_rule::AppliedRule;
pub use canonical::{
    AnulacionChainData, CanonicalHash, ChainHashable, CreditNoteChainData, OrderChainData,
    UpgradeChainData, compute_anulacion_chain_hash, compute_chain_hash,
    compute_credit_note_chain_hash, compute_event_chain_hash, compute_order_chain_hash,
    compute_upgrade_chain_hash,
};
pub use command::{OrderCommand, OrderCommandPayload};
pub use event::{EventPayload, MgItemDiscount, OrderEvent, OrderEventType};
pub use snapshot::{OrderSnapshot, OrderStatus};
pub use types::*;
pub use verifactu::{
    HuellaAltaInput, HuellaBajaInput, HuellaError, compute_verifactu_huella_alta,
    compute_verifactu_huella_baja,
};

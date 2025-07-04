// OSPF Protocol Implementation - Modular Structure
//
// This module provides a clean, modular implementation of OSPF protocol components.
// The refactored structure separates concerns and improves maintainability.

use serde::{Serialize, Deserialize};

// pub mod converters; // TODO: Implement converters module
pub mod events;
pub mod packets;
pub mod state;
pub mod packet_processor;
pub mod error_handling;

// Re-export commonly used types for convenience
pub use events::{OSPFEvent, EventBus, EventHandler};
pub use packets::{PacketType, PacketHandler, OSPFPacket, HelloPacket, 
    DatabaseDescriptionPacket, LinkStateRequestPacket, LinkStateUpdatePacket, LinkStateAckPacket,
    DDPacket, LSRequestPacket, LSUpdatePacket, LSAckPacket};
pub use state::{NeighborState, InterfaceState};

// Re-export LSA-related types from packet modules
pub use packets::lsu::{LSA, LSAHeader as LSUHeader};
pub use packets::dd::{LsaHeader as DDLsaHeader};
pub use packets::lsr::{LSRequest};
pub use packets::lsack::{LSAHeader};

// Type alias for backward compatibility
pub type LinkStateAcknowledgmentPacket = LSAckPacket;
pub type LSARequest = LSRequest;

// Core OSPF packet types and enums
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OspfPacketType {
    Hello = 1,
    DatabaseDescription = 2,
    LinkStateRequest = 3,
    LinkStateUpdate = 4,
    LinkStateAcknowledgment = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LsaType {
    Router = 1,
    Network = 2,
    Summary = 3,
    ASBRSummary = 4,
    ASExternal = 5,
}

// Legacy type aliases for backward compatibility
// TODO: Gradually migrate code to use the new packet structures
pub type OSPFPacketType = OspfPacketType;
pub type OSPFPacketData = OSPFPacket;
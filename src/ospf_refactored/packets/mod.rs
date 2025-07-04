// OSPF Packet Definitions and Handlers
//
// This module contains all OSPF packet type definitions, separated from
// processing logic for better maintainability.

pub mod hello;
pub mod dd;
pub mod lsr;
pub mod lsu;
pub mod lsack;

use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

// Re-export packet types for convenience
pub use hello::{HelloPacket};
pub use dd::{DDPacket};
pub use lsr::{LSRequestPacket};
pub use lsu::{LSUpdatePacket};
pub use lsack::{LSAckPacket};

// Type aliases for backward compatibility
pub type DatabaseDescriptionPacket = DDPacket;
pub type LinkStateRequestPacket = LSRequestPacket;
pub type LinkStateUpdatePacket = LSUpdatePacket;
pub type LinkStateAckPacket = LSAckPacket;

/// Common OSPF packet header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OSPFHeader {
    pub version: u8,
    pub packet_type: PacketType,
    pub packet_length: u16,
    pub router_id: Ipv4Addr,
    pub area_id: Ipv4Addr,
    pub checksum: u16,
    pub auth_type: u16,
    pub authentication: [u8; 8],
}

/// OSPF packet types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum PacketType {
    Hello = 1,
    DatabaseDescription = 2,
    LinkStateRequest = 3,
    LinkStateUpdate = 4,
    LinkStateAck = 5,
}

/// Trait for packet handlers
pub trait PacketHandler: Send + Sync {
    /// Process a packet and return any response packets
    fn handle_packet(&mut self, packet: OSPFPacket, from_router: u32) -> Result<Vec<OSPFPacket>, PacketError>;
    
    /// Get the packet types this handler processes
    fn handled_types(&self) -> Vec<PacketType>;
}

/// Wrapper for all OSPF packet types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OSPFPacket {
    Hello(HelloPacket),
    DatabaseDescription(DDPacket),
    LinkStateRequest(LSRequestPacket),
    LinkStateUpdate(LSUpdatePacket),
    LinkStateAck(LSAckPacket),
}

impl OSPFPacket {
    /// Get the packet type
    pub fn packet_type(&self) -> PacketType {
        match self {
            OSPFPacket::Hello(_) => PacketType::Hello,
            OSPFPacket::DatabaseDescription(_) => PacketType::DatabaseDescription,
            OSPFPacket::LinkStateRequest(_) => PacketType::LinkStateRequest,
            OSPFPacket::LinkStateUpdate(_) => PacketType::LinkStateUpdate,
            OSPFPacket::LinkStateAck(_) => PacketType::LinkStateAck,
        }
    }
    
    /// Get the common header
    pub fn header(&self) -> &OSPFHeader {
        match self {
            OSPFPacket::Hello(p) => &p.header,
            OSPFPacket::DatabaseDescription(p) => &p.header,
            OSPFPacket::LinkStateRequest(p) => &p.header,
            OSPFPacket::LinkStateUpdate(p) => &p.header,
            OSPFPacket::LinkStateAck(p) => &p.header,
        }
    }
}

/// Errors that can occur during packet processing
#[derive(Debug, Clone)]
pub enum PacketError {
    InvalidFormat(String),
    AuthenticationFailed,
    ChecksumMismatch,
    InvalidPacketType(u8),
    ProcessingError(String),
}

impl std::fmt::Display for PacketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PacketError::InvalidFormat(msg) => write!(f, "Invalid packet format: {}", msg),
            PacketError::AuthenticationFailed => write!(f, "Authentication failed"),
            PacketError::ChecksumMismatch => write!(f, "Checksum mismatch"),
            PacketError::InvalidPacketType(t) => write!(f, "Invalid packet type: {}", t),
            PacketError::ProcessingError(msg) => write!(f, "Processing error: {}", msg),
        }
    }
}

impl std::error::Error for PacketError {}

/// Utility functions for packet handling
pub mod utils {
    
    
    /// Calculate OSPF packet checksum
    pub fn calculate_checksum(data: &[u8]) -> u16 {
        let mut sum: u32 = 0;
        let mut i = 0;
        
        // Skip the checksum field itself (bytes 12-13)
        while i < data.len() {
            if i == 12 {
                i += 2;
                continue;
            }
            
            if i + 1 < data.len() {
                sum += u16::from_be_bytes([data[i], data[i + 1]]) as u32;
            } else {
                sum += (data[i] as u32) << 8;
            }
            
            i += 2;
        }
        
        // Add carry bits
        while (sum >> 16) != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }
        
        !sum as u16
    }
    
    /// Verify packet checksum
    pub fn verify_checksum(data: &[u8]) -> bool {
        calculate_checksum(data) == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_packet_type_values() {
        assert_eq!(PacketType::Hello as u8, 1);
        assert_eq!(PacketType::DatabaseDescription as u8, 2);
        assert_eq!(PacketType::LinkStateRequest as u8, 3);
        assert_eq!(PacketType::LinkStateUpdate as u8, 4);
        assert_eq!(PacketType::LinkStateAck as u8, 5);
    }
    
    #[test]
    fn test_checksum_calculation() {
        let data = vec![0x01, 0x02, 0x03, 0x04];
        let checksum = utils::calculate_checksum(&data);
        assert_ne!(checksum, 0);
    }
}
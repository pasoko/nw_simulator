// use crate::ospf::OspfPacketType; // Not needed, using local definitions
use super::{OSPFHeader, PacketError};
use crate::ospf_refactored::events::OSPFEvent;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::net::Ipv4Addr;

/// Database Description (DD) packet structure (RFC 2328 Section A.3.3)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DDPacket {
    pub header: OSPFHeader,
    pub interface_mtu: u16,
    pub options: u8,
    pub flags: u8,
    pub dd_sequence_number: u32,
    pub lsa_headers: Vec<LsaHeader>,
}

/// LSA Header structure (simplified for now)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LsaHeader {
    pub ls_age: u16,
    pub options: u8,
    pub ls_type: u8,
    pub link_state_id: u32,
    pub advertising_router: u32,
    pub ls_sequence_number: u32,
    pub ls_checksum: u16,
    pub length: u16,
}

impl DDPacket {
    /// Create a new DD packet
    pub fn new(interface_mtu: u16, dd_sequence_number: u32) -> Self {
        Self {
            header: OSPFHeader {
                version: 2,
                packet_type: super::PacketType::DatabaseDescription,
                packet_length: 0, // To be calculated
                router_id: std::net::Ipv4Addr::new(0, 0, 0, 0),
                area_id: std::net::Ipv4Addr::new(0, 0, 0, 0),
                checksum: 0,
                auth_type: 0,
                authentication: [0; 8],
            },
            interface_mtu,
            options: 0x02, // E-bit set by default
            flags: 0x07,   // I-bit, M-bit, MS-bit set initially
            dd_sequence_number,
            lsa_headers: Vec::new(),
        }
    }

    /// Check if this is an initial DD packet
    pub fn is_initial(&self) -> bool {
        (self.flags & 0x04) != 0  // I-bit
    }

    /// Check if more DD packets follow
    pub fn has_more(&self) -> bool {
        (self.flags & 0x02) != 0  // M-bit
    }

    /// Check if sender is master
    pub fn is_master(&self) -> bool {
        (self.flags & 0x01) != 0  // MS-bit
    }

    // TODO: Implement DD packet serialization
    // TODO: Implement DD packet deserialization
    // TODO: Implement LSA header extraction
}

/// Handle incoming DD packet
pub fn handle_dd_packet(
    _packet_data: &[u8],
    _source_router_id: u32,
    _interface_id: u32,
) -> Result<(), String> {
    // TODO: Implement DD packet processing logic
    // - Parse packet structure
    // - Update neighbor state machine
    // - Handle master/slave negotiation
    // - Process LSA headers
    // - Generate response if needed
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dd_packet_creation() {
        let dd = DDPacket::new(1500, 12345);
        assert_eq!(dd.interface_mtu, 1500);
        assert_eq!(dd.dd_sequence_number, 12345);
        assert!(dd.is_initial());
        assert!(dd.has_more());
        assert!(dd.is_master());
    }

    // TODO: Add more tests for DD packet handling
}

/// Database Description Packet Handler
#[derive(Debug)]
pub struct DDPacketHandler {
    router_id: Ipv4Addr,
    /// Track DD exchange state per neighbor
    neighbor_dd_state: HashMap<u32, DDExchangeState>,
}

#[derive(Debug, Clone)]
struct DDExchangeState {
    /// Are we the master?
    is_master: bool,
    /// Current DD sequence number
    dd_sequence: u32,
    /// Last received DD sequence
    last_received_sequence: u32,
    /// Is negotiation complete?
    negotiation_complete: bool,
    /// Is exchange complete?
    exchange_complete: bool,
    /// LSAs we need to request
    lsas_to_request: Vec<LsaHeader>,
}

impl DDPacketHandler {
    pub fn new(router_id: Ipv4Addr) -> Self {
        Self {
            router_id,
            neighbor_dd_state: HashMap::new(),
        }
    }
    
    /// Handle incoming DD packet
    pub fn handle_dd_packet(
        &mut self,
        packet: &DatabaseDescriptionPacket,
        from_neighbor: u32,
    ) -> Result<Vec<OSPFEvent>, PacketError> {
        let mut events = Vec::new();
        
        // Get or create neighbor DD state
        self.neighbor_dd_state.entry(from_neighbor)
            .or_insert_with(|| DDExchangeState {
                is_master: u32::from_be_bytes(self.router_id.octets()) > from_neighbor,
                dd_sequence: 0,
                last_received_sequence: 0,
                negotiation_complete: false,
                exchange_complete: false,
                lsas_to_request: Vec::new(),
            });
        
        if packet.is_initial() {
            // Handle master/slave negotiation
            self.handle_initial_dd(packet, from_neighbor, &mut events)?;
        } else {
            // Handle DD exchange
            self.handle_dd_exchange(packet, from_neighbor, &mut events)?;
        }
        
        Ok(events)
    }
    
    /// Handle initial DD packet (master/slave negotiation)
    fn handle_initial_dd(
        &mut self,
        packet: &DatabaseDescriptionPacket,
        from_neighbor: u32,
        events: &mut Vec<OSPFEvent>,
    ) -> Result<(), PacketError> {
        let state = self.neighbor_dd_state.get_mut(&from_neighbor).unwrap();
        let our_router_id = u32::from_be_bytes(self.router_id.octets());
        
        if packet.is_master() {
            // Neighbor thinks it's master
            if our_router_id > from_neighbor {
                // We should be master, send DD with MS=1
                state.is_master = true;
                state.dd_sequence = packet.dd_sequence_number + 1;
                
                events.push(OSPFEvent::PacketSendRequired {
                    packet_type: crate::ospf_refactored::events::PacketType::DatabaseDescription,
                    destination: from_neighbor,
                    interface_id: 0, // Should be provided
                    additional_data: Some(format!("ms=1,seq={}", state.dd_sequence)),
                });
            } else {
                // Accept neighbor as master
                state.is_master = false;
                state.dd_sequence = packet.dd_sequence_number;
                state.negotiation_complete = true;
                
                events.push(OSPFEvent::PacketSendRequired {
                    packet_type: crate::ospf_refactored::events::PacketType::DatabaseDescription,
                    destination: from_neighbor,
                    interface_id: 0,
                    additional_data: Some(format!("ms=0,seq={}", state.dd_sequence)),
                });
            }
        } else {
            // Neighbor accepts us as master
            if state.is_master {
                state.negotiation_complete = true;
            }
        }
        
        state.last_received_sequence = packet.dd_sequence_number;
        Ok(())
    }
    
    /// Handle DD exchange
    fn handle_dd_exchange(
        &mut self,
        packet: &DatabaseDescriptionPacket,
        from_neighbor: u32,
        events: &mut Vec<OSPFEvent>,
    ) -> Result<(), PacketError> {
        // Process LSA headers first (before mutable borrow)
        let lsas_to_request: Vec<LsaHeader> = packet.lsa_headers.iter()
            .filter(|lsa_header| self.need_lsa(lsa_header))
            .cloned()
            .collect();
        
        let state = self.neighbor_dd_state.get_mut(&from_neighbor).unwrap();
        // Verify sequence number
        if state.is_master {
            if packet.dd_sequence_number != state.dd_sequence {
                return Err(PacketError::ProcessingError(
                    format!("DD sequence mismatch: expected {}, got {}", 
                            state.dd_sequence, packet.dd_sequence_number)
                ));
            }
        } else {
            state.dd_sequence = packet.dd_sequence_number;
        }
        
        state.lsas_to_request.extend(lsas_to_request);
        
        // Check if exchange is complete
        if !packet.has_more() {
            state.exchange_complete = true;
            
            // Generate LSR if we have LSAs to request
            if !state.lsas_to_request.is_empty() {
                events.push(OSPFEvent::PacketSendRequired {
                    packet_type: crate::ospf_refactored::events::PacketType::LinkStateRequest,
                    destination: from_neighbor,
                    interface_id: 0,
                    additional_data: Some(format!("{} LSAs", state.lsas_to_request.len())),
                });
            }
        } else {
            // Send next DD packet
            if state.is_master {
                state.dd_sequence += 1;
            }
            
            events.push(OSPFEvent::PacketSendRequired {
                packet_type: crate::ospf_refactored::events::PacketType::DatabaseDescription,
                destination: from_neighbor,
                interface_id: 0,
                additional_data: Some(format!("seq={}", state.dd_sequence)),
            });
        }
        
        state.last_received_sequence = packet.dd_sequence_number;
        Ok(())
    }
    
    /// Check if we need this LSA
    fn need_lsa(&self, lsa_header: &LsaHeader) -> bool {
        // Simplified: always request LSAs for now
        // In real implementation, would check against local LSDB
        true
    }
    
    /// Check if negotiation is complete for a neighbor
    pub fn is_negotiation_complete(&self, neighbor_id: u32) -> bool {
        self.neighbor_dd_state.get(&neighbor_id)
            .map(|state| state.negotiation_complete)
            .unwrap_or(false)
    }
    
    /// Check if exchange is complete for a neighbor
    pub fn is_exchange_complete(&self, neighbor_id: u32) -> bool {
        self.neighbor_dd_state.get(&neighbor_id)
            .map(|state| state.exchange_complete)
            .unwrap_or(false)
    }
    
    /// Check if we have LSAs to request from a neighbor
    pub fn has_lsas_to_request(&self, neighbor_id: u32) -> bool {
        self.neighbor_dd_state.get(&neighbor_id)
            .map(|state| !state.lsas_to_request.is_empty())
            .unwrap_or(false)
    }
}

// Type alias for compatibility
pub type DatabaseDescriptionPacket = DDPacket;
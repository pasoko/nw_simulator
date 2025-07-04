// OSPF Hello Packet Definition and Handler
//
// Hello packets are used for neighbor discovery and maintenance.
// This module separates the packet definition from processing logic.

use super::{OSPFHeader, PacketType, PacketError, OSPFPacket};
use crate::ospf_refactored::events::{OSPFEvent, EventResult};
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;
use std::collections::HashSet;

/// OSPF Hello packet structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloPacket {
    pub header: OSPFHeader,
    pub network_mask: Ipv4Addr,
    pub hello_interval: u16,
    pub options: u8,
    pub router_priority: u8,
    pub router_dead_interval: u32,
    pub designated_router: Ipv4Addr,
    pub backup_designated_router: Ipv4Addr,
    pub neighbors: Vec<Ipv4Addr>,
}

impl HelloPacket {
    /// Create a new Hello packet
    pub fn new(
        router_id: Ipv4Addr,
        area_id: Ipv4Addr,
        network_mask: Ipv4Addr,
        hello_interval: u16,
        router_priority: u8,
        router_dead_interval: u32,
    ) -> Self {
        Self {
            header: OSPFHeader {
                version: 2,
                packet_type: PacketType::Hello,
                packet_length: 0, // Will be calculated when serializing
                router_id,
                area_id,
                checksum: 0, // Will be calculated when serializing
                auth_type: 0,
                authentication: [0; 8],
            },
            network_mask,
            hello_interval,
            options: 0x02, // E-bit set
            router_priority,
            router_dead_interval,
            designated_router: Ipv4Addr::new(0, 0, 0, 0),
            backup_designated_router: Ipv4Addr::new(0, 0, 0, 0),
            neighbors: Vec::new(),
        }
    }
    
    /// Add a neighbor to the hello packet
    pub fn add_neighbor(&mut self, neighbor_id: Ipv4Addr) {
        if !self.neighbors.contains(&neighbor_id) {
            self.neighbors.push(neighbor_id);
        }
    }
    
    /// Check if this hello indicates bidirectional communication
    pub fn is_bidirectional(&self, my_router_id: Ipv4Addr) -> bool {
        self.neighbors.contains(&my_router_id)
    }
    
    /// Validate hello packet parameters
    pub fn validate(&self) -> Result<(), PacketError> {
        if self.hello_interval == 0 {
            return Err(PacketError::InvalidFormat("Hello interval cannot be 0".into()));
        }
        
        if self.router_dead_interval < self.hello_interval as u32 {
            return Err(PacketError::InvalidFormat(
                "Dead interval must be >= hello interval".into()
            ));
        }
        
        if self.header.version != 2 {
            return Err(PacketError::InvalidFormat(
                format!("Unsupported OSPF version: {}", self.header.version)
            ));
        }
        
        Ok(())
    }
}

/// Handler for Hello packets
pub struct HelloPacketHandler {
    router_id: Ipv4Addr,
    area_id: Ipv4Addr,
    hello_interval: u16,
    dead_interval: u32,
    active_neighbors: HashSet<Ipv4Addr>,
}

impl HelloPacketHandler {
    pub fn new(
        router_id: Ipv4Addr,
        area_id: Ipv4Addr,
        hello_interval: u16,
        dead_interval: u32,
    ) -> Self {
        Self {
            router_id,
            area_id,
            hello_interval,
            dead_interval,
            active_neighbors: HashSet::new(),
        }
    }
    
    /// Process a received hello packet
    pub fn process_hello(&mut self, packet: &HelloPacket, from_router: u32) -> EventResult {
        let mut events = Vec::new();
        
        // Validate packet
        packet.validate().map_err(|e| crate::ospf_refactored::events::EventError::InvalidEventData(e.to_string()))?;
        
        // Check area ID
        if packet.header.area_id != self.area_id {
            // TODO: Add debug logging
            // console_log!("Hello from different area: {} vs {}", packet.header.area_id, self.area_id);
            return Ok(events);
        }
        
        // Check hello/dead intervals
        if packet.hello_interval != self.hello_interval || 
           packet.router_dead_interval != self.dead_interval {
            // TODO: Add debug logging
            // console_log!("Hello interval mismatch from router {}", from_router);
            return Ok(events);
        }
        
        let neighbor_id = packet.header.router_id;
        let was_known = self.active_neighbors.contains(&neighbor_id);
        let is_bidirectional = packet.is_bidirectional(self.router_id);
        
        // Update neighbor list
        if !was_known {
            self.active_neighbors.insert(neighbor_id);
            
            // Generate neighbor discovered event
            events.push(OSPFEvent::NeighborStateChanged {
                router_id: u32::from_be_bytes(self.router_id.octets()),
                neighbor_id: from_router,
                from_state: crate::ospf_refactored::state::NeighborState::Down,
                to_state: if is_bidirectional {
                    crate::ospf_refactored::state::NeighborState::TwoWay
                } else {
                    crate::ospf_refactored::state::NeighborState::Init
                },
                interface_id: 0, // Would be provided by actual implementation
            });
        }
        
        // Check for DR/BDR changes
        if packet.designated_router != Ipv4Addr::new(0, 0, 0, 0) ||
           packet.backup_designated_router != Ipv4Addr::new(0, 0, 0, 0) {
            events.push(OSPFEvent::DRElectionRequired {
                interface_id: 0, // Would be provided by actual implementation
                priority_changed: false,
            });
        }
        
        Ok(events)
    }
    
    /// Generate a hello packet
    pub fn generate_hello(&self, interface_id: u32) -> HelloPacket {
        let mut packet = HelloPacket::new(
            self.router_id,
            self.area_id,
            Ipv4Addr::new(255, 255, 255, 0), // Would come from interface
            self.hello_interval,
            1, // Default priority
            self.dead_interval,
        );
        
        // Add known neighbors
        for neighbor in &self.active_neighbors {
            packet.add_neighbor(*neighbor);
        }
        
        packet
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hello_packet_creation() {
        let packet = HelloPacket::new(
            Ipv4Addr::new(1, 1, 1, 1),
            Ipv4Addr::new(0, 0, 0, 0),
            Ipv4Addr::new(255, 255, 255, 0),
            10,
            1,
            40,
        );
        
        assert_eq!(packet.hello_interval, 10);
        assert_eq!(packet.router_dead_interval, 40);
        assert_eq!(packet.neighbors.len(), 0);
    }
    
    #[test]
    fn test_hello_validation() {
        let mut packet = HelloPacket::new(
            Ipv4Addr::new(1, 1, 1, 1),
            Ipv4Addr::new(0, 0, 0, 0),
            Ipv4Addr::new(255, 255, 255, 0),
            10,
            1,
            40,
        );
        
        assert!(packet.validate().is_ok());
        
        // Invalid: hello interval = 0
        packet.hello_interval = 0;
        assert!(packet.validate().is_err());
        
        // Invalid: dead interval < hello interval
        packet.hello_interval = 10;
        packet.router_dead_interval = 5;
        assert!(packet.validate().is_err());
    }
    
    #[test]
    fn test_bidirectional_check() {
        let mut packet = HelloPacket::new(
            Ipv4Addr::new(1, 1, 1, 1),
            Ipv4Addr::new(0, 0, 0, 0),
            Ipv4Addr::new(255, 255, 255, 0),
            10,
            1,
            40,
        );
        
        let my_id = Ipv4Addr::new(2, 2, 2, 2);
        assert!(!packet.is_bidirectional(my_id));
        
        packet.add_neighbor(my_id);
        assert!(packet.is_bidirectional(my_id));
    }
}
use std::collections::HashMap;
use crate::ospf::{OSPFPacket, OSPFPacketType, OSPFPacketData, HelloPacket, DatabaseDescriptionPacket};
use crate::router::{OSPFNeighbor, OSPFNeighborState};
use crate::protocol::{ProtocolPacket, PacketEvent};

pub struct OSPFEngine {
    router_id: String,
    area_id: String,
    hello_interval: u16,
    dead_interval: u32,
    neighbors: HashMap<u32, OSPFNeighbor>,
}

impl OSPFEngine {
    pub fn new(router_id: String, area_id: String) -> Self {
        OSPFEngine {
            router_id,
            area_id,
            hello_interval: 10,
            dead_interval: 40,
            neighbors: HashMap::new(),
        }
    }

    pub fn process_hello_packet(&mut self, packet: &HelloPacket, from_router_id: u32, interface_id: u32) -> Vec<PacketEvent> {
        let events = Vec::new();
        
        // Check if we already know this neighbor
        let (current_state, _is_new) = if let Some(neighbor) = self.neighbors.get_mut(&from_router_id) {
            // Update last seen time
            (neighbor.state.clone(), false)
        } else {
            // New neighbor discovered
            let new_neighbor = OSPFNeighbor {
                router_id: format!("{}.{}.{}.{}", 1, 1, 1, from_router_id),
                state: OSPFNeighborState::Down,
                interface_id,
                priority: packet.router_priority,
            };
            self.neighbors.insert(from_router_id, new_neighbor);
            (OSPFNeighborState::Down, true)
        };

        // State machine progression
        match current_state {
            OSPFNeighborState::Down => {
                // Move to Init state - we heard from them
                if let Some(neighbor) = self.neighbors.get_mut(&from_router_id) {
                    neighbor.state = OSPFNeighborState::Init;
                }
            }
            OSPFNeighborState::Init => {
                // Check if we are in neighbor's hello packet
                let our_router_id = format!("{}.{}.{}.{}", 1, 1, 1, self.router_id.parse::<u32>().unwrap_or(0));
                if packet.neighbors.contains(&our_router_id) || packet.neighbors.contains(&self.router_id) {
                    if let Some(neighbor) = self.neighbors.get_mut(&from_router_id) {
                        neighbor.state = OSPFNeighborState::TwoWay;
                        
                        // For simplicity, immediately move to Full state
                        // In real OSPF, there would be DD, LS Request/Update exchange
                        neighbor.state = OSPFNeighborState::Full;
                    }
                }
            }
            OSPFNeighborState::TwoWay => {
                // For simplicity, move directly to Full
                if let Some(neighbor) = self.neighbors.get_mut(&from_router_id) {
                    neighbor.state = OSPFNeighborState::Full;
                }
            }
            _ => {
                // Maintain current state
            }
        }

        events
    }

    pub fn process_dd_packet(&mut self, packet: &DatabaseDescriptionPacket, from_router_id: u32) -> Vec<PacketEvent> {
        let events = Vec::new();
        
        if let Some(neighbor) = self.neighbors.get_mut(&from_router_id) {
            match neighbor.state {
                OSPFNeighborState::ExStart => {
                    // Master/Slave negotiation
                    neighbor.state = OSPFNeighborState::Exchange;
                    // TODO: Exchange LSA headers
                }
                OSPFNeighborState::Exchange => {
                    // Exchange LSA headers
                    if packet.lsa_headers.is_empty() {
                        neighbor.state = OSPFNeighborState::Loading;
                    }
                }
                OSPFNeighborState::Loading => {
                    // All LSAs loaded
                    neighbor.state = OSPFNeighborState::Full;
                }
                _ => {}
            }
        }
        
        events
    }

    pub fn generate_hello_packet(&self) -> HelloPacket {
        HelloPacket {
            network_mask: "255.255.255.252".to_string(),
            hello_interval: self.hello_interval,
            options: 0x02, // E-bit set
            router_priority: 1,
            router_dead_interval: self.dead_interval,
            designated_router: "0.0.0.0".to_string(),
            backup_designated_router: "0.0.0.0".to_string(),
            neighbors: self.neighbors.iter()
                .filter(|(_, n)| matches!(n.state, OSPFNeighborState::Init | OSPFNeighborState::TwoWay | OSPFNeighborState::Full))
                .map(|(id, _)| format!("{}.{}.{}.{}", 1, 1, 1, id))
                .collect(),
        }
    }

    fn create_dd_packet_event(&self, to_router_id: u32) -> PacketEvent {
        let dd_packet = DatabaseDescriptionPacket {
            interface_mtu: 1500,
            options: 0x02,
            flags: 0x07, // I, M, MS bits set
            dd_sequence_number: 1,
            lsa_headers: Vec::new(), // Start with empty
        };
        
        let ospf_packet = OSPFPacket {
            version: 2,
            packet_type: OSPFPacketType::DatabaseDescription,
            router_id: self.router_id.clone(),
            area_id: self.area_id.clone(),
            checksum: 0,
            auth_type: 0,
            authentication: 0,
            data: OSPFPacketData::DatabaseDescription(dd_packet),
        };
        
        PacketEvent {
            timestamp: 0.0, // Will be set by scheduler
            from_router_id: self.router_id.parse().unwrap_or(0),
            to_router_id,
            packet: ProtocolPacket::OSPF(ospf_packet),
        }
    }

    pub fn get_neighbor_states(&self) -> HashMap<u32, OSPFNeighborState> {
        self.neighbors.iter()
            .map(|(id, neighbor)| (*id, neighbor.state.clone()))
            .collect()
    }
}
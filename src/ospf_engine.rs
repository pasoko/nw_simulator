use std::collections::HashMap;
use crate::ospf::{OSPFPacket, OSPFPacketType, OSPFPacketData, HelloPacket, DatabaseDescriptionPacket, LSA, LSAHeader, LinkStateUpdatePacket};
use crate::router::{OSPFNeighbor, OSPFNeighborState, RouterLSA, RouterLink, LinkType, LSAType, LSAData, LSAHeader as RouterLSAHeader};
use crate::protocol::{ProtocolPacket, PacketEvent};

pub struct OSPFEngine {
    router_id: String,
    area_id: String,
    hello_interval: u16,
    dead_interval: u32,
    neighbors: HashMap<u32, OSPFNeighbor>,
    neighbor_last_hello: HashMap<u32, f64>,  // Track last hello time
    current_time: f64,
    lsa_database: HashMap<String, crate::router::LSA>,  // Key: LSA identifier (type:ls_id:adv_router)
    lsa_sequence_number: u32,  // Current sequence number for self-originated LSAs
    router_links: Vec<(u32, u32, u32)>,  // (neighbor_id, interface_id, cost) for Router LSA generation
}

impl OSPFEngine {
    pub fn new(router_id: String, area_id: String) -> Self {
        OSPFEngine {
            router_id,
            area_id,
            hello_interval: 10,
            dead_interval: 40,
            neighbors: HashMap::new(),
            neighbor_last_hello: HashMap::new(),
            current_time: 0.0,
            lsa_database: HashMap::new(),
            lsa_sequence_number: 0x80000001,  // Start with initial sequence number
            router_links: Vec::new(),
        }
    }
    
    pub fn update_time(&mut self, time: f64) {
        let time_delta = time - self.current_time;
        self.current_time = time;
        self.check_dead_neighbors();
        self.age_lsas(time_delta);
    }
    
    fn age_lsas(&mut self, time_delta: f64) {
        const MAX_AGE: u16 = 3600;  // 1 hour in seconds
        let mut expired_lsas = Vec::new();
        
        for (key, lsa) in self.lsa_database.iter_mut() {
            // Age the LSA
            let new_age = lsa.header.ls_age as f64 + time_delta;
            if new_age >= MAX_AGE as f64 {
                expired_lsas.push(key.clone());
            } else {
                lsa.header.ls_age = new_age as u16;
            }
        }
        
        // Remove expired LSAs
        for key in expired_lsas {
            self.lsa_database.remove(&key);
        }
    }
    
    fn check_dead_neighbors(&mut self) {
        let mut dead_neighbors = Vec::new();
        
        for (id, last_hello) in &self.neighbor_last_hello {
            if self.current_time - last_hello > self.dead_interval as f64 {
                dead_neighbors.push(*id);
            }
        }
        
        for id in dead_neighbors {
            if let Some(neighbor) = self.neighbors.get_mut(&id) {
                neighbor.state = OSPFNeighborState::Down;
            }
            self.neighbor_last_hello.remove(&id);
        }
    }

    pub fn process_hello_packet(&mut self, packet: &HelloPacket, from_router_id: u32, interface_id: u32) -> Vec<PacketEvent> {
        let mut events = Vec::new();
        
        // Update last hello time
        self.neighbor_last_hello.insert(from_router_id, self.current_time);
        
        // Check if we already know this neighbor
        let (current_state, _is_new) = if let Some(neighbor) = self.neighbors.get_mut(&from_router_id) {
            // Update neighbor info
            neighbor.priority = packet.router_priority;
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
                let our_router_id = self.router_id.clone();
                if packet.neighbors.contains(&our_router_id) {
                    if let Some(neighbor) = self.neighbors.get_mut(&from_router_id) {
                        neighbor.state = OSPFNeighborState::TwoWay;
                        
                        // Start forming adjacency - move to ExStart
                        neighbor.state = OSPFNeighborState::ExStart;
                        
                        // Send Database Description packet
                        events.push(self.create_dd_packet_event(from_router_id));
                    }
                }
            }
            OSPFNeighborState::TwoWay => {
                // If not already forming adjacency, start now
                if let Some(neighbor) = self.neighbors.get_mut(&from_router_id) {
                    neighbor.state = OSPFNeighborState::ExStart;
                    events.push(self.create_dd_packet_event(from_router_id));
                }
            }
            _ => {
                // Maintain current state
            }
        }

        events
    }

    pub fn process_dd_packet(&mut self, _packet: &DatabaseDescriptionPacket, from_router_id: u32) -> Vec<PacketEvent> {
        let mut events = Vec::new();
        
        if let Some(neighbor) = self.neighbors.get_mut(&from_router_id) {
            match neighbor.state {
                OSPFNeighborState::ExStart => {
                    // Master/Slave negotiation - simplified
                    neighbor.state = OSPFNeighborState::Exchange;
                    
                    // Send our database summary (empty for now)
                    events.push(self.create_dd_packet_event(from_router_id));
                }
                OSPFNeighborState::Exchange => {
                    // For simplicity, immediately move to Full
                    // In real OSPF, would exchange LSA headers
                    neighbor.state = OSPFNeighborState::Full;
                    
                    // Generate Router LSA when adjacency forms
                    let router_lsa = self.generate_router_lsa();
                    let lsa_clone = router_lsa.clone();
                    self.update_lsa_database(router_lsa);
                    
                    // Flood the new LSA to neighbors
                    let flood_events = self.flood_lsa(&lsa_clone);
                    events.extend(flood_events);
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
        // Convert our LSA headers to OSPF packet format
        let lsa_headers: Vec<LSAHeader> = self.lsa_database.values().map(|lsa| {
            LSAHeader {
                age: lsa.header.ls_age,
                options: 0x02,
                lsa_type: lsa.header.ls_type.clone() as u8,
                link_state_id: lsa.header.link_state_id.clone(),
                advertising_router: lsa.header.advertising_router.clone(),
                sequence_number: lsa.header.ls_sequence_number,
                checksum: lsa.header.ls_checksum,
                length: lsa.header.length,
            }
        }).collect();
        
        let dd_packet = DatabaseDescriptionPacket {
            interface_mtu: 1500,
            options: 0x02,
            flags: 0x07, // I, M, MS bits set
            dd_sequence_number: 1,
            lsa_headers,
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
    
    pub fn remove_neighbor(&mut self, neighbor_id: u32) -> bool {
        self.neighbors.remove(&neighbor_id).is_some()
    }
    
    pub fn get_neighbor_count(&self) -> usize {
        self.neighbors.len()
    }
    
    pub fn add_router_link(&mut self, neighbor_id: u32, interface_id: u32, cost: u32) {
        // Update router links for LSA generation
        self.router_links.retain(|(n, _, _)| *n != neighbor_id);
        self.router_links.push((neighbor_id, interface_id, cost));
    }
    
    pub fn generate_router_lsa(&mut self) -> crate::router::LSA {
        let mut links = Vec::new();
        
        // Add point-to-point links for each neighbor in Full state
        for (neighbor_id, neighbor) in &self.neighbors {
            if neighbor.state == OSPFNeighborState::Full {
                // Find the link info for this neighbor
                if let Some((_, interface_id, cost)) = self.router_links.iter().find(|(n, _, _)| n == neighbor_id) {
                    // Point-to-point link
                    links.push(RouterLink {
                        link_id: format!("1.1.1.{}", neighbor_id),
                        link_data: format!("0.0.0.{}", interface_id),  // Interface ID
                        link_type: LinkType::PointToPoint,
                        num_tos: 0,
                        metric: *cost as u16,
                    });
                }
            }
        }
        
        // Create Router LSA
        let router_lsa = RouterLSA {
            flags: 0x00,  // No special flags for now
            num_links: links.len() as u16,
            links,
        };
        
        let header = crate::router::LSAHeader {
            ls_age: 0,  // New LSA
            ls_type: LSAType::RouterLSA,
            link_state_id: self.router_id.clone(),
            advertising_router: self.router_id.clone(),
            ls_sequence_number: self.lsa_sequence_number,
            ls_checksum: 0,  // Would be calculated in real implementation
            length: 20 + (router_lsa.links.len() * 12) as u16,  // Header + link data
        };
        
        // Increment sequence number for next LSA
        self.lsa_sequence_number += 1;
        
        crate::router::LSA {
            header,
            data: LSAData::Router(router_lsa),
        }
    }
    
    pub fn update_lsa_database(&mut self, lsa: crate::router::LSA) {
        let key = format!("{}:{}:{}", 
            lsa.header.ls_type.clone() as u8, 
            lsa.header.link_state_id.clone(), 
            lsa.header.advertising_router.clone()
        );
        self.lsa_database.insert(key, lsa);
    }
    
    pub fn get_lsa_headers(&self) -> Vec<crate::router::LSAHeader> {
        self.lsa_database.values().map(|lsa| lsa.header.clone()).collect()
    }
    
    pub fn get_lsa_count(&self) -> usize {
        self.lsa_database.len()
    }
    
    pub fn flood_lsa(&self, lsa: &crate::router::LSA) -> Vec<PacketEvent> {
        let mut events = Vec::new();
        
        // Create Link State Update packet with the LSA
        let lsa_for_packet = LSA {
            header: LSAHeader {
                age: lsa.header.ls_age,
                options: 0x02,
                lsa_type: lsa.header.ls_type.clone() as u8,
                link_state_id: lsa.header.link_state_id.clone(),
                advertising_router: lsa.header.advertising_router.clone(),
                sequence_number: lsa.header.ls_sequence_number,
                checksum: lsa.header.ls_checksum,
                length: lsa.header.length,
            },
            data: format!("{:?}", lsa.data),  // Serialize LSA data for simplicity
        };
        
        let lsu_packet = LinkStateUpdatePacket {
            lsas: vec![lsa_for_packet],
        };
        
        // Send to all neighbors in Exchange or Full state
        for (neighbor_id, neighbor) in &self.neighbors {
            if matches!(neighbor.state, OSPFNeighborState::Exchange | OSPFNeighborState::Full) {
                let ospf_packet = OSPFPacket {
                    version: 2,
                    packet_type: OSPFPacketType::LinkStateUpdate,
                    router_id: self.router_id.clone(),
                    area_id: self.area_id.clone(),
                    checksum: 0,
                    auth_type: 0,
                    authentication: 0,
                    data: OSPFPacketData::LinkStateUpdate(lsu_packet.clone()),
                };
                
                events.push(PacketEvent {
                    timestamp: 0.0,  // Will be set by scheduler
                    from_router_id: self.router_id.parse::<f64>().unwrap_or(0.0) as u32,
                    to_router_id: *neighbor_id,
                    packet: ProtocolPacket::OSPF(ospf_packet),
                });
            }
        }
        
        events
    }
    
    pub fn process_lsu_packet(&mut self, packet: &LinkStateUpdatePacket, from_router_id: u32) -> Vec<PacketEvent> {
        let mut events = Vec::new();
        let mut updated_lsas = Vec::new();
        
        for lsa in &packet.lsas {
            let key = format!("{}:{}:{}", 
                lsa.header.lsa_type, 
                lsa.header.link_state_id, 
                lsa.header.advertising_router
            );
            
            let should_update = if let Some(existing_lsa) = self.lsa_database.get(&key) {
                // Compare sequence numbers - higher is newer
                lsa.header.sequence_number > existing_lsa.header.ls_sequence_number
            } else {
                // New LSA
                true
            };
            
            if should_update {
                // Convert from OSPF packet LSA to router LSA
                let router_lsa_header = RouterLSAHeader {
                    ls_age: lsa.header.age,
                    ls_type: match lsa.header.lsa_type {
                        1 => LSAType::RouterLSA,
                        2 => LSAType::NetworkLSA,
                        3 => LSAType::SummaryLSA,
                        5 => LSAType::ASExternalLSA,
                        _ => LSAType::RouterLSA,
                    },
                    link_state_id: lsa.header.link_state_id.clone(),
                    advertising_router: lsa.header.advertising_router.clone(),
                    ls_sequence_number: lsa.header.sequence_number,
                    ls_checksum: lsa.header.checksum,
                    length: lsa.header.length,
                };
                
                // For now, create a simple Router LSA
                let lsa_data = if lsa.header.lsa_type == 1 {
                    LSAData::Router(RouterLSA {
                        flags: 0,
                        num_links: 0,
                        links: Vec::new(),
                    })
                } else {
                    // Default to Router LSA for simplicity
                    LSAData::Router(RouterLSA {
                        flags: 0,
                        num_links: 0,
                        links: Vec::new(),
                    })
                };
                
                let router_lsa = crate::router::LSA {
                    header: router_lsa_header,
                    data: lsa_data,
                };
                
                self.update_lsa_database(router_lsa.clone());
                updated_lsas.push(router_lsa);
            }
        }
        
        // Flood updated LSAs to other neighbors (except the one who sent it)
        for lsa in &updated_lsas {
            let flood_events = self.flood_lsa(lsa);
            for event in flood_events {
                if event.to_router_id != from_router_id {
                    events.push(event);
                }
            }
        }
        
        events
    }
}
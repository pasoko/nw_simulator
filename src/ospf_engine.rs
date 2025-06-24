use std::collections::HashMap;
use crate::ospf::{OSPFPacket, OSPFPacketType, OSPFPacketData, HelloPacket, DatabaseDescriptionPacket, LSA, LSAHeader, LinkStateUpdatePacket, LinkStateRequestPacket, LSARequest, LinkStateAcknowledgmentPacket};
use crate::router::{OSPFNeighbor, OSPFNeighborState, RouterLSA, RouterLink, LinkType, LSAType, LSAData, LSAHeader as RouterLSAHeader};
use crate::protocol::{ProtocolPacket, PacketEvent};
use crate::console_log;

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
    dd_sequence_number: u32,  // DD sequence number for exchange
    neighbor_dd_state: HashMap<u32, DDExchangeState>,  // Track DD exchange state per neighbor
    neighbor_previous_state: HashMap<u32, OSPFNeighborState>,  // Track previous state for logging
}

#[derive(Clone)]
struct DDExchangeState {
    dd_seq_num: u32,
    is_master: bool,
    last_received_dd_seq: u32,
    lsa_headers_to_request: Vec<crate::router::LSAHeader>,
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
            dd_sequence_number: 0x80000001,
            neighbor_dd_state: HashMap::new(),
            neighbor_previous_state: HashMap::new(),
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
            let time_since_hello = self.current_time - last_hello;
            if time_since_hello > self.dead_interval as f64 {
                dead_neighbors.push(*id);
                console_log!("Router {} marking neighbor {} as dead - last hello {:.1}s ago (dead interval: {}s)",
                    self.router_id, id, time_since_hello, self.dead_interval);
            }
        }
        
        for id in dead_neighbors {
            if let Some(neighbor) = self.neighbors.get_mut(&id) {
                // Only log if neighbor was not already Down
                if neighbor.state != OSPFNeighborState::Down {
                    console_log!("Router {} neighbor {} state changed to Down due to dead timer",
                        self.router_id, id);
                }
                neighbor.state = OSPFNeighborState::Down;
            }
            self.neighbor_last_hello.remove(&id);
        }
    }

    pub fn process_hello_packet(&mut self, packet: &HelloPacket, from_router_id: u32, interface_id: u32) -> Vec<PacketEvent> {
        let mut events = Vec::new();
        
        // Update last hello time
        self.neighbor_last_hello.insert(from_router_id, self.current_time);
        console_log!("Router {} received Hello from router {} at time {:.1}s",
            self.router_id, from_router_id, self.current_time);
        
        // Check if we already know this neighbor
        let (current_state, _is_new) = if let Some(neighbor) = self.neighbors.get(&from_router_id) {
            // Store previous state before updating
            let prev_state = neighbor.state.clone();
            let neighbor_state = neighbor.state.clone();
            self.neighbor_previous_state.insert(from_router_id, prev_state);
            (neighbor_state, false)
        } else {
            // New neighbor discovered
            self.neighbor_previous_state.insert(from_router_id, OSPFNeighborState::Down);
            let new_neighbor = OSPFNeighbor {
                router_id: format!("{}.{}.{}.{}", 1, 1, 1, from_router_id),
                state: OSPFNeighborState::Down,
                interface_id,
                priority: packet.router_priority,
            };
            self.neighbors.insert(from_router_id, new_neighbor);
            (OSPFNeighborState::Down, true)
        };
        
        // Update neighbor priority if exists
        if let Some(neighbor) = self.neighbors.get_mut(&from_router_id) {
            neighbor.priority = packet.router_priority;
        }

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
                        
                        // Decide whether to form adjacency (for now, always form)
                        // In real OSPF, this depends on network type and DR/BDR election
                        self.decide_adjacency(from_router_id, &mut events);
                    }
                }
            }
            OSPFNeighborState::TwoWay => {
                // Stay in TwoWay unless we decide to form adjacency
                // For point-to-point links, always form adjacency
                self.decide_adjacency(from_router_id, &mut events);
            }
            _ => {
                // Maintain current state
            }
        }

        events
    }

    pub fn process_dd_packet(&mut self, packet: &DatabaseDescriptionPacket, from_router_id: u32) -> Vec<PacketEvent> {
        let mut events = Vec::new();
        
        // Get neighbor state first
        let neighbor_state = self.neighbors.get(&from_router_id).map(|n| n.state.clone());
        
        match neighbor_state {
            Some(OSPFNeighborState::ExStart) => {
                // Master/Slave negotiation
                let our_router_id_num = self.router_id.split('.').last().unwrap_or("0").parse::<u32>().unwrap_or(0);
                let is_master = our_router_id_num > from_router_id;
                
                // Initialize DD exchange state
                let dd_state = DDExchangeState {
                    dd_seq_num: if is_master { self.dd_sequence_number } else { packet.dd_sequence_number },
                    is_master,
                    last_received_dd_seq: packet.dd_sequence_number,
                    lsa_headers_to_request: Vec::new(),
                };
                self.neighbor_dd_state.insert(from_router_id, dd_state);
                
                // Update neighbor state
                if let Some(neighbor) = self.neighbors.get_mut(&from_router_id) {
                    neighbor.state = OSPFNeighborState::Exchange;
                }
                
                // Send our database summary
                events.push(self.create_dd_packet_event(from_router_id));
            }
            Some(OSPFNeighborState::Exchange) => {
                    // Variables to track state changes outside of borrow
                    let mut should_send_dd = false;
                    let mut should_move_to_full = false;
                    let mut should_move_to_loading = false;
                    
                    // Process received LSA headers
                    if let Some(dd_state) = self.neighbor_dd_state.get_mut(&from_router_id) {
                        // Check for LSAs we don't have
                        for lsa_header in &packet.lsa_headers {
                            let key = format!("{}:{}:{}", 
                                lsa_header.lsa_type, 
                                lsa_header.link_state_id, 
                                lsa_header.advertising_router
                            );
                            
                            let need_lsa = if let Some(our_lsa) = self.lsa_database.get(&key) {
                                lsa_header.sequence_number > our_lsa.header.ls_sequence_number
                            } else {
                                true
                            };
                            
                            if need_lsa {
                                // Convert OSPF LSA header to Router LSA header
                                let router_lsa_header = crate::router::LSAHeader {
                                    ls_age: lsa_header.age,
                                    ls_type: match lsa_header.lsa_type {
                                        1 => LSAType::RouterLSA,
                                        2 => LSAType::NetworkLSA,
                                        3 => LSAType::SummaryLSA,
                                        5 => LSAType::ASExternalLSA,
                                        _ => LSAType::RouterLSA,
                                    },
                                    link_state_id: lsa_header.link_state_id.clone(),
                                    advertising_router: lsa_header.advertising_router.clone(),
                                    ls_sequence_number: lsa_header.sequence_number,
                                    ls_checksum: lsa_header.checksum,
                                    length: lsa_header.length,
                                };
                                dd_state.lsa_headers_to_request.push(router_lsa_header);
                            }
                        }
                        
                        // Check if DD exchange is complete
                        let more_flag = packet.flags & 0x02 != 0;  // M bit
                        let init_flag = packet.flags & 0x04 != 0;  // I bit
                        
                        console_log!("Router {} received DD from {}: M={}, I={}, seq={}", 
                            self.router_id, from_router_id, more_flag, init_flag, packet.dd_sequence_number);
                        
                        // Send our DD packet in response (if we're slave or need to acknowledge)
                        should_send_dd = !dd_state.is_master || (dd_state.is_master && more_flag);
                        
                        // If this is the final DD packet from neighbor
                        if !more_flag && !init_flag && dd_state.lsa_headers_to_request.is_empty() {
                            console_log!("Router {} moving neighbor {} to Full state", self.router_id, from_router_id);
                            should_move_to_full = true;
                        } else if !more_flag && !dd_state.lsa_headers_to_request.is_empty() {
                            should_move_to_loading = true;
                        }
                    }
                    
                    // Apply state changes outside of borrow
                    if should_send_dd {
                        events.push(self.create_dd_packet_event(from_router_id));
                    }
                    
                    if should_move_to_full {
                        if let Some(neighbor) = self.neighbors.get_mut(&from_router_id) {
                            neighbor.state = OSPFNeighborState::Full;
                        }
                        
                        // Generate Router LSA when adjacency forms
                        let router_lsa = self.generate_router_lsa();
                        
                        // Debug: Log LSA generation details
                        if let LSAData::Router(ref rlsa) = router_lsa.data {
                            console_log!("Router {} generated Router LSA with {} links", 
                                self.router_id, rlsa.links.len());
                            for link in &rlsa.links {
                                console_log!("  Link to {} via interface {}, metric {}", 
                                    link.link_id, link.link_data, link.metric);
                            }
                        }
                        
                        let lsa_clone = router_lsa.clone();
                        self.update_lsa_database(router_lsa);
                        
                        // Flood the new LSA to neighbors
                        console_log!("Router {} flooding LSA to neighbors", self.router_id);
                        let flood_events = self.flood_lsa(&lsa_clone);
                        events.extend(flood_events);
                    } else if should_move_to_loading {
                        // Move to Loading state to request LSAs
                        if let Some(neighbor) = self.neighbors.get_mut(&from_router_id) {
                            neighbor.state = OSPFNeighborState::Loading;
                        }
                        
                        // Send Link State Request
                        events.push(self.create_lsr_packet_event(from_router_id));
                    }
            }
            Some(OSPFNeighborState::Loading) => {
                // Continue processing while in Loading state
                // In real implementation, would track which LSAs are still needed
            }
            _ => {}
        }
        
        events
    }

    pub fn generate_hello_packet(&self) -> HelloPacket {
        // Include all neighbors that are not Down in the hello packet
        let neighbor_list: Vec<String> = self.neighbors.iter()
            .filter(|(_, n)| n.state != OSPFNeighborState::Down)
            .map(|(id, _)| format!("{}.{}.{}.{}", 1, 1, 1, id))
            .collect();
        
        console_log!("Router {} generating Hello packet with {} neighbors: {:?}", 
            self.router_id, neighbor_list.len(), neighbor_list);
        
        HelloPacket {
            network_mask: "255.255.255.252".to_string(),
            hello_interval: self.hello_interval,
            options: 0x02, // E-bit set
            router_priority: 1,
            router_dead_interval: self.dead_interval,
            designated_router: "0.0.0.0".to_string(),
            backup_designated_router: "0.0.0.0".to_string(),
            neighbors: neighbor_list,
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
        
        // Determine DD flags based on state
        let mut flags = 0u8;
        
        // Check if we have DD state for this neighbor
        if let Some(dd_state) = self.neighbor_dd_state.get(&to_router_id) {
            // We're in Exchange state
            if dd_state.is_master {
                flags |= 0x01; // MS bit
            }
            // Don't set M bit since we have no more data to send
            // Don't set I bit for subsequent DD packets
        } else {
            // First DD packet (ExStart state)
            flags = 0x07; // I, M, MS bits all set for initial negotiation
        }
        
        let dd_packet = DatabaseDescriptionPacket {
            interface_mtu: 1500,
            options: 0x02,
            flags,
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
        
        // Extract router ID from IP address (e.g., "1.1.1.5" -> 5)
        let from_id = self.router_id.split('.').last()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(1);
        
        PacketEvent {
            timestamp: 0.0, // Will be set by scheduler
            from_router_id: from_id,
            to_router_id,
            packet: ProtocolPacket::OSPF(ospf_packet),
        }
    }

    
    pub fn get_neighbor_state_transitions(&self) -> HashMap<u32, (OSPFNeighborState, OSPFNeighborState)> {
        self.neighbors.iter()
            .filter_map(|(id, neighbor)| {
                self.neighbor_previous_state.get(id)
                    .map(|prev| (*id, (prev.clone(), neighbor.state.clone())))
            })
            .collect()
    }
    
    pub fn remove_neighbor(&mut self, neighbor_id: u32) -> bool {
        self.neighbors.remove(&neighbor_id).is_some()
    }
    
    pub fn get_neighbor_count(&self) -> usize {
        self.neighbors.len()
    }
    
    fn decide_adjacency(&mut self, neighbor_id: u32, events: &mut Vec<PacketEvent>) {
        // For point-to-point links, always form adjacency
        // In real OSPF, this would depend on network type and DR/BDR election
        if let Some(neighbor) = self.neighbors.get_mut(&neighbor_id) {
            if neighbor.state == OSPFNeighborState::TwoWay {
                neighbor.state = OSPFNeighborState::ExStart;
                events.push(self.create_dd_packet_event(neighbor_id));
            }
        }
    }
    
    pub fn add_router_link(&mut self, neighbor_id: u32, interface_id: u32, cost: u32) {
        // Update router links for LSA generation
        self.router_links.retain(|(n, _, _)| *n != neighbor_id);
        self.router_links.push((neighbor_id, interface_id, cost));
    }
    
    pub fn generate_router_lsa(&mut self) -> crate::router::LSA {
        let mut links = Vec::new();
        
        // Add point-to-point links for ALL configured links
        // Include all physical links in the LSA, regardless of OSPF neighbor state
        for (neighbor_id, interface_id, cost) in &self.router_links {
            // Always include the link - it's a physical connection
            links.push(RouterLink {
                link_id: format!("1.1.1.{}", neighbor_id),
                link_data: format!("0.0.0.{}", interface_id),  // Interface ID
                link_type: LinkType::PointToPoint,
                num_tos: 0,
                metric: *cost as u16,
            });
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
        
        // Debug: Log LSA database update
        console_log!("Router {} updating LSA database with key: {}", self.router_id, key);
        if let LSAData::Router(ref rlsa) = lsa.data {
            console_log!("  Router LSA with {} links", rlsa.links.len());
        }
        
        self.lsa_database.insert(key, lsa);
        console_log!("  LSA database now contains {} entries", self.lsa_database.len());
    }
    
    
    pub fn get_lsa_count(&self) -> usize {
        self.lsa_database.len()
    }
    
    pub fn get_lsa_database(&self) -> &HashMap<String, crate::router::LSA> {
        &self.lsa_database
    }
    
    pub fn flood_lsa(&self, lsa: &crate::router::LSA) -> Vec<PacketEvent> {
        let mut events = Vec::new();
        
        // Debug: Log LSA flooding
        console_log!("Router {} flooding LSA: type={}, id={}, adv_router={}", 
            self.router_id, 
            lsa.header.ls_type.clone() as u8,
            lsa.header.link_state_id,
            lsa.header.advertising_router
        );
        
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
            data: lsa.data.clone()
        };
        
        let lsu_packet = LinkStateUpdatePacket {
            lsas: vec![lsa_for_packet],
        };
        
        // Send to all neighbors in Exchange or Full state
        let eligible_neighbors: Vec<u32> = self.neighbors.iter()
            .filter(|(_, n)| matches!(n.state, OSPFNeighborState::Exchange | OSPFNeighborState::Full))
            .map(|(id, _)| *id)
            .collect();
        
        console_log!("  Flooding to {} eligible neighbors: {:?}", eligible_neighbors.len(), eligible_neighbors);
        
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
                
                // Extract router ID from IP address (e.g., "1.1.1.5" -> 5)
                let from_id = self.router_id.split('.').last()
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(1);
                
                events.push(PacketEvent {
                    timestamp: 0.0,  // Will be set by scheduler
                    from_router_id: from_id,
                    to_router_id: *neighbor_id,
                    packet: ProtocolPacket::OSPF(ospf_packet),
                });
            }
        }
        
        events
    }
    
    fn create_lsr_packet_event(&self, to_router_id: u32) -> PacketEvent {
        let mut requests = Vec::new();
        
        if let Some(dd_state) = self.neighbor_dd_state.get(&to_router_id) {
            for lsa_header in &dd_state.lsa_headers_to_request {
                requests.push(LSARequest {
                    lsa_type: lsa_header.ls_type.clone() as u8,
                    link_state_id: lsa_header.link_state_id.clone(),
                    advertising_router: lsa_header.advertising_router.clone(),
                });
            }
        }
        
        let lsr_packet = LinkStateRequestPacket { requests };
        
        let ospf_packet = OSPFPacket {
            version: 2,
            packet_type: OSPFPacketType::LinkStateRequest,
            router_id: self.router_id.clone(),
            area_id: self.area_id.clone(),
            checksum: 0,
            auth_type: 0,
            authentication: 0,
            data: OSPFPacketData::LinkStateRequest(lsr_packet),
        };
        
        // Extract router ID from IP address (e.g., "1.1.1.5" -> 5)
        let from_id = self.router_id.split('.').last()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(1);
        
        PacketEvent {
            timestamp: 0.0,  // Will be set by scheduler
            from_router_id: from_id,
            to_router_id,
            packet: ProtocolPacket::OSPF(ospf_packet),
        }
    }
    
    pub fn process_lsr_packet(&mut self, packet: &LinkStateRequestPacket, from_router_id: u32) -> Vec<PacketEvent> {
        let mut events = Vec::new();
        let mut lsas_to_send = Vec::new();
        
        // Find requested LSAs in our database
        for request in &packet.requests {
            let key = format!("{}:{}:{}", 
                request.lsa_type, 
                request.link_state_id, 
                request.advertising_router
            );
            
            if let Some(lsa) = self.lsa_database.get(&key) {
                // Convert to OSPF packet LSA format
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
                    data: lsa.data.clone()
                };
                lsas_to_send.push(lsa_for_packet);
            }
        }
        
        // Send Link State Update with requested LSAs
        if !lsas_to_send.is_empty() {
            let lsu_packet = LinkStateUpdatePacket { lsas: lsas_to_send };
            
            let ospf_packet = OSPFPacket {
                version: 2,
                packet_type: OSPFPacketType::LinkStateUpdate,
                router_id: self.router_id.clone(),
                area_id: self.area_id.clone(),
                checksum: 0,
                auth_type: 0,
                authentication: 0,
                data: OSPFPacketData::LinkStateUpdate(lsu_packet),
            };
            
            // Extract router ID from IP address (e.g., "1.1.1.5" -> 5)
            let from_id = self.router_id.split('.').last()
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(1);
            
            events.push(PacketEvent {
                timestamp: 0.0,
                from_router_id: from_id,
                to_router_id: from_router_id,
                packet: ProtocolPacket::OSPF(ospf_packet),
            });
        }
        
        events
    }
    
    pub fn process_lsack_packet(&mut self, _packet: &LinkStateAcknowledgmentPacket, _from_router_id: u32) -> Vec<PacketEvent> {
        // Process acknowledgments - for now, just mark LSAs as acknowledged
        // In a full implementation, would track retransmission timers
        Vec::new()
    }
    
    pub fn process_lsu_packet(&mut self, packet: &LinkStateUpdatePacket, from_router_id: u32) -> Vec<PacketEvent> {
        let mut events = Vec::new();
        let mut updated_lsas = Vec::new();
        let mut ack_headers = Vec::new();
        
        console_log!("Router {} processing LSU from router {} with {} LSAs", 
            self.router_id, from_router_id, packet.lsas.len());
        
        for lsa in &packet.lsas {
            let key = format!("{}:{}:{}", 
                lsa.header.lsa_type, 
                lsa.header.link_state_id, 
                lsa.header.advertising_router
            );
            
            let should_update = if let Some(existing_lsa) = self.lsa_database.get(&key) {
                // Compare sequence numbers - higher is newer
                let is_newer = lsa.header.sequence_number > existing_lsa.header.ls_sequence_number;
                console_log!("  Existing LSA found with seq {}, new seq {} - update: {}", 
                    existing_lsa.header.ls_sequence_number, lsa.header.sequence_number, is_newer);
                is_newer
            } else {
                // New LSA
                console_log!("  New LSA (key: {})", key);
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
                
                // LSA data is already properly typed
                let lsa_data = lsa.data.clone();
                
                // Debug logging
                match &lsa_data {
                    LSAData::Router(router_lsa) => {
                        console_log!("  Router LSA with {} links", router_lsa.links.len());
                        for link in &router_lsa.links {
                            console_log!("    Link: {} -> {} (metric {})", 
                                link.link_id, link.link_data, link.metric);
                        }
                    },
                    LSAData::Network(_) => console_log!("  Network LSA"),
                    LSAData::Summary(_) => console_log!("  Summary LSA"),
                    LSAData::ASExternal(_) => console_log!("  AS-External LSA"),
                }
                
                let router_lsa = crate::router::LSA {
                    header: router_lsa_header,
                    data: lsa_data,
                };
                
                self.update_lsa_database(router_lsa.clone());
                updated_lsas.push(router_lsa);
                ack_headers.push(lsa.header.clone());
            }
        }
        
        // Send Link State Acknowledgment
        if !ack_headers.is_empty() {
            let lsack_packet = LinkStateAcknowledgmentPacket { lsa_headers: ack_headers };
            
            let ospf_packet = OSPFPacket {
                version: 2,
                packet_type: OSPFPacketType::LinkStateAcknowledgment,
                router_id: self.router_id.clone(),
                area_id: self.area_id.clone(),
                checksum: 0,
                auth_type: 0,
                authentication: 0,
                data: OSPFPacketData::LinkStateAcknowledgment(lsack_packet),
            };
            
            // Extract router ID from IP address (e.g., "1.1.1.5" -> 5)
            let from_id = self.router_id.split('.').last()
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(1);
            
            events.push(PacketEvent {
                timestamp: 0.0,
                from_router_id: from_id,
                to_router_id: from_router_id,
                packet: ProtocolPacket::OSPF(ospf_packet),
            });
        }
        
        // Check if we were in Loading state and can now move to Full
        if let Some(neighbor) = self.neighbors.get_mut(&from_router_id) {
            if neighbor.state == OSPFNeighborState::Loading {
                // Check if we have received all requested LSAs
                if let Some(dd_state) = self.neighbor_dd_state.get_mut(&from_router_id) {
                    // Remove received LSAs from request list
                    for lsa in &packet.lsas {
                        dd_state.lsa_headers_to_request.retain(|req| {
                            !(req.ls_type.clone() as u8 == lsa.header.lsa_type &&
                              req.link_state_id == lsa.header.link_state_id &&
                              req.advertising_router == lsa.header.advertising_router)
                        });
                    }
                    
                    // If all LSAs received, move to Full
                    if dd_state.lsa_headers_to_request.is_empty() {
                        neighbor.state = OSPFNeighborState::Full;
                        
                        // Generate Router LSA when adjacency forms
                        let router_lsa = self.generate_router_lsa();
                        
                        // Debug: Log LSA generation details
                        if let LSAData::Router(ref rlsa) = router_lsa.data {
                            console_log!("Router {} generated Router LSA (from Loading) with {} links", 
                                self.router_id, rlsa.links.len());
                        }
                        
                        let lsa_clone = router_lsa.clone();
                        self.update_lsa_database(router_lsa);
                        
                        // Flood the new LSA to neighbors
                        let flood_events = self.flood_lsa(&lsa_clone);
                        events.extend(flood_events);
                    }
                }
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
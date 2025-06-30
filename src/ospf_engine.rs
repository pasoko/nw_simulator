use std::collections::HashMap;
use crate::ospf::{HelloPacket, DatabaseDescriptionPacket, 
    LinkStateRequestPacket, LinkStateUpdatePacket, LinkStateAcknowledgmentPacket,
    OSPFPacket, OSPFPacketType, OSPFPacketData};
use crate::router::{OSPFNeighborState, LSA as RouterLSA};
use crate::protocol::{PacketEvent, ProtocolPacket};
use crate::ospf_neighbor::OSPFNeighborManager;
use crate::ospf_lsa_manager::OSPFLSAManager;
use crate::ospf_packet_processor::OSPFPacketProcessor;
use crate::ospf_timer::{OSPFTimerManager, OSPFTimerEvent};
use crate::ospf_checksum::verify_lsa_checksum;
use crate::console_log;

/// Refactored OSPF Engine
/// 
/// A simplified OSPF engine that delegates responsibilities to specialized components:
/// - OSPFNeighborManager: Neighbor state management
/// - OSPFLSAManager: LSA database management
/// - OSPFPacketProcessor: Packet processing
/// - OSPFTimerManager: Timer management
pub struct OSPFEngine {
    router_id: String,
    area_id: String,
    current_time: f64,
    spf_calculation_pending: bool,  // RFC 2328 Section 16.1 - Track if SPF is scheduled
    
    // Specialized components
    neighbor_manager: OSPFNeighborManager,
    lsa_manager: OSPFLSAManager,
    packet_processor: OSPFPacketProcessor,
    timer_manager: OSPFTimerManager,
}

impl OSPFEngine {
    pub fn new(router_id: String, area_id: String) -> Self {
        let mut timer_manager = OSPFTimerManager::new(router_id.clone());
        timer_manager.start_hello_timer();  // Start hello timer immediately
        
        OSPFEngine {
            neighbor_manager: OSPFNeighborManager::new(40), // 40s dead interval
            lsa_manager: OSPFLSAManager::new(router_id.clone()),
            packet_processor: OSPFPacketProcessor::new(router_id.clone(), area_id.clone()),
            timer_manager,
            router_id,
            area_id,
            current_time: 0.0,
            spf_calculation_pending: false,
        }
    }
    
    pub fn update_time(&mut self, time: f64) -> Vec<PacketEvent> {
        let time_delta = if self.current_time > 0.0 { time - self.current_time } else { 0.0 };
        self.current_time = time;
        
        self.neighbor_manager.update_time(time);
        self.timer_manager.update_time(time);
        self.lsa_manager.update_time(time);
        
        // Age LSAs and handle MaxAge reflooding
        let maxage_lsas = self.lsa_manager.age_lsas(time_delta);
        
        let mut events = Vec::new();
        
        // Reflood MaxAge LSAs before deletion
        for lsa in maxage_lsas {
            console_log!("Router {} reflooding MaxAge LSA: {}:{}", 
                self.router_id, lsa.header.link_state_id, lsa.header.advertising_router);
            let flood_events = self.flood_lsa(&lsa);
            events.extend(flood_events);
        }
        
        // Process expired timers
        let expired_events = self.timer_manager.process_expired_timers();
        if !expired_events.is_empty() {
            console_log!("Router {} checking timers at {:.1}s, found {} expired events", 
                self.router_id, time, expired_events.len());
        }
        
        for event in expired_events {
            match event {
                OSPFTimerEvent::HelloTimer => {
                    console_log!("Router {} hello timer expired at {:.1}s", self.router_id, time);
                    // Generate Hello packets for all neighbors
                    let hello_events = self.generate_hello_events();
                    console_log!("Router {} scheduling {} hello packets", self.router_id, hello_events.len());
                    events.extend(hello_events);
                }
                OSPFTimerEvent::DeadTimer(neighbor_id) => {
                    console_log!("Router {} dead timer expired for neighbor {}", 
                        self.router_id, neighbor_id);
                    self.neighbor_manager.remove_neighbor(neighbor_id);
                }
                OSPFTimerEvent::LSARefresh => {
                    console_log!("Router {} LSA refresh timer expired", self.router_id);
                    // Generate new Router LSA
                    let lsa = self.lsa_manager.regenerate_router_lsa();
                    if self.neighbor_manager.get_neighbor_count() > 0 {
                        let flood_events = self.flood_lsa(&lsa);
                        events.extend(flood_events);
                    }
                }
                OSPFTimerEvent::RetransmissionTimer(neighbor_id) => {
                    console_log!("Router {} retransmission timer expired for neighbor {}", 
                        self.router_id, neighbor_id);
                    // Handle retransmission logic
                }
                OSPFTimerEvent::DDRetransmissionTimer(neighbor_id) => {
                    console_log!("Router {} DD retransmission timer expired for neighbor {}", 
                        self.router_id, neighbor_id);
                    // Handle DD retransmission as per RFC 2328 Section 10.8
                    if let Some(dd_packet) = self.packet_processor.get_last_dd_packet(neighbor_id) {
                        console_log!("Router {} retransmitting DD packet to neighbor {}", 
                            self.router_id, neighbor_id);
                        let event = self.packet_processor.create_dd_retransmit_event(neighbor_id, dd_packet);
                        events.push(event);
                        // Restart the DD retransmission timer
                        self.timer_manager.start_dd_retransmission_timer(neighbor_id);
                    }
                }
                OSPFTimerEvent::SPFDelay => {
                    console_log!("Router {} SPF delay timer expired, calculation can proceed", 
                        self.router_id);
                    self.spf_calculation_pending = false;
                    // The actual SPF calculation will be triggered by the simulation layer
                }
            }
        }
        
        events
    }
    
    pub fn process_hello_packet(&mut self, packet: &HelloPacket, from_router_id: u32, interface_id: u32) -> Vec<PacketEvent> {
        let mut events = Vec::new();
        
        // Get current neighbor state before processing
        let current_state = self.neighbor_manager.get_neighbor_state(from_router_id);
        
        // Add or update neighbor
        let _is_new = self.neighbor_manager.add_or_update_neighbor(from_router_id, interface_id, packet.router_priority);
        
        // Process hello packet
        let (should_process, hello_neighbors) = self.packet_processor.process_hello_packet(packet, from_router_id);
        
        if should_process {
            // Start/reset dead timer
            self.timer_manager.reset_neighbor_dead_timer(from_router_id);
            
            // Only progress state if neighbor is in Down, Init or TwoWay
            // Don't regress from higher states (ExStart, Exchange, Loading, Full)
            match current_state {
                None | Some(OSPFNeighborState::Down) | Some(OSPFNeighborState::Init) | Some(OSPFNeighborState::TwoWay) => {
                    // Progress neighbor state
                    let state_changed = self.neighbor_manager.progress_neighbor_state(
                        from_router_id, 
                        &hello_neighbors, 
                        &self.router_id
                    );
                    
                    // Check if should form adjacency
                    if state_changed && self.neighbor_manager.should_form_adjacency(from_router_id) {
                        console_log!("Router {} neighbor {} is in TwoWay state, starting adjacency formation", 
                            self.router_id, from_router_id);
                        if self.neighbor_manager.start_adjacency(from_router_id) {
                            console_log!("Router {} neighbor {} moved to ExStart", 
                                self.router_id, from_router_id);
                            
                            // Generate initial LSA if we don't have one yet
                            if self.lsa_manager.get_lsa_count() == 0 && self.lsa_manager.get_router_links().len() > 0 {
                                console_log!("Router {} generating initial Router LSA before DD exchange", self.router_id);
                                let lsa = self.lsa_manager.generate_router_lsa();
                                console_log!("Router {} generated Router LSA with {} links, database now has {} LSAs", 
                                    self.router_id, self.lsa_manager.get_router_links().len(), self.lsa_manager.get_lsa_count());
                                console_log!("  LSA: Type={:?}, ID={}, AdvRouter={}, SeqNum={}", 
                                    lsa.header.ls_type, lsa.header.link_state_id, 
                                    lsa.header.advertising_router, lsa.header.ls_sequence_number);
                                
                                // Immediately flood this LSA to neighbors in ExStart or higher state
                                let exchange_neighbors = self.neighbor_manager.get_neighbors_in_state(OSPFNeighborState::ExStart);
                                if !exchange_neighbors.is_empty() {
                                    console_log!("Router {} flooding initial LSA to {} neighbors in ExStart", 
                                        self.router_id, exchange_neighbors.len());
                                    let flood_events = self.flood_lsa(&lsa);
                                    events.extend(flood_events);
                                }
                            }
                            
                            // Send Database Description packet
                            let dd_event = self.packet_processor.create_dd_packet_event(
                                from_router_id, 
                                self.lsa_manager.get_lsa_database()
                            );
                            events.push(dd_event);
                            
                            // Start DD retransmission timer (RFC 2328 Section 10.8)
                            if self.packet_processor.should_start_dd_retransmit(from_router_id) {
                                self.timer_manager.start_dd_retransmission_timer(from_router_id);
                                console_log!("Router {} started DD retransmission timer for neighbor {}", 
                                    self.router_id, from_router_id);
                            }
                        }
                    }
                }
                _ => {
                    // For higher states, just maintain bidirectional communication
                    if !hello_neighbors.contains(&self.router_id) {
                        console_log!("Warning: Router {} not in neighbor {}'s hello packet while in state {:?}", 
                            self.router_id, from_router_id, current_state);
                    }
                }
            }
        }
        
        events
    }
    
    pub fn process_dd_packet(&mut self, packet: &DatabaseDescriptionPacket, from_router_id: u32) -> Vec<PacketEvent> {
        let mut events = Vec::new();
        
        console_log!("Router {} processing DD packet from {} (current LSA count: {})", 
            self.router_id, from_router_id, self.lsa_manager.get_lsa_count());
        
        if let Some(current_state) = self.neighbor_manager.get_neighbor_state(from_router_id) {
            let (new_state, should_send_dd, lsa_headers_to_request) = self.packet_processor.process_dd_packet(
                packet, 
                from_router_id, 
                current_state
            );
            
            // Stop DD retransmission timer if we received acknowledgment
            if !self.packet_processor.should_start_dd_retransmit(from_router_id) {
                self.timer_manager.stop_dd_retransmission_timer(from_router_id);
            }
            
            // Update neighbor state if needed
            if let Some(state) = new_state {
                self.neighbor_manager.update_neighbor_state(from_router_id, state.clone());
                
                match state {
                    OSPFNeighborState::Full => {
                        console_log!("Router {} neighbor {} reached Full state", 
                            self.router_id, from_router_id);
                        
                        // When first neighbor reaches Full state, flood our LSA
                        let full_neighbor_count = self.neighbor_manager.get_neighbors_in_state(OSPFNeighborState::Full).len();
                        console_log!("Router {} now has {} Full neighbors", self.router_id, full_neighbor_count);
                        
                        if full_neighbor_count == 1 && self.lsa_manager.get_lsa_count() > 0 {
                            // This is our first Full neighbor - flood our existing LSA
                            console_log!("Router {} flooding existing LSAs to first Full neighbor", self.router_id);
                            let lsa_database = self.lsa_manager.get_lsa_database();
                            for (key, lsa) in lsa_database {
                                if lsa.header.advertising_router == self.router_id {
                                    console_log!("Router {} flooding own LSA: {}", self.router_id, key);
                                    let flood_events = self.flood_lsa(lsa);
                                    events.extend(flood_events);
                                }
                            }
                        }
                    }
                    OSPFNeighborState::Loading => {
                        // Send LSA requests
                        if !lsa_headers_to_request.is_empty() {
                            console_log!("Router {} entering Loading state, requesting {} LSAs from neighbor {}", 
                                self.router_id, lsa_headers_to_request.len(), from_router_id);
                            events.push(self.packet_processor.create_lsr_packet_event(
                                from_router_id, 
                                &lsa_headers_to_request
                            ));
                        }
                    }
                    _ => {}
                }
            }
            
            // Send DD packet if needed
            if should_send_dd {
                let dd_event = self.packet_processor.create_dd_packet_event(
                    from_router_id, 
                    self.lsa_manager.get_lsa_database()
                );
                events.push(dd_event);
                
                // Start DD retransmission timer if sending new DD packet
                if self.packet_processor.should_start_dd_retransmit(from_router_id) {
                    self.timer_manager.start_dd_retransmission_timer(from_router_id);
                }
            }
        }
        
        events
    }
    
    pub fn process_lsr_packet(&mut self, packet: &LinkStateRequestPacket, from_router_id: u32) -> Vec<PacketEvent> {
        let mut events = Vec::new();
        
        console_log!("Router {} processing LSR from router {} with {} requests", 
            self.router_id, from_router_id, packet.requests.len());
        
        let lsas_to_send = self.packet_processor.process_lsr_packet(packet, self.lsa_manager.get_lsa_database());
        
        if !lsas_to_send.is_empty() {
            console_log!("Router {} sending LSU to router {} with {} LSAs", 
                self.router_id, from_router_id, lsas_to_send.len());
            
            // Create and send LSU packet
            let lsu_event = self.packet_processor.create_lsu_packet_event(from_router_id, &lsas_to_send);
            events.push(lsu_event);
        } else {
            console_log!("Router {} has no LSAs to send in response to LSR from router {}", 
                self.router_id, from_router_id);
        }
        
        events
    }
    
    pub fn process_lsu_packet(&mut self, packet: &LinkStateUpdatePacket, from_router_id: u32) -> Vec<PacketEvent> {
        let mut events = Vec::new();
        
        let (updated_lsas, ack_headers, neighbor_to_full) = self.packet_processor.process_lsu_packet(packet, from_router_id);
        
        // Update LSA database and track which LSAs were actually updated
        let mut lsas_updated = false;
        let mut updated_lsa_keys = Vec::new();
        let mut verified_ack_headers = Vec::new();
        
        for (idx, lsa) in updated_lsas.iter().enumerate() {
            // Verify checksum before accepting LSA
            if !verify_lsa_checksum(lsa) {
                console_log!("Router {} rejected LSA from {} due to checksum mismatch", 
                    self.router_id, from_router_id);
                continue;
            }
            
            if self.lsa_manager.should_update_lsa(lsa) {
                let key = format!("{}:{}:{}", 
                    lsa.header.ls_type.clone() as u8,
                    lsa.header.link_state_id,
                    lsa.header.advertising_router
                );
                
                // Check if we can update (MinLSInterval)
                if self.lsa_manager.was_recently_updated(&key, self.current_time) {
                    console_log!("Router {} skipping update of LSA {} due to MinLSInterval", 
                        self.router_id, key);
                    continue;
                }
                
                console_log!("Router {} updating LSA: {}", self.router_id, key);
                self.lsa_manager.update_lsa_database(lsa.clone());
                updated_lsa_keys.push(key);
                lsas_updated = true;
            }
            
            // Only acknowledge LSAs that passed checksum verification
            if idx < ack_headers.len() {
                verified_ack_headers.push(ack_headers[idx].clone());
            }
        }
        
        // Send acknowledgment for received LSAs that passed verification
        if !verified_ack_headers.is_empty() {
            console_log!("Router {} sending LSAck to router {} for {} LSAs", 
                self.router_id, from_router_id, verified_ack_headers.len());
            let lsack_event = self.packet_processor.create_lsack_packet_event(from_router_id, &verified_ack_headers);
            events.push(lsack_event);
        }
        
        // Update neighbor state if needed
        if neighbor_to_full {
            self.neighbor_manager.update_neighbor_state(from_router_id, OSPFNeighborState::Full);
            
            console_log!("Router {} neighbor {} reached Full state via LSU", 
                self.router_id, from_router_id);
            
            // Don't regenerate LSA just because a neighbor reached Full state
            // LSAs should only be regenerated when topology actually changes
            // This prevents flooding loops
        }
        
        // Only flood LSAs if we actually updated our database
        if lsas_updated && !neighbor_to_full {
            // Only flood LSAs that were actually updated (not just acknowledged)
            console_log!("Router {} checking which LSAs to flood (except to sender {})", 
                self.router_id, from_router_id);
            
            // Only flood the LSAs that were actually updated in our database
            for key in &updated_lsa_keys {
                if let Some(lsa) = self.lsa_manager.get_lsa_by_key(key) {
                    // Don't flood LSAs that originated from us
                    if lsa.header.advertising_router != self.router_id {
                        console_log!("  Flooding updated LSA {} to other neighbors", key);
                        let flood_events = self.flood_lsa_except(lsa, from_router_id);
                        events.extend(flood_events);
                    } else {
                        console_log!("  Skipping flood of our own LSA");
                    }
                }
            }
        }
        
        events
    }
    
    pub fn process_lsack_packet(&mut self, packet: &LinkStateAcknowledgmentPacket, from_router_id: u32) -> Vec<PacketEvent> {
        console_log!("Router {} received LSAck from router {} with {} headers", 
            self.router_id, from_router_id, packet.lsa_headers.len());
        
        // In a full implementation, we would:
        // 1. Stop retransmission timers for acknowledged LSAs
        // 2. Remove LSAs from retransmission lists
        // 3. Track acknowledgment state
        
        // For now, just log the acknowledgment
        for header in &packet.lsa_headers {
            console_log!("  LSA acknowledged: Type={}, ID={}, AdvRouter={}", 
                header.lsa_type, header.link_state_id, header.advertising_router);
        }
        
        Vec::new()
    }
    
    pub fn generate_hello_packet(&self) -> HelloPacket {
        let active_neighbors = self.neighbor_manager.get_all_active_neighbors();
        self.packet_processor.generate_hello_packet(&active_neighbors)
    }
    
    pub fn add_router_link(&mut self, neighbor_id: u32, interface_id: u32, cost: u32) {
        self.lsa_manager.add_router_link(neighbor_id, interface_id, cost);
        // Don't generate LSA immediately - wait until neighbors are discovered
        console_log!("Router {} added link configuration to neighbor {} (LSA generation deferred)", 
            self.router_id, neighbor_id);
    }
    
    pub fn remove_link(&mut self, neighbor_id: u32) {
        self.lsa_manager.remove_router_link(neighbor_id);
    }
    
    pub fn add_link(&mut self, neighbor_id: u32, interface_id: u32, cost: u32) {
        self.add_router_link(neighbor_id, interface_id, cost);
    }
    
    pub fn remove_neighbor(&mut self, neighbor_id: u32) -> bool {
        let removed = self.neighbor_manager.remove_neighbor(neighbor_id);
        if removed {
            self.timer_manager.clear_all_neighbor_timers(neighbor_id);
        }
        removed
    }
    
    pub fn get_neighbor_count(&self) -> usize {
        self.neighbor_manager.get_neighbor_count()
    }
    
    pub fn get_lsa_count(&self) -> usize {
        self.lsa_manager.get_lsa_count()
    }
    
    pub fn get_lsa_database(&self) -> &HashMap<String, RouterLSA> {
        self.lsa_manager.get_lsa_database()
    }
    
    pub fn get_neighbor_state_transitions(&mut self) -> HashMap<u32, (OSPFNeighborState, OSPFNeighborState)> {
        self.neighbor_manager.get_state_transitions()
    }
    
    pub fn generate_router_lsa(&mut self) -> RouterLSA {
        self.lsa_manager.generate_router_lsa()
    }
    
    pub fn regenerate_router_lsa(&mut self) -> Vec<PacketEvent> {
        let router_lsa = self.lsa_manager.regenerate_router_lsa();
        self.flood_lsa(&router_lsa)
    }
    
    pub fn needs_lsa_regeneration(&self) -> bool {
        self.lsa_manager.needs_lsa_regeneration()
    }
    
    pub fn update_lsa_database(&mut self, lsa: RouterLSA) {
        self.lsa_manager.update_lsa_database(lsa);
    }
    
    pub fn flood_lsa(&self, lsa: &RouterLSA) -> Vec<PacketEvent> {
        let mut events = Vec::new();
        
        // Check if we recently flooded this LSA (MinLSInterval)
        let lsa_key = format!("{}:{}:{}", 
            lsa.header.ls_type.clone() as u8,
            lsa.header.link_state_id,
            lsa.header.advertising_router
        );
        
        if self.lsa_manager.was_recently_updated(&lsa_key, self.current_time) {
            console_log!("Router {} skipping flood of LSA {} due to MinLSInterval", 
                self.router_id, lsa_key);
            return events;
        }
        
        // Get neighbors in Exchange or Full state
        let exchange_neighbors = self.neighbor_manager.get_neighbors_in_state(OSPFNeighborState::Exchange);
        let full_neighbors = self.neighbor_manager.get_neighbors_in_state(OSPFNeighborState::Full);
        
        let mut eligible_neighbors = exchange_neighbors;
        eligible_neighbors.extend(full_neighbors);
        
        console_log!("Router {} flooding LSA {} to {} neighbors", 
            self.router_id, lsa.header.link_state_id, eligible_neighbors.len());
        
        if eligible_neighbors.is_empty() {
            console_log!("Router {} has no eligible neighbors for LSA flooding", self.router_id);
            return events;
        }
        
        // Convert RouterLSA to packet LSA format
        let packet_lsa = crate::ospf::LSA {
            header: crate::ospf::LSAHeader {
                age: lsa.header.ls_age,
                options: 0x02,
                lsa_type: lsa.header.ls_type.clone() as u8,
                link_state_id: lsa.header.link_state_id.clone(),
                advertising_router: lsa.header.advertising_router.clone(),
                sequence_number: lsa.header.ls_sequence_number,
                checksum: lsa.header.ls_checksum,
                length: lsa.header.length,
            },
            data: lsa.data.clone(),
        };
        
        // Create LSU packets for eligible neighbors
        for neighbor_id in eligible_neighbors {
            console_log!("Router {} sending LSU to neighbor {} for LSA {}", 
                self.router_id, neighbor_id, lsa.header.link_state_id);
            
            let lsu_event = self.packet_processor.create_lsu_packet_event(neighbor_id, &[packet_lsa.clone()]);
            events.push(lsu_event);
        }
        
        events
    }
    
    pub fn get_router_links(&self) -> &Vec<(u32, u32, u32)> {
        self.lsa_manager.get_router_links()
    }
    
    pub fn get_area_id(&self) -> &str {
        &self.area_id
    }
    
    pub fn request_spf_calculation(&mut self) {
        // RFC 2328 Section 16.1 - Delay SPF calculation to avoid CPU overload
        if !self.spf_calculation_pending {
            console_log!("Router {} requesting SPF calculation with delay", self.router_id);
            self.spf_calculation_pending = true;
            self.timer_manager.start_spf_delay_timer();
        } else {
            console_log!("Router {} SPF calculation already pending", self.router_id);
        }
    }
    
    pub fn is_spf_pending(&self) -> bool {
        self.spf_calculation_pending
    }
    
    pub fn flood_lsa_except(&self, lsa: &RouterLSA, except_neighbor: u32) -> Vec<PacketEvent> {
        let mut events = Vec::new();
        
        // Get neighbors in Exchange or Full state
        let exchange_neighbors = self.neighbor_manager.get_neighbors_in_state(OSPFNeighborState::Exchange);
        let full_neighbors = self.neighbor_manager.get_neighbors_in_state(OSPFNeighborState::Full);
        
        let mut eligible_neighbors = exchange_neighbors;
        eligible_neighbors.extend(full_neighbors);
        
        // Remove the except_neighbor from the list
        eligible_neighbors.retain(|&id| id != except_neighbor);
        
        console_log!("Router {} flooding LSA {} to {} neighbors (except {})", 
            self.router_id, lsa.header.link_state_id, eligible_neighbors.len(), except_neighbor);
        
        if eligible_neighbors.is_empty() {
            console_log!("Router {} has no eligible neighbors for LSA flooding", self.router_id);
            return events;
        }
        
        // Convert RouterLSA to packet LSA format
        let packet_lsa = crate::ospf::LSA {
            header: crate::ospf::LSAHeader {
                age: lsa.header.ls_age,
                options: 0x02,
                lsa_type: lsa.header.ls_type.clone() as u8,
                link_state_id: lsa.header.link_state_id.clone(),
                advertising_router: lsa.header.advertising_router.clone(),
                sequence_number: lsa.header.ls_sequence_number,
                checksum: lsa.header.ls_checksum,
                length: lsa.header.length,
            },
            data: lsa.data.clone(),
        };
        
        // Create LSU packets for eligible neighbors
        for neighbor_id in eligible_neighbors {
            console_log!("Router {} sending LSU to neighbor {} for LSA {}", 
                self.router_id, neighbor_id, lsa.header.link_state_id);
            
            let lsu_event = self.packet_processor.create_lsu_packet_event(neighbor_id, &[packet_lsa.clone()]);
            events.push(lsu_event);
        }
        
        events
    }
    
    /// Generate Hello packet events for all physically connected neighbors
    fn generate_hello_events(&self) -> Vec<PacketEvent> {
        let mut events = Vec::new();
        
        // Get our router ID as u32
        let our_router_id = self.router_id.split('.').last()
            .unwrap_or("0").parse::<u32>().unwrap_or(0);
        
        // Get all router links (physical connections)
        let router_links = self.lsa_manager.get_router_links();
        
        console_log!("Router {} generating Hello events, has {} physical links", 
            self.router_id, router_links.len());
        
        if router_links.is_empty() {
            console_log!("Router {} has no physical links for Hello packets", self.router_id);
            return events;
        }
        
        // Generate Hello packet with current active neighbor list
        let hello_packet = self.generate_hello_packet();
        
        // Create Hello packet event for each physical neighbor
        for (neighbor_id, _interface_id, _cost) in router_links {
            let packet = ProtocolPacket::OSPF(OSPFPacket {
                version: 2,
                packet_type: OSPFPacketType::Hello,
                router_id: self.router_id.clone(),
                area_id: self.area_id.clone(),
                checksum: 0,
                auth_type: 0,
                authentication: 0,
                data: OSPFPacketData::Hello(hello_packet.clone()),
            });
            
            let event = PacketEvent {
                timestamp: 0.0, // Will be set by caller
                from_router_id: our_router_id,
                to_router_id: *neighbor_id,
                packet,
            };
            
            events.push(event);
        }
        
        console_log!("Router {} generated {} Hello packet events for neighbors {:?}", 
            self.router_id, events.len(), 
            router_links.iter().map(|(id, _, _)| id).collect::<Vec<_>>());
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = OSPFEngine::new("1.1.1.1".to_string(), "0.0.0.0".to_string());
        assert_eq!(engine.get_neighbor_count(), 0);
        assert_eq!(engine.get_lsa_count(), 0);
    }
}
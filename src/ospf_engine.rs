use std::collections::{HashMap, HashSet};
use crate::ospf::{HelloPacket, DatabaseDescriptionPacket, 
    LinkStateRequestPacket, LinkStateUpdatePacket, LinkStateAcknowledgmentPacket,
    OSPFPacket, OSPFPacketType, OSPFPacketData};
use crate::router::{OSPFNeighborState, LSA as RouterLSA, OSPFNeighbor, LSA};
use crate::protocol::{PacketEvent, ProtocolPacket};
use crate::ospf_neighbor::OSPFNeighborManager;
use crate::ospf_lsa_manager::OSPFLSAManager;
use crate::ospf_packet_processor::OSPFPacketProcessor;
use crate::ospf_timer::{OSPFTimerManager, OSPFTimerEvent};
use crate::ospf_checksum::verify_lsa_checksum;
use crate::ospf_dr_election::{DRElectionManager, DRElectionCandidate};
use crate::network_type::OSPFNetworkType;
use crate::network_lsa::NetworkLSAGenerator;
use crate::summary_lsa::SummaryLSAGenerator;
use crate::as_external_lsa::{ASExternalLSAGenerator, ExternalMetricType};
use crate::ospf_auth::AuthConfig;
use crate::ospf_options::OSPFOptions;
use crate::ospf_interface_state::{ExtendedInterfaceState, InterfaceStateManager, OSPFInterfaceState};
use crate::ospf_tos::{TOSCapabilities, TOSValue, TOSMetric, TOSRoutingTable};
use crate::console_log;

/// Refactored OSPF Engine
/// 
/// A simplified OSPF engine that delegates responsibilities to specialized components:
/// - OSPFNeighborManager: Neighbor state management
/// - OSPFLSAManager: LSA database management
/// - OSPFPacketProcessor: Packet processing
/// - OSPFTimerManager: Timer management
/// - DRElectionManager: DR/BDR election management
pub struct OSPFEngine {
    router_id: String,
    area_id: String,
    current_time: f64,
    spf_calculation_pending: bool,  // RFC 2328 Section 16.1 - Track if SPF is scheduled
    
    // SPF calculation parameters (RFC 2328 Section 16.1)
    spf_delay: u16,           // Minimum delay between SPF calculations (seconds)
    spf_holdtime: u16,        // Hold time between consecutive SPF calculations (seconds)
    spf_max_age: u16,         // Maximum age before full SPF recalculation (seconds)
    last_spf_calculation: f64, // Timestamp of last SPF calculation
    
    // Specialized components
    neighbor_manager: OSPFNeighborManager,
    lsa_manager: OSPFLSAManager,
    packet_processor: OSPFPacketProcessor,
    timer_manager: OSPFTimerManager,
    dr_election_managers: HashMap<u32, DRElectionManager>,  // Per-interface DR election
    network_lsa_generator: NetworkLSAGenerator,
    summary_lsa_generator: SummaryLSAGenerator,
    as_external_lsa_generator: ASExternalLSAGenerator,
    
    // Track interface states for Network LSA generation
    interface_states: HashMap<u32, InterfaceState>,
    
    // Extended interface state management
    interface_state_manager: InterfaceStateManager,
    
    // Track area membership for ABR functionality
    connected_areas: HashSet<String>,
    
    // Track external routes for ASBR functionality
    external_routes: Vec<(String, String, u32, ExternalMetricType, String, u32)>,
    
    // OSPF Options configuration
    area_options: OSPFOptions,  // Options for this area
    
    // TOS support
    tos_capabilities: TOSCapabilities,
    tos_routing_table: TOSRoutingTable,
}

#[derive(Debug, Clone)]
struct InterfaceState {
    is_dr: bool,
    interface_ip: String,
    network_mask: String,
    fully_adjacent_neighbors: HashSet<String>,
}

impl OSPFEngine {
    pub fn new(router_id: String, area_id: String) -> Self {
        let mut timer_manager = OSPFTimerManager::new(router_id.clone());
        timer_manager.start_hello_timer();  // Start hello timer immediately
        
        let mut connected_areas = HashSet::new();
        connected_areas.insert(area_id.clone());
        
        OSPFEngine {
            neighbor_manager: OSPFNeighborManager::new(40), // 40s dead interval
            lsa_manager: OSPFLSAManager::new(router_id.clone()),
            packet_processor: OSPFPacketProcessor::new(router_id.clone(), area_id.clone()),
            timer_manager,
            network_lsa_generator: NetworkLSAGenerator::new(router_id.clone()),
            summary_lsa_generator: SummaryLSAGenerator::new(router_id.clone()),
            as_external_lsa_generator: ASExternalLSAGenerator::new(router_id.clone()),
            router_id,
            area_id,
            current_time: 0.0,
            spf_calculation_pending: false,
            // SPF calculation parameters (RFC 2328 defaults)
            spf_delay: 5,              // 5 seconds initial delay
            spf_holdtime: 10,          // 10 seconds hold time
            spf_max_age: 300,          // 5 minutes max age
            last_spf_calculation: 0.0,
            dr_election_managers: HashMap::new(),
            interface_states: HashMap::new(),
            interface_state_manager: InterfaceStateManager::new(),
            connected_areas,
            external_routes: Vec::new(),
            area_options: OSPFOptions::standard_area_options(),
            tos_capabilities: TOSCapabilities::new(),
            tos_routing_table: TOSRoutingTable::new(),
        }
    }
    
    pub fn update_time(&mut self, time: f64) -> Vec<PacketEvent> {
        let time_delta = if self.current_time > 0.0 { time - self.current_time } else { 0.0 };
        
        // Log significant time jumps (more than 1 second)
        if time_delta > 1.0 {
            console_log!("Router {} large time jump detected: {:.2}s -> {:.2}s (delta: {:.2}s)", 
                self.router_id, self.current_time, time, time_delta);
        }
        
        self.current_time = time;
        self.interface_state_manager.update_time(time);
        
        let dead_neighbors = self.neighbor_manager.update_time(time);
        // Clean up DD state for neighbors that went down
        for neighbor_id in dead_neighbors {
            self.packet_processor.cleanup_neighbor_dd_state(neighbor_id);
        }
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
                    // Clean up DD exchange state to prevent sequence number persistence
                    self.packet_processor.cleanup_neighbor_dd_state(neighbor_id);
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
                    
                    // Check if neighbor is in Full state - if so, ignore DD retransmission
                    if let Some(neighbor_state) = self.neighbor_manager.get_neighbor_state(neighbor_id) {
                        if neighbor_state == OSPFNeighborState::Full {
                            console_log!("Router {} ignoring DD retransmission for neighbor {} in Full state", 
                                self.router_id, neighbor_id);
                            // Stop the timer to prevent further retransmissions
                            self.timer_manager.stop_dd_retransmission_timer(neighbor_id);
                            continue;
                        }
                    }
                    
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
                    console_log!("Router {} SPF delay timer expired at {:.2}s, calculation can proceed", 
                        self.router_id, self.current_time);
                    self.spf_calculation_pending = false;
                    // The actual SPF calculation will be triggered by the simulation layer
                    // Mark that SPF is ready to run by ensuring database_updated flag is set
                    if self.lsa_manager.was_database_updated() {
                        console_log!("Router {} SPF calculation ready - database was updated", self.router_id);
                    } else {
                        console_log!("Router {} SPF timer expired but database was not updated", self.router_id);
                    }
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
                    
                    // Check for TwoWay state to trigger DR election
                    if state_changed {
                        if let Some(OSPFNeighborState::TwoWay) = self.neighbor_manager.get_neighbor_state(from_router_id) {
                            console_log!("Router {} neighbor {} reached TwoWay state, checking DR election", 
                                self.router_id, from_router_id);
                            
                            // Run DR election for this interface if required
                            if let Some(dr_manager) = self.dr_election_managers.get(&interface_id) {
                                if dr_manager.is_election_required() {
                                    console_log!("Router {} running DR election on interface {} due to TwoWay state", 
                                        self.router_id, interface_id);
                                    
                                    // Collect Hello packets from neighbors on this interface
                                    let mut interface_neighbors = Vec::new();
                                    interface_neighbors.push((from_router_id, packet.clone()));
                                    
                                    // Run DR election with collected neighbors
                                    let election_changed = self.run_dr_election(interface_id, interface_neighbors);
                                    
                                    if election_changed {
                                        console_log!("Router {} DR election changed on interface {} in TwoWay state", 
                                            self.router_id, interface_id);
                                        
                                        // Generate or update Network LSA if we are DR
                                        if let Some(network_lsa) = self.update_dr_status(interface_id) {
                                            console_log!("Router {} generated/updated Network LSA after DR election", 
                                                self.router_id);
                                            // The LSA is already added to database, will be flooded in next update
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    // Check if should form adjacency
                    if state_changed && self.neighbor_manager.should_form_adjacency(from_router_id) {
                        console_log!("Router {} neighbor {} is ready for adjacency formation", 
                            self.router_id, from_router_id);
                        if self.neighbor_manager.start_adjacency(from_router_id) {
                            console_log!("Router {} neighbor {} moved to ExStart", 
                                self.router_id, from_router_id);
                            
                            // Generate initial LSA if we don't have one yet
                            if self.lsa_manager.get_lsa_count() == 0 && self.lsa_manager.get_router_links().len() > 0 {
                                console_log!("Router {} generating initial Router LSA before DD exchange", self.router_id);
                                let _lsa = self.lsa_manager.generate_router_lsa();
                                console_log!("Router {} generated Router LSA with {} links, database now has {} LSAs", 
                                    self.router_id, self.lsa_manager.get_router_links().len(), self.lsa_manager.get_lsa_count());
                            }
                            
                            // Send Database Description packet
                            let interface_id = self.get_neighbor_interface(from_router_id);
                            let dd_event = self.packet_processor.create_dd_packet_event(
                                from_router_id, 
                                interface_id,
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
                        
                        // Stop DD retransmission timer when reaching Full state
                        self.timer_manager.stop_dd_retransmission_timer(from_router_id);
                        console_log!("Router {} stopped DD retransmission timer for neighbor {} (Full state)", 
                            self.router_id, from_router_id);
                        
                        // When neighbor reaches Full state from Exchange
                        let full_neighbor_count = self.neighbor_manager.get_neighbors_in_state(OSPFNeighborState::Full).len();
                        console_log!("Router {} now has {} Full neighbors", self.router_id, full_neighbor_count);
                        
                        // Update Network LSA if we are DR on any interface
                        let neighbors = self.neighbor_manager.get_all_neighbors();
                        if let Some(neighbor) = neighbors.iter().find(|n| n.router_id == format!("1.1.1.{}", from_router_id)) {
                            let interface_id = neighbor.interface_id;
                            if let Some(_network_lsa) = self.update_dr_status(interface_id) {
                                console_log!("Router {} updated Network LSA after neighbor {} reached Full state", 
                                    self.router_id, from_router_id);
                                // The LSA is already added to database, will be flooded in next update
                            }
                        }
                        
                        // DD exchange and LSR/LSU process should have already synchronized LSAs
                        // No need to flood all LSAs again
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
                let interface_id = self.get_neighbor_interface(from_router_id);
                let dd_event = self.packet_processor.create_dd_packet_event(
                    from_router_id, 
                    interface_id,
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
            let interface_id = self.get_neighbor_interface(from_router_id);
            let lsu_event = self.packet_processor.create_lsu_packet_event(from_router_id, interface_id, &lsas_to_send);
            events.push(lsu_event);
        } else {
            console_log!("Router {} has no LSAs to send in response to LSR from router {}", 
                self.router_id, from_router_id);
        }
        
        events
    }
    
    pub fn process_lsu_packet(&mut self, packet: &LinkStateUpdatePacket, from_router_id: u32) -> Vec<PacketEvent> {
        let mut events = Vec::new();
        
        console_log!("Router {} processing LSU from {} with {} LSAs", 
            self.router_id, from_router_id, packet.lsas.len());
        
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
            
            let should_update = self.lsa_manager.should_update_lsa(lsa);
            console_log!("Router {} checking LSA from {}: type={}, should_update={}", 
                self.router_id, from_router_id, lsa.header.ls_type.clone() as u8, should_update);
            
            if should_update {
                let key = format!("{}:{}:{}", 
                    lsa.header.ls_type.clone() as u8,
                    lsa.header.link_state_id,
                    lsa.header.advertising_router
                );
                
                // Always update the database if should_update is true
                // MinLSInterval should only prevent flooding, not database updates
                console_log!("Router {} updating LSA: {}", self.router_id, key);
                self.lsa_manager.update_lsa_database(lsa.clone());
                
                // Add to flood list
                updated_lsa_keys.push(key.clone());
                console_log!("Router {} will flood LSA {} to neighbors", self.router_id, key);
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
            
            // Stop DD retransmission timer when reaching Full state
            self.timer_manager.stop_dd_retransmission_timer(from_router_id);
            console_log!("Router {} stopped DD retransmission timer for neighbor {} (Full state via LSU)", 
                self.router_id, from_router_id);
            
            // Update Network LSA if we are DR on any interface
            let neighbors = self.neighbor_manager.get_all_neighbors();
            if let Some(neighbor) = neighbors.iter().find(|n| n.router_id == format!("1.1.1.{}", from_router_id)) {
                let interface_id = neighbor.interface_id;
                if let Some(_network_lsa) = self.update_dr_status(interface_id) {
                    console_log!("Router {} updated Network LSA after neighbor {} reached Full state via LSU", 
                        self.router_id, from_router_id);
                    // The LSA is already added to database, will be flooded in next update
                }
            }
            
            // DD exchange and LSR/LSU process should have already synchronized LSAs
            // No need to flood all LSAs again when reaching Full state
        }
        
        // Always flood LSAs if we actually updated our database
        // RFC 2328 Section 13: LSAs must be flooded regardless of neighbor state transitions
        console_log!("Router {} LSU processing result: lsas_updated={}, updated_count={}", 
            self.router_id, lsas_updated, updated_lsa_keys.len());
        
        if lsas_updated {
            // Only flood LSAs that were actually updated (not just acknowledged)
            console_log!("Router {} checking which LSAs to flood (except to sender {})", 
                self.router_id, from_router_id);
            
            // Flood all LSAs that were updated and not recently flooded
            // RFC 2328: Even self-originated LSAs should be flooded when received from others
            let lsas_to_flood: Vec<(String, RouterLSA)> = updated_lsa_keys.iter()
                .filter_map(|key| {
                    self.lsa_manager.get_lsa_by_key(key)
                        .map(|lsa| (key.clone(), lsa.clone()))
                })
                .collect();
            
            for (key, lsa) in lsas_to_flood {
                console_log!("  Router {} flooding updated LSA {} to other neighbors (except {})", 
                    self.router_id, key, from_router_id);
                let flood_events = self.flood_lsa_except(&lsa, from_router_id);
                console_log!("  Router {} generated {} flood events for LSA {}", 
                    self.router_id, flood_events.len(), key);
                // Do not mark as flooded here - let the flooding complete first
                events.extend(flood_events);
            }
            
            // Request SPF calculation when LSA database changes (RFC 2328 Section 16.1)
            console_log!("Router {} checking SPF: pending={}, database_updated={}", 
                self.router_id, self.spf_calculation_pending, self.lsa_manager.was_database_updated());
            
            if !self.spf_calculation_pending {
                console_log!("Router {} requesting SPF calculation after LSU updated database", self.router_id);
                self.request_spf_calculation();
            } else {
                console_log!("Router {} SPF already pending, not requesting again", self.router_id);
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
        // RFC 2328 Section 9.5: The neighbor field lists all routers from which
        // valid Hello packets have been seen recently on the network segment
        let active_neighbors = self.neighbor_manager.get_all_active_neighbors();
        console_log!("Router {} generating Hello packet with {} active neighbors: {:?}", 
            self.router_id, active_neighbors.len(), active_neighbors);
        // For compatibility, return default DR/BDR values when interface is not specified
        // Use broadcast network mask as default
        self.packet_processor.generate_hello_packet(&active_neighbors, "0.0.0.0".to_string(), "0.0.0.0".to_string(), "255.255.255.0".to_string())
    }
    
    pub fn add_router_link(&mut self, neighbor_id: u32, interface_id: u32, cost: u32) {
        self.lsa_manager.add_router_link(neighbor_id, interface_id, cost);
        // Don't generate LSA immediately - wait until neighbors are discovered
        console_log!("Router {} added link configuration to neighbor {} (LSA generation deferred)", 
            self.router_id, neighbor_id);
    }
    
    pub fn remove_link(&mut self, neighbor_id: u32) -> Vec<PacketEvent> {
        console_log!("Router {} remove_link called for neighbor {}", self.router_id, neighbor_id);
        self.lsa_manager.remove_router_link(neighbor_id);
        
        // RFC 2328 Section 13.2: Generate new Router-LSA when link state changes
        // LSA must be regenerated whenever topology changes, regardless of neighbor count
        console_log!("Router {} regenerating LSA after removing link to {}", 
            self.router_id, neighbor_id);
        let events = self.regenerate_router_lsa();
        console_log!("Router {} regenerate_router_lsa returned {} events", self.router_id, events.len());
        
        // Request SPF calculation (RFC 2328 Section 16.1)
        self.request_spf_calculation();
        console_log!("Router {} requested SPF calculation after link removal", self.router_id);
        
        // If we have neighbors to flood to, return the events
        let neighbor_count = self.get_neighbor_count();
        console_log!("Router {} has {} neighbors remaining", self.router_id, neighbor_count);
        
        if neighbor_count > 0 {
            events
        } else {
            console_log!("Router {} has no neighbors to flood LSA to", self.router_id);
            Vec::new()
        }
    }
    
    pub fn add_link(&mut self, neighbor_id: u32, interface_id: u32, cost: u32) {
        self.add_router_link(neighbor_id, interface_id, cost);
    }
    
    pub fn remove_neighbor(&mut self, neighbor_id: u32) -> bool {
        console_log!("Router {} remove_neighbor called for neighbor {}", self.router_id, neighbor_id);
        let removed = self.neighbor_manager.remove_neighbor(neighbor_id);
        console_log!("Router {} neighbor_manager.remove_neighbor({}) returned: {}", self.router_id, neighbor_id, removed);
        if removed {
            self.timer_manager.clear_all_neighbor_timers(neighbor_id);
            // Clean up DD exchange state to prevent sequence number persistence
            self.packet_processor.cleanup_neighbor_dd_state(neighbor_id);
            console_log!("Router {} cleaned up timers and DD state for neighbor {}", self.router_id, neighbor_id);
        }
        
        // Log current neighbor state
        let active_neighbors = self.neighbor_manager.get_all_active_neighbors();
        console_log!("Router {} active neighbors after removal: {:?}", self.router_id, active_neighbors);
        
        removed
    }
    
    pub fn get_neighbor_count(&self) -> usize {
        self.neighbor_manager.get_neighbor_count()
    }
    
    pub fn get_neighbor_state(&self, neighbor_id: u32) -> Option<OSPFNeighborState> {
        self.neighbor_manager.get_neighbor_state(neighbor_id)
    }
    
    pub fn get_lsa_count(&self) -> usize {
        self.lsa_manager.get_lsa_count()
    }
    
    pub fn get_neighbors(&self) -> &HashMap<u32, OSPFNeighbor> {
        self.neighbor_manager.get_neighbors()
    }
    
    pub fn get_lsa_database(&self) -> &HashMap<String, RouterLSA> {
        self.lsa_manager.get_lsa_database()
    }
    
    pub fn get_neighbor_state_transitions(&mut self) -> HashMap<u32, (OSPFNeighborState, OSPFNeighborState)> {
        self.neighbor_manager.get_state_transitions()
    }
    
    pub fn clean_unreachable_lsas(&mut self, reachable_routers: &std::collections::HashSet<u32>) {
        self.lsa_manager.remove_unreachable_lsas(reachable_routers);
    }
    
    pub fn generate_router_lsa(&mut self) -> RouterLSA {
        self.lsa_manager.generate_router_lsa()
    }
    
    pub fn regenerate_router_lsa(&mut self) -> Vec<PacketEvent> {
        console_log!("Router {} regenerate_router_lsa called", self.router_id);
        let router_lsa = self.lsa_manager.regenerate_router_lsa();
        
        // Count links in the LSA
        let link_count = match &router_lsa.data {
            crate::router::LSAData::Router(data) => data.links.len(),
            _ => 0,
        };
        console_log!("Router {} LSA regenerated with {} links", 
            self.router_id, link_count);
        
        // Request SPF calculation after LSA regeneration
        self.request_spf_calculation();
        console_log!("Router {} requested SPF calculation after LSA regeneration", self.router_id);
        
        let events = self.flood_lsa(&router_lsa);
        console_log!("Router {} flood_lsa returned {} events", self.router_id, events.len());
        events
    }
    
    pub fn needs_lsa_regeneration(&self) -> bool {
        self.lsa_manager.needs_lsa_regeneration()
    }
    
    pub fn update_lsa_database(&mut self, lsa: RouterLSA) {
        self.lsa_manager.update_lsa_database(lsa);
    }
    
    pub fn flood_lsa(&mut self, lsa: &RouterLSA) -> Vec<PacketEvent> {
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
                options: OSPFOptions::standard_area_options(),
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
            
            let interface_id = self.get_neighbor_interface(neighbor_id);
            let lsu_event = self.packet_processor.create_lsu_packet_event(neighbor_id, interface_id, &[packet_lsa.clone()]);
            events.push(lsu_event);
        }
        
        // Mark LSA as flooded to prevent flooding loops
        if !events.is_empty() {
            self.lsa_manager.mark_lsa_flooded(&lsa_key);
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
            // Calculate delay based on time since last SPF calculation
            let time_since_last_spf = if self.last_spf_calculation > 0.0 {
                self.current_time - self.last_spf_calculation
            } else {
                f64::INFINITY
            };
            
            let delay = if time_since_last_spf < (self.spf_holdtime as f64) {
                // Recent SPF calculation - use hold time
                self.spf_holdtime
            } else {
                // Normal case - use initial delay
                self.spf_delay
            };
            
            console_log!("Router {} requesting SPF calculation with {}s delay (last SPF: {:.2}s ago)", 
                self.router_id, delay, time_since_last_spf);
            
            self.spf_calculation_pending = true;
            self.timer_manager.start_spf_delay_timer();
        } else {
            console_log!("Router {} SPF calculation already pending", self.router_id);
        }
    }
    
    pub fn is_spf_pending(&self) -> bool {
        self.spf_calculation_pending
    }
    
    pub fn was_lsa_database_updated(&self) -> bool {
        // This is tracked by the lsa_manager
        self.lsa_manager.was_database_updated()
    }
    
    /// Mark SPF calculation as completed
    pub fn mark_spf_calculation_completed(&mut self) {
        self.spf_calculation_pending = false;
        self.last_spf_calculation = self.current_time;
        console_log!("Router {} marked SPF calculation completed at {:.2}s", 
            self.router_id, self.current_time);
    }
    
    /// Update SPF calculation parameters
    pub fn update_spf_parameters(&mut self, spf_delay: u16, spf_holdtime: u16, spf_max_age: u16) {
        self.spf_delay = spf_delay;
        self.spf_holdtime = spf_holdtime;
        self.spf_max_age = spf_max_age;
        console_log!("Router {} SPF parameters updated: delay={}s, holdtime={}s, max_age={}s", 
            self.router_id, spf_delay, spf_holdtime, spf_max_age);
    }
    
    /// Get SPF calculation parameters
    pub fn get_spf_parameters(&self) -> (u16, u16, u16) {
        (self.spf_delay, self.spf_holdtime, self.spf_max_age)
    }
    
    /// Update area options configuration
    pub fn update_area_options(&mut self, options: OSPFOptions) {
        self.area_options = options;
        console_log!("Router {} updated area options: {}", self.router_id, options.to_string());
    }
    
    /// Get current area options
    pub fn get_area_options(&self) -> OSPFOptions {
        self.area_options
    }
    
    /// Configure area as stub area
    pub fn configure_stub_area(&mut self) {
        self.area_options = OSPFOptions::stub_area_options();
        console_log!("Router {} configured as stub area", self.router_id);
    }
    
    /// Configure area as NSSA area
    pub fn configure_nssa_area(&mut self) {
        self.area_options = OSPFOptions::nssa_area_options();
        console_log!("Router {} configured as NSSA area", self.router_id);
    }
    
    /// Check if router supports multicast capabilities
    pub fn supports_multicast(&self) -> bool {
        self.area_options.get_mc_bit()
    }
    
    /// Enable or disable multicast support
    pub fn set_multicast_support(&mut self, enabled: bool) {
        self.area_options.set_mc_bit(enabled);
        console_log!("Router {} multicast support: {}", self.router_id, enabled);
    }
    
    /// Check if router supports demand circuits
    pub fn supports_demand_circuits(&self) -> bool {
        self.area_options.get_dc_bit()
    }
    
    /// Enable or disable demand circuit support
    pub fn set_demand_circuit_support(&mut self, enabled: bool) {
        self.area_options.set_dc_bit(enabled);
        console_log!("Router {} demand circuit support: {}", self.router_id, enabled);
    }
    
    /// Check if router supports Opaque LSAs
    pub fn supports_opaque_lsa(&self) -> bool {
        self.area_options.get_o_bit()
    }
    
    /// Enable or disable Opaque LSA support
    pub fn set_opaque_lsa_support(&mut self, enabled: bool) {
        self.area_options.set_o_bit(enabled);
        console_log!("Router {} Opaque LSA support: {}", self.router_id, enabled);
    }
    
    /// Get extended interface state
    pub fn get_extended_interface_state(&self, interface_id: u32) -> Option<&ExtendedInterfaceState> {
        self.interface_state_manager.get_interface(interface_id)
    }
    
    /// Get mutable extended interface state
    pub fn get_extended_interface_state_mut(&mut self, interface_id: u32) -> Option<&mut ExtendedInterfaceState> {
        self.interface_state_manager.get_interface_mut(interface_id)
    }
    
    /// Transition interface to new state
    pub fn transition_interface_state(&mut self, interface_id: u32, new_state: OSPFInterfaceState) {
        if let Some(interface_state) = self.interface_state_manager.get_interface_mut(interface_id) {
            let old_state = interface_state.state;
            interface_state.transition_to_state(new_state, self.current_time);
            
            console_log!("Router {} interface {} transitioned from {} to {}", 
                self.router_id, interface_id, 
                old_state as u8, new_state as u8);
            
            // Update legacy interface state for Network LSA generation
            if let Some(legacy_state) = self.interface_states.get_mut(&interface_id) {
                legacy_state.is_dr = matches!(new_state, OSPFInterfaceState::DR);
            }
        }
    }
    
    /// Update interface with neighbor information
    pub fn update_interface_neighbor(&mut self, interface_id: u32, neighbor_id: String, is_full: bool) {
        if let Some(interface_state) = self.interface_state_manager.get_interface_mut(interface_id) {
            interface_state.add_neighbor(neighbor_id.clone());
            
            if is_full {
                interface_state.mark_neighbor_full(neighbor_id.clone());
            }
            
            // Update legacy interface state
            if let Some(legacy_state) = self.interface_states.get_mut(&interface_id) {
                if is_full {
                    legacy_state.fully_adjacent_neighbors.insert(neighbor_id);
                }
            }
        }
    }
    
    /// Remove neighbor from interface
    pub fn remove_interface_neighbor(&mut self, interface_id: u32, neighbor_id: &str) {
        if let Some(interface_state) = self.interface_state_manager.get_interface_mut(interface_id) {
            interface_state.remove_neighbor(neighbor_id);
            
            // Update legacy interface state
            if let Some(legacy_state) = self.interface_states.get_mut(&interface_id) {
                legacy_state.fully_adjacent_neighbors.remove(neighbor_id);
            }
        }
    }
    
    /// Get interface state summary
    pub fn get_interface_state_summary(&self, interface_id: u32) -> Option<String> {
        self.interface_state_manager.get_interface(interface_id).map(|state| state.get_summary())
    }
    
    /// Get all interface states
    pub fn get_all_interface_states(&self) -> Vec<(u32, String)> {
        self.interface_state_manager
            .get_all_interfaces()
            .iter()
            .map(|(id, state)| (*id, state.get_summary()))
            .collect()
    }
    
    /// Get DR interfaces
    pub fn get_dr_interfaces(&self) -> Vec<u32> {
        self.interface_state_manager
            .get_dr_interfaces()
            .iter()
            .map(|(id, _)| *id)
            .collect()
    }
    
    /// Get BDR interfaces
    pub fn get_bdr_interfaces(&self) -> Vec<u32> {
        self.interface_state_manager
            .get_bdr_interfaces()
            .iter()
            .map(|(id, _)| *id)
            .collect()
    }
    
    /// Get total adjacency count across all interfaces
    pub fn get_total_adjacency_count(&self) -> usize {
        self.interface_state_manager.get_total_adjacency_count()
    }
    
    /// Check for expired wait timers and return interfaces ready for DR election
    pub fn check_wait_timers(&mut self) -> Vec<u32> {
        self.interface_state_manager.check_wait_timers()
    }
    
    /// Start wait timer for interface
    pub fn start_interface_wait_timer(&mut self, interface_id: u32) {
        if let Some(interface_state) = self.interface_state_manager.get_interface_mut(interface_id) {
            interface_state.start_wait_timer(self.current_time);
            console_log!("Router {} started wait timer for interface {}", self.router_id, interface_id);
        }
    }
    
    /// Update interface DR/BDR information
    pub fn update_interface_dr_bdr(&mut self, interface_id: u32, dr_ip: String, bdr_ip: String) {
        if let Some(interface_state) = self.interface_state_manager.get_interface_mut(interface_id) {
            interface_state.update_dr_bdr(dr_ip, bdr_ip);
        }
    }
    
    /// Set interface as passive
    pub fn set_interface_passive(&mut self, interface_id: u32, is_passive: bool) {
        if let Some(interface_state) = self.interface_state_manager.get_interface_mut(interface_id) {
            interface_state.set_passive(is_passive);
            console_log!("Router {} set interface {} passive: {}", self.router_id, interface_id, is_passive);
        }
    }
    
    /// Set interface as stub
    pub fn set_interface_stub(&mut self, interface_id: u32, is_stub: bool) {
        if let Some(interface_state) = self.interface_state_manager.get_interface_mut(interface_id) {
            interface_state.set_stub(is_stub);
            console_log!("Router {} set interface {} stub: {}", self.router_id, interface_id, is_stub);
        }
    }
    
    // TOS Support Methods
    
    /// Enable TOS support for this router
    pub fn enable_tos_support(&mut self) {
        self.tos_capabilities.enable_tos_support();
        // Set T-bit in options
        self.area_options.set_t_bit(true);
        console_log!("Router {} enabled TOS support", self.router_id);
    }
    
    /// Disable TOS support
    pub fn disable_tos_support(&mut self) {
        self.tos_capabilities.disable_tos_support();
        self.area_options.set_t_bit(false);
        self.tos_routing_table.clear();
        console_log!("Router {} disabled TOS support", self.router_id);
    }
    
    /// Check if TOS support is enabled
    pub fn is_tos_enabled(&self) -> bool {
        self.tos_capabilities.tos_support_enabled
    }
    
    /// Add a supported TOS value
    pub fn add_supported_tos(&mut self, tos: TOSValue) {
        self.tos_capabilities.add_supported_tos(tos);
        console_log!("Router {} added support for TOS {}", self.router_id, tos.value());
    }
    
    /// Remove a supported TOS value
    pub fn remove_supported_tos(&mut self, tos: TOSValue) {
        self.tos_capabilities.remove_supported_tos(tos);
        console_log!("Router {} removed support for TOS {}", self.router_id, tos.value());
    }
    
    /// Set TOS metrics for an interface
    pub fn set_interface_tos_metrics(&mut self, interface_id: u32, metrics: Vec<TOSMetric>) {
        self.tos_capabilities.set_interface_tos_metrics(interface_id, metrics.clone());
        console_log!("Router {} set {} TOS metrics for interface {}", 
            self.router_id, metrics.len(), interface_id);
    }
    
    /// Get TOS metric for a specific interface and TOS
    pub fn get_interface_tos_metric(&self, interface_id: u32, tos: &TOSValue) -> Option<u32> {
        self.tos_capabilities.get_interface_tos_metric(interface_id, tos)
    }
    
    /// Get all TOS metrics for an interface
    pub fn get_interface_all_tos_metrics(&self, interface_id: u32) -> Vec<TOSMetric> {
        self.tos_capabilities.get_interface_all_tos_metrics(interface_id)
    }
    
    /// Get TOS routing table
    pub fn get_tos_routing_table(&self) -> &TOSRoutingTable {
        &self.tos_routing_table
    }
    
    /// Get mutable TOS routing table
    pub fn get_tos_routing_table_mut(&mut self) -> &mut TOSRoutingTable {
        &mut self.tos_routing_table
    }
    
    /// Get TOS capabilities
    pub fn get_tos_capabilities(&self) -> &TOSCapabilities {
        &self.tos_capabilities
    }
    
    /// Check if a specific TOS is supported
    pub fn is_tos_supported(&self, tos: &TOSValue) -> bool {
        self.tos_capabilities.is_tos_supported(tos)
    }
    
    /// Get supported TOS values
    pub fn get_supported_tos_values(&self) -> Vec<TOSValue> {
        self.tos_capabilities.supported_tos_values.clone()
    }
    
    pub fn reset_database_updated_flag(&mut self) {
        // Reset the flag after SPF calculation
        self.lsa_manager.reset_database_updated();
    }
    
    
    pub fn flood_lsa_except(&mut self, lsa: &RouterLSA, except_neighbor: u32) -> Vec<PacketEvent> {
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
        console_log!("  Eligible neighbors: {:?}", eligible_neighbors);
        
        if eligible_neighbors.is_empty() {
            console_log!("Router {} has no eligible neighbors for LSA flooding", self.router_id);
            return events;
        }
        
        // Convert RouterLSA to packet LSA format
        let packet_lsa = crate::ospf::LSA {
            header: crate::ospf::LSAHeader {
                age: lsa.header.ls_age,
                options: OSPFOptions::standard_area_options(),
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
            
            let interface_id = self.get_neighbor_interface(neighbor_id);
            let lsu_event = self.packet_processor.create_lsu_packet_event(neighbor_id, interface_id, &[packet_lsa.clone()]);
            events.push(lsu_event);
        }
        
        // Mark LSA as flooded to prevent flooding loops
        if !events.is_empty() {
            let lsa_key = format!("{}:{}:{}", 
                lsa.header.ls_type.clone() as u8,
                lsa.header.link_state_id,
                lsa.header.advertising_router
            );
            self.lsa_manager.mark_lsa_flooded(&lsa_key);
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
        
        // Get active neighbors for Hello packet
        let active_neighbors = self.neighbor_manager.get_all_active_neighbors();
        
        // Create Hello packet event for each physical neighbor
        for (neighbor_id, interface_id, _cost) in router_links {
            // Get DR/BDR for this interface
            let (dr, bdr) = self.get_interface_dr_bdr(*interface_id);
            
            // Get network mask for this interface
            let network_mask = if let Some(dr_manager) = self.dr_election_managers.get(interface_id) {
                // Get mask based on network type
                match dr_manager.get_network_type() {
                    OSPFNetworkType::PointToPoint => "255.255.255.252",
                    OSPFNetworkType::Broadcast => "255.255.255.0",
                    OSPFNetworkType::NBMA => "255.255.255.0",
                    OSPFNetworkType::PointToMultipoint => "255.255.255.0",
                }.to_string()
            } else {
                "255.255.255.0".to_string() // Default broadcast mask
            };
            
            // Generate interface-specific Hello packet
            let hello_packet = self.packet_processor.generate_hello_packet(&active_neighbors, dr, bdr, network_mask);
            
            // Get authentication data for this interface
            let (auth_type, auth_data) = self.packet_processor.get_auth_data(*interface_id);
            
            let packet = ProtocolPacket::OSPF(OSPFPacket {
                version: 2,
                packet_type: OSPFPacketType::Hello,
                router_id: self.router_id.clone(),
                area_id: self.area_id.clone(),
                checksum: 0,
                auth_type,
                auth_data,
                data: OSPFPacketData::Hello(hello_packet),
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
    
    /// Initialize DR election manager for an interface
    pub fn initialize_interface_dr_election(&mut self, interface_id: u32, network_type: OSPFNetworkType, priority: u8) {
        if !self.dr_election_managers.contains_key(&interface_id) {
            let dr_manager = DRElectionManager::new(self.router_id.clone(), priority, network_type);
            self.dr_election_managers.insert(interface_id, dr_manager);
            console_log!("Router {} initialized DR election for interface {} with network type {:?}", 
                self.router_id, interface_id, network_type);
        }
    }
    
    /// Run DR/BDR election for an interface
    pub fn run_dr_election(&mut self, interface_id: u32, neighbors: Vec<(u32, HelloPacket)>) -> bool {
        if let Some(dr_manager) = self.dr_election_managers.get_mut(&interface_id) {
            if !dr_manager.is_election_required() {
                return false;
            }
            
            // Build candidate list including ourselves
            let mut candidates = vec![
                DRElectionCandidate {
                    router_id: self.router_id.clone(),
                    router_priority: dr_manager.get_priority(),
                    current_dr: dr_manager.get_dr().to_string(),
                    current_bdr: dr_manager.get_bdr().to_string(),
                    interface_ip: format!("10.0.{}.1", interface_id), // Simplified IP
                }
            ];
            
            // Add neighbors to candidate list
            for (neighbor_id, hello) in neighbors {
                candidates.push(DRElectionCandidate {
                    router_id: format!("1.1.1.{}", neighbor_id),
                    router_priority: hello.router_priority,
                    current_dr: hello.designated_router.clone(),
                    current_bdr: hello.backup_designated_router.clone(),
                    interface_ip: format!("10.0.{}.{}", interface_id, neighbor_id),
                });
            }
            
            let (changed, new_dr, new_bdr) = dr_manager.run_election(candidates);
            
            if changed {
                console_log!("Router {} DR election changed on interface {}: DR={}, BDR={}", 
                    self.router_id, interface_id, new_dr, new_bdr);
                
                // If we became DR or BDR, regenerate LSAs immediately
                if dr_manager.is_dr() || dr_manager.is_bdr() {
                    console_log!("Router {} became DR/BDR on interface {}, LSA regeneration needed", 
                        self.router_id, interface_id);
                    // LSA regeneration will be handled by the caller after DR election
                }
            }
            
            changed
        } else {
            false
        }
    }
    
    /// Get DR/BDR for an interface
    pub fn get_interface_dr_bdr(&self, interface_id: u32) -> (String, String) {
        if let Some(dr_manager) = self.dr_election_managers.get(&interface_id) {
            (dr_manager.get_dr().to_string(), dr_manager.get_bdr().to_string())
        } else {
            ("0.0.0.0".to_string(), "0.0.0.0".to_string())
        }
    }
    
    /// Get all interface IDs with DR election
    pub fn get_dr_election_interfaces(&self) -> Vec<u32> {
        self.dr_election_managers.keys().cloned().collect()
    }

    /// Update interface timers
    pub fn update_interface_timers(&mut self, interface_id: u32, hello_interval: u16, dead_interval: u16) {
        // OSPFタイマーマネージャーにインターフェース固有のタイマー設定を保存
        self.timer_manager.update_interface_timers(interface_id, hello_interval, dead_interval);
        
        // 既存のネイバーのDead intervalも更新
        self.neighbor_manager.update_interface_dead_interval(interface_id, dead_interval);
    }
    
    /// Update interface OSPF parameters including RFC 2328 parameters
    pub fn update_interface_ospf_params(&mut self, interface_id: u32, hello_interval: u16, dead_interval: u16, 
                                       inf_trans_delay: u16, rxmt_interval: u16) {
        // Update timers
        self.update_interface_timers(interface_id, hello_interval, dead_interval);
        
        // Store interface-specific OSPF parameters
        // These will be used for LSA age calculation and retransmission
        console_log!("Router {} updated interface {} OSPF parameters: hello={}s, dead={}s, inf_trans_delay={}s, rxmt_interval={}s",
            self.router_id, interface_id, hello_interval, dead_interval, inf_trans_delay, rxmt_interval);
    }
    
    /// Initialize interface state for Network LSA generation
    pub fn initialize_interface_state(&mut self, interface_id: u32, interface_ip: String, network_mask: String) {
        self.interface_states.insert(interface_id, InterfaceState {
            is_dr: false,
            interface_ip: interface_ip.clone(),
            network_mask: network_mask.clone(),
            fully_adjacent_neighbors: HashSet::new(),
        });
        
        // Also initialize extended interface state
        let extended_state = ExtendedInterfaceState::new(
            interface_ip,
            network_mask,
            self.area_id.clone(),
            OSPFNetworkType::Broadcast,
        );
        self.interface_state_manager.add_interface(interface_id, extended_state);
        
        console_log!("Router {} initialized interface state for interface {}", self.router_id, interface_id);
    }
    
    /// Update DR status and generate Network LSA if needed
    pub fn update_dr_status(&mut self, interface_id: u32) -> Option<LSA> {
        let is_dr = if let Some(dr_manager) = self.dr_election_managers.get(&interface_id) {
            dr_manager.is_dr()
        } else {
            false
        };
        
        if let Some(interface_state) = self.interface_states.get_mut(&interface_id) {
            let was_dr = interface_state.is_dr;
            interface_state.is_dr = is_dr;
            
            // Update fully adjacent neighbors
            interface_state.fully_adjacent_neighbors.clear();
            interface_state.fully_adjacent_neighbors.insert(self.router_id.clone()); // DR itself
            
            // Add all Full neighbors on this interface
            for neighbor in self.neighbor_manager.get_all_neighbors() {
                if neighbor.state == OSPFNeighborState::Full && neighbor.interface_id == interface_id {
                    interface_state.fully_adjacent_neighbors.insert(neighbor.router_id.clone());
                }
            }
            
            // Generate Network LSA if we are DR and have adjacent neighbors
            if is_dr && interface_state.fully_adjacent_neighbors.len() > 1 {
                let lsa = self.network_lsa_generator.generate_network_lsa(
                    &interface_state.interface_ip,
                    &interface_state.network_mask,
                    &interface_state.fully_adjacent_neighbors,
                    self.lsa_manager.get_next_sequence_number(),
                );
                
                // Add to LSA database
                self.lsa_manager.add_lsa(lsa.clone());
                
                console_log!("Router {} generated Network LSA for interface {}", 
                    self.router_id, interface_id);
                
                return Some(lsa);
            } else if was_dr && !is_dr {
                // No longer DR - flush Network LSA
                let lsa = self.network_lsa_generator.create_maxage_network_lsa(
                    &interface_state.interface_ip,
                    &interface_state.network_mask,
                    self.lsa_manager.get_next_sequence_number(),
                );
                
                self.lsa_manager.add_lsa(lsa.clone());
                
                console_log!("Router {} flushing Network LSA for interface {} (no longer DR)", 
                    self.router_id, interface_id);
                
                return Some(lsa);
            }
        }
        
        None
    }
    
    /// Add an area to the connected areas (for ABR functionality)
    pub fn add_area(&mut self, area_id: String) {
        self.connected_areas.insert(area_id);
        console_log!("Router {} now connected to {} areas", 
            self.router_id, self.connected_areas.len());
    }
    
    /// Check if this router is an ABR
    pub fn is_abr(&self) -> bool {
        self.connected_areas.len() > 1
    }
    
    /// Check if this router is an ASBR
    pub fn is_asbr(&self) -> bool {
        !self.external_routes.is_empty()
    }
    
    /// Add an external route (for ASBR functionality)
    pub fn add_external_route(
        &mut self,
        network: String,
        mask: String,
        metric: u32,
        metric_type: ExternalMetricType,
        forwarding_address: String,
        tag: u32,
    ) {
        self.external_routes.push((network, mask, metric, metric_type, forwarding_address, tag));
        console_log!("Router {} added external route, now has {} external routes", 
            self.router_id, self.external_routes.len());
    }
    
    /// Generate Summary LSAs for inter-area routes
    pub fn generate_summary_lsas(&mut self, inter_area_routes: &[(String, String, u32, String)]) -> Vec<LSA> {
        if !self.is_abr() {
            return Vec::new();
        }
        
        let mut all_lsas = Vec::new();
        
        // Generate Summary LSAs for each connected area
        for area in &self.connected_areas.clone() {
            let sequence_start = self.lsa_manager.get_next_sequence_number();
            let lsas = self.summary_lsa_generator.generate_all_summary_lsas(
                inter_area_routes,
                area,
                sequence_start,
            );
            
            // Add to LSA database
            for lsa in &lsas {
                self.lsa_manager.add_lsa(lsa.clone());
            }
            
            all_lsas.extend(lsas);
        }
        
        if !all_lsas.is_empty() {
            console_log!("ABR {} generated {} Summary LSAs", 
                self.router_id, all_lsas.len());
        }
        
        all_lsas
    }
    
    /// Generate AS-External LSAs for external routes
    pub fn generate_as_external_lsas(&mut self) -> Vec<LSA> {
        if !self.is_asbr() {
            return Vec::new();
        }
        
        let sequence_start = self.lsa_manager.get_next_sequence_number();
        let lsas = self.as_external_lsa_generator.generate_all_as_external_lsas(
            &self.external_routes,
            sequence_start,
        );
        
        // Add to LSA database
        for lsa in &lsas {
            self.lsa_manager.add_lsa(lsa.clone());
        }
        
        if !lsas.is_empty() {
            console_log!("ASBR {} generated {} AS-External LSAs", 
                self.router_id, lsas.len());
        }
        
        lsas
    }
    
    /// Update authentication configuration for an interface
    pub fn update_interface_auth(&mut self, interface_id: u32, auth_config: AuthConfig) {
        self.packet_processor.update_interface_auth(interface_id, auth_config);
    }
    
    /// Get interface ID for a neighbor
    fn get_neighbor_interface(&self, neighbor_id: u32) -> u32 {
        // Try to get the actual interface ID from neighbor manager
        self.neighbor_manager.get_neighbor_interface(neighbor_id)
            .unwrap_or_else(|| {
                // Fallback: find the interface from router links
                self.lsa_manager.get_router_links()
                    .iter()
                    .find(|(n, _, _)| *n == neighbor_id)
                    .map(|(_, interface_id, _)| *interface_id)
                    .unwrap_or(1) // Default interface ID
            })
    }
    
    /// Verify packet authentication
    pub fn verify_packet_authentication(&self, packet: &OSPFPacket, interface_id: u32) -> bool {
        // Get expected authentication configuration for this interface
        let expected_auth_config = self.packet_processor.get_interface_auth_config(interface_id);
        
        match expected_auth_config {
            Some(auth_config) => {
                // Verify authentication
                match crate::ospf_auth::verify_authentication(
                    packet.auth_type.clone(),
                    &packet.auth_data,
                    &auth_config,
                ) {
                    Ok(()) => {
                        // Authentication successful
                        true
                    }
                    Err(err) => {
                        console_log!("Router {} authentication failed on interface {}: {}", 
                            self.router_id, interface_id, err);
                        false
                    }
                }
            }
            None => {
                // No authentication configured, accept only Null authentication
                if packet.auth_type == crate::ospf_auth::AuthType::Null {
                    true
                } else {
                    console_log!("Router {} received authenticated packet on interface {} without auth config", 
                        self.router_id, interface_id);
                    false
                }
            }
        }
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
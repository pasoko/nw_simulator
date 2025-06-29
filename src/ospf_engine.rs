use std::collections::HashMap;
use crate::ospf::{HelloPacket, DatabaseDescriptionPacket, 
    LinkStateRequestPacket, LinkStateUpdatePacket, LinkStateAcknowledgmentPacket};
use crate::router::{OSPFNeighborState, LSA as RouterLSA};
use crate::protocol::PacketEvent;
use crate::ospf_neighbor::OSPFNeighborManager;
use crate::ospf_lsa_manager::OSPFLSAManager;
use crate::ospf_packet_processor::OSPFPacketProcessor;
use crate::ospf_timer::{OSPFTimerManager, OSPFTimerEvent};
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
    
    // Specialized components
    neighbor_manager: OSPFNeighborManager,
    lsa_manager: OSPFLSAManager,
    packet_processor: OSPFPacketProcessor,
    timer_manager: OSPFTimerManager,
}

impl OSPFEngine {
    pub fn new(router_id: String, area_id: String) -> Self {
        OSPFEngine {
            neighbor_manager: OSPFNeighborManager::new(40), // 40s dead interval
            lsa_manager: OSPFLSAManager::new(router_id.clone()),
            packet_processor: OSPFPacketProcessor::new(router_id.clone(), area_id.clone()),
            timer_manager: OSPFTimerManager::new(router_id.clone()),
            router_id,
            area_id,
        }
    }
    
    pub fn update_time(&mut self, time: f64) {
        self.neighbor_manager.update_time(time);
        self.timer_manager.update_time(time);
        self.lsa_manager.age_lsas(0.1); // Small time delta for aging
        
        // Process expired timers
        let expired_events = self.timer_manager.process_expired_timers();
        for event in expired_events {
            match event {
                OSPFTimerEvent::HelloTimer => {
                    console_log!("Router {} hello timer expired", self.router_id);
                }
                OSPFTimerEvent::DeadTimer(neighbor_id) => {
                    console_log!("Router {} dead timer expired for neighbor {}", 
                        self.router_id, neighbor_id);
                    self.neighbor_manager.remove_neighbor(neighbor_id);
                }
                OSPFTimerEvent::LSARefresh => {
                    console_log!("Router {} LSA refresh timer expired", self.router_id);
                    // Generate new Router LSA
                    let _lsa = self.lsa_manager.regenerate_router_lsa();
                    console_log!("Router {} regenerated LSA", self.router_id);
                }
                OSPFTimerEvent::RetransmissionTimer(neighbor_id) => {
                    console_log!("Router {} retransmission timer expired for neighbor {}", 
                        self.router_id, neighbor_id);
                    // Handle retransmission logic
                }
            }
        }
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
                        if self.neighbor_manager.start_adjacency(from_router_id) {
                            // Send Database Description packet
                            events.push(self.packet_processor.create_dd_packet_event(
                                from_router_id, 
                                self.lsa_manager.get_lsa_database()
                            ));
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
        
        if let Some(current_state) = self.neighbor_manager.get_neighbor_state(from_router_id) {
            let (new_state, should_send_dd, lsa_headers_to_request) = self.packet_processor.process_dd_packet(
                packet, 
                from_router_id, 
                current_state
            );
            
            // Update neighbor state if needed
            if let Some(state) = new_state {
                self.neighbor_manager.update_neighbor_state(from_router_id, state.clone());
                
                match state {
                    OSPFNeighborState::Full => {
                        console_log!("Router {} neighbor {} reached Full state, triggering LSA generation and flooding", 
                            self.router_id, from_router_id);
                        
                        // Generate fresh Router LSA with all links
                        let router_lsa = self.lsa_manager.regenerate_router_lsa();
                        console_log!("Router {} generated Router LSA with {} links", 
                            self.router_id, self.lsa_manager.get_router_links().len());
                        
                        // Flood to all neighbors in Exchange or Full state
                        let flood_events = self.flood_lsa(&router_lsa);
                        console_log!("Router {} flooding LSA to {} neighbors", 
                            self.router_id, flood_events.len());
                        events.extend(flood_events);
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
                events.push(self.packet_processor.create_dd_packet_event(
                    from_router_id, 
                    self.lsa_manager.get_lsa_database()
                ));
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
        
        let (updated_lsas, _ack_headers, neighbor_to_full) = self.packet_processor.process_lsu_packet(packet, from_router_id);
        
        // Update LSA database
        for lsa in updated_lsas {
            if self.lsa_manager.should_update_lsa(&lsa) {
                self.lsa_manager.update_lsa_database(lsa);
            }
        }
        
        // Update neighbor state if needed
        if neighbor_to_full {
            self.neighbor_manager.update_neighbor_state(from_router_id, OSPFNeighborState::Full);
            
            console_log!("Router {} neighbor {} reached Full state via LSU, triggering LSA generation and flooding", 
                self.router_id, from_router_id);
            
            // Generate and flood fresh Router LSA
            let router_lsa = self.lsa_manager.regenerate_router_lsa();
            let flood_events = self.flood_lsa(&router_lsa);
            console_log!("Router {} flooding LSA to {} neighbors after LSU", 
                self.router_id, flood_events.len());
            events.extend(flood_events);
        }
        
        // Send acknowledgment (simplified)
        
        events
    }
    
    pub fn process_lsack_packet(&mut self, _packet: &LinkStateAcknowledgmentPacket, _from_router_id: u32) -> Vec<PacketEvent> {
        // Stop retransmission timers for acknowledged LSAs
        Vec::new()
    }
    
    pub fn generate_hello_packet(&self) -> HelloPacket {
        let active_neighbors = self.neighbor_manager.get_all_active_neighbors();
        self.packet_processor.generate_hello_packet(&active_neighbors)
    }
    
    pub fn add_router_link(&mut self, neighbor_id: u32, interface_id: u32, cost: u32) {
        self.lsa_manager.add_router_link(neighbor_id, interface_id, cost);
    }
    
    pub fn remove_link(&mut self, neighbor_id: u32) {
        self.lsa_manager.remove_router_link(neighbor_id);
    }
    
    pub fn add_link(&mut self, neighbor_id: u32, interface_id: u32, cost: u32) {
        self.lsa_manager.add_router_link(neighbor_id, interface_id, cost);
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
    
    pub fn get_neighbor_state_transitions(&self) -> HashMap<u32, (OSPFNeighborState, OSPFNeighborState)> {
        self.neighbor_manager.get_state_transitions()
    }
    
    pub fn generate_router_lsa(&mut self) -> RouterLSA {
        self.lsa_manager.generate_router_lsa()
    }
    
    pub fn regenerate_router_lsa(&mut self) -> Vec<PacketEvent> {
        let router_lsa = self.lsa_manager.regenerate_router_lsa();
        self.flood_lsa(&router_lsa)
    }
    
    pub fn update_lsa_database(&mut self, lsa: RouterLSA) {
        self.lsa_manager.update_lsa_database(lsa);
    }
    
    pub fn flood_lsa(&self, lsa: &RouterLSA) -> Vec<PacketEvent> {
        let mut events = Vec::new();
        
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
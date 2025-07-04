// Refactored OSPF Engine - Breaking down complex functions
//
// This file contains the refactored version of ospf_engine.rs with
// smaller, more focused functions following the single responsibility principle.

use crate::ospf::{HelloPacket, DatabaseDescriptionPacket, LinkStateUpdatePacket};
use crate::ospf_neighbor::{OSPFNeighborState, OSPFNeighborManager};
use crate::ospf_lsa_manager::OSPFLSAManager;
use crate::ospf_packet_processor::OSPFPacketProcessor;
use crate::ospf_timer::OSPFTimerManager;
use crate::ospf_dr_election::DRElectionManager;
use crate::event_manager::PacketEvent;
use std::collections::HashMap;

/// Refactored OSPF Engine with cleaner method organization
pub struct OSPFEngineRefactored {
    pub router_id: String,
    pub area_id: u32,
    pub current_time: f64,
    pub neighbor_manager: OSPFNeighborManager,
    pub lsa_manager: OSPFLSAManager,
    pub packet_processor: OSPFPacketProcessor,
    pub timer_manager: OSPFTimerManager,
    pub dr_election_managers: HashMap<u32, DRElectionManager>,
    pub spf_calculation_pending: bool,
}

impl OSPFEngineRefactored {
    /// Main entry point for processing hello packets - now clean and simple
    pub fn process_hello_packet(
        &mut self, 
        packet: &HelloPacket, 
        from_router_id: u32, 
        interface_id: u32
    ) -> Vec<PacketEvent> {
        let mut events = Vec::new();
        
        // Step 1: Validate and extract packet data
        let validation_result = self.validate_hello_packet(packet, from_router_id);
        if let Err(e) = validation_result {
            console_log!("Router {} rejected hello from {}: {}", self.router_id, from_router_id, e);
            return events;
        }
        
        let (should_process, hello_neighbors) = validation_result.unwrap();
        if !should_process {
            return events;
        }
        
        // Step 2: Update neighbor information
        self.update_neighbor_from_hello(from_router_id, interface_id, packet.router_priority);
        
        // Step 3: Handle state progression
        let state_events = self.handle_neighbor_state_progression(
            from_router_id, 
            interface_id, 
            &hello_neighbors
        );
        events.extend(state_events);
        
        // Step 4: Check for DR election requirements
        if self.should_trigger_dr_election(from_router_id, interface_id) {
            let dr_events = self.handle_dr_election(interface_id, from_router_id, packet);
            events.extend(dr_events);
        }
        
        // Step 5: Handle adjacency formation if needed
        if self.should_start_adjacency(from_router_id) {
            let adjacency_events = self.handle_adjacency_formation(from_router_id);
            events.extend(adjacency_events);
        }
        
        events
    }
    
    /// Validate hello packet and extract neighbor list
    fn validate_hello_packet(
        &self, 
        packet: &HelloPacket, 
        from_router_id: u32
    ) -> Result<(bool, Vec<String>), String> {
        // Delegate to packet processor for validation
        let (should_process, hello_neighbors) = self.packet_processor.process_hello_packet(packet, from_router_id);
        
        if !should_process {
            return Ok((false, vec![]));
        }
        
        // Additional validation could go here
        
        Ok((should_process, hello_neighbors))
    }
    
    /// Update neighbor information from hello packet
    fn update_neighbor_from_hello(
        &mut self, 
        from_router_id: u32, 
        interface_id: u32, 
        router_priority: u8
    ) {
        // Add or update neighbor
        let _is_new = self.neighbor_manager.add_or_update_neighbor(
            from_router_id, 
            interface_id, 
            router_priority
        );
        
        // Reset dead timer
        self.timer_manager.reset_neighbor_dead_timer(from_router_id);
    }
    
    /// Handle neighbor state progression based on hello packet
    fn handle_neighbor_state_progression(
        &mut self,
        from_router_id: u32,
        interface_id: u32,
        hello_neighbors: &[String]
    ) -> Vec<PacketEvent> {
        let current_state = self.neighbor_manager.get_neighbor_state(from_router_id);
        
        // Only progress from Down, Init, or TwoWay states
        match current_state {
            None | Some(OSPFNeighborState::Down) | Some(OSPFNeighborState::Init) | Some(OSPFNeighborState::TwoWay) => {
                let state_changed = self.neighbor_manager.progress_neighbor_state(
                    from_router_id,
                    hello_neighbors,
                    &self.router_id
                );
                
                if state_changed {
                    console_log!("Router {} neighbor {} state changed", self.router_id, from_router_id);
                }
            }
            Some(state) => {
                // For higher states, just verify bidirectional communication
                if !hello_neighbors.contains(&self.router_id) {
                    console_log!(
                        "Warning: Router {} not in neighbor {}'s hello packet while in state {:?}",
                        self.router_id, from_router_id, state
                    );
                }
            }
        }
        
        Vec::new()
    }
    
    /// Check if DR election should be triggered
    fn should_trigger_dr_election(&self, from_router_id: u32, interface_id: u32) -> bool {
        // Check if neighbor reached TwoWay state
        if let Some(OSPFNeighborState::TwoWay) = self.neighbor_manager.get_neighbor_state(from_router_id) {
            // Check if DR election is required on this interface
            if let Some(dr_manager) = self.dr_election_managers.get(&interface_id) {
                return dr_manager.is_election_required();
            }
        }
        false
    }
    
    /// Handle DR/BDR election process
    fn handle_dr_election(
        &mut self,
        interface_id: u32,
        from_router_id: u32,
        packet: &HelloPacket
    ) -> Vec<PacketEvent> {
        console_log!(
            "Router {} running DR election on interface {} due to TwoWay state",
            self.router_id, interface_id
        );
        
        // Collect Hello packets from neighbors on this interface
        let mut interface_neighbors = Vec::new();
        interface_neighbors.push((from_router_id, packet.clone()));
        
        // Run DR election with collected neighbors
        let election_changed = self.run_dr_election(interface_id, interface_neighbors);
        
        if election_changed {
            console_log!(
                "Router {} DR election changed on interface {} in TwoWay state",
                self.router_id, interface_id
            );
        }
        
        Vec::new()
    }
    
    /// Check if adjacency should be started with neighbor
    fn should_start_adjacency(&self, from_router_id: u32) -> bool {
        self.neighbor_manager.should_form_adjacency(from_router_id)
    }
    
    /// Handle adjacency formation process
    fn handle_adjacency_formation(&mut self, from_router_id: u32) -> Vec<PacketEvent> {
        let mut events = Vec::new();
        
        console_log!(
            "Router {} neighbor {} is ready for adjacency formation",
            self.router_id, from_router_id
        );
        
        if !self.neighbor_manager.start_adjacency(from_router_id) {
            return events;
        }
        
        console_log!("Router {} neighbor {} moved to ExStart", self.router_id, from_router_id);
        
        // Generate initial LSA if needed
        let lsa_events = self.generate_initial_lsa_if_needed();
        events.extend(lsa_events);
        
        // Send Database Description packet
        let dd_event = self.create_initial_dd_packet(from_router_id);
        events.push(dd_event);
        
        // Start DD retransmission timer
        self.start_dd_retransmission_if_needed(from_router_id);
        
        events
    }
    
    /// Generate initial Router LSA if needed
    fn generate_initial_lsa_if_needed(&mut self) -> Vec<PacketEvent> {
        let mut events = Vec::new();
        
        // Check if we need to generate initial LSA
        if self.lsa_manager.get_lsa_count() > 0 || self.lsa_manager.get_router_links().is_empty() {
            return events;
        }
        
        console_log!("Router {} generating initial Router LSA before DD exchange", self.router_id);
        let lsa = self.lsa_manager.generate_router_lsa();
        
        console_log!(
            "Router {} generated Router LSA with {} links, database now has {} LSAs",
            self.router_id,
            self.lsa_manager.get_router_links().len(),
            self.lsa_manager.get_lsa_count()
        );
        
        // Log LSA details
        console_log!(
            "  LSA: Type={:?}, ID={}, AdvRouter={}, SeqNum={}",
            lsa.header.ls_type,
            lsa.header.link_state_id,
            lsa.header.advertising_router,
            lsa.header.ls_sequence_number
        );
        
        // Flood LSA to neighbors in ExStart or higher state
        let flood_events = self.flood_initial_lsa(&lsa);
        events.extend(flood_events);
        
        events
    }
    
    /// Flood initial LSA to eligible neighbors
    fn flood_initial_lsa(&self, lsa: &crate::ospf::LSA) -> Vec<PacketEvent> {
        let exchange_neighbors = self.neighbor_manager.get_neighbors_in_state(OSPFNeighborState::ExStart);
        
        if exchange_neighbors.is_empty() {
            return Vec::new();
        }
        
        console_log!(
            "Router {} flooding initial LSA to {} neighbors in ExStart",
            self.router_id, exchange_neighbors.len()
        );
        
        self.flood_lsa(lsa)
    }
    
    /// Create initial Database Description packet
    fn create_initial_dd_packet(&self, from_router_id: u32) -> PacketEvent {
        self.packet_processor.create_dd_packet_event(
            from_router_id,
            self.lsa_manager.get_lsa_database()
        )
    }
    
    /// Start DD retransmission timer if needed
    fn start_dd_retransmission_if_needed(&mut self, from_router_id: u32) {
        if self.packet_processor.should_start_dd_retransmit(from_router_id) {
            self.timer_manager.start_dd_retransmission_timer(from_router_id);
            console_log!(
                "Router {} started DD retransmission timer for neighbor {}",
                self.router_id, from_router_id
            );
        }
    }
    
    // Placeholder methods that would be implemented
    fn run_dr_election(&mut self, interface_id: u32, neighbors: Vec<(u32, HelloPacket)>) -> bool {
        // Implementation would go here
        false
    }
    
    fn flood_lsa(&self, lsa: &crate::ospf::LSA) -> Vec<PacketEvent> {
        // Implementation would go here
        Vec::new()
    }
}

// Timer handling - refactored update_time
impl OSPFEngineRefactored {
    /// Main time update function - now clean and delegated
    pub fn update_time(&mut self, time: f64) -> Vec<PacketEvent> {
        let mut events = Vec::new();
        
        // Update current time and calculate delta
        let time_delta = self.calculate_time_delta(time);
        self.current_time = time;
        
        // Update time for all managers
        self.update_manager_times(time);
        
        // Handle LSA aging
        let aging_events = self.handle_lsa_aging(time_delta);
        events.extend(aging_events);
        
        // Process expired timers
        let timer_events = self.process_expired_timers(time);
        events.extend(timer_events);
        
        events
    }
    
    /// Calculate time delta since last update
    fn calculate_time_delta(&self, new_time: f64) -> f64 {
        if self.current_time > 0.0 {
            new_time - self.current_time
        } else {
            0.0
        }
    }
    
    /// Update time for all sub-managers
    fn update_manager_times(&mut self, time: f64) {
        self.neighbor_manager.update_time(time);
        self.timer_manager.update_time(time);
        self.lsa_manager.update_time(time);
    }
    
    /// Handle LSA aging and MaxAge reflooding
    fn handle_lsa_aging(&mut self, time_delta: f64) -> Vec<PacketEvent> {
        let mut events = Vec::new();
        
        // Age LSAs and get MaxAge LSAs that need reflooding
        let maxage_lsas = self.lsa_manager.age_lsas(time_delta);
        
        // Reflood MaxAge LSAs before deletion
        for lsa in maxage_lsas {
            console_log!(
                "Router {} reflooding MaxAge LSA: {}:{}",
                self.router_id, lsa.header.link_state_id, lsa.header.advertising_router
            );
            let flood_events = self.flood_lsa(&lsa);
            events.extend(flood_events);
        }
        
        events
    }
    
    /// Process all expired timers
    fn process_expired_timers(&mut self, time: f64) -> Vec<PacketEvent> {
        let mut events = Vec::new();
        
        let expired_events = self.timer_manager.process_expired_timers();
        if !expired_events.is_empty() {
            console_log!(
                "Router {} checking timers at {:.1}s, found {} expired events",
                self.router_id, time, expired_events.len()
            );
        }
        
        // Process each timer event with dedicated handler
        for event in expired_events {
            let timer_events = self.handle_timer_event(event);
            events.extend(timer_events);
        }
        
        events
    }
    
    /// Handle a single timer event
    fn handle_timer_event(&mut self, event: crate::ospf_timer::OSPFTimerEvent) -> Vec<PacketEvent> {
        use crate::ospf_timer::OSPFTimerEvent;
        
        match event {
            OSPFTimerEvent::HelloTimer => self.handle_hello_timer(),
            OSPFTimerEvent::DeadTimer(neighbor_id) => self.handle_dead_timer(neighbor_id),
            OSPFTimerEvent::LSARefresh => self.handle_lsa_refresh_timer(),
            OSPFTimerEvent::RetransmissionTimer(neighbor_id) => {
                self.handle_retransmission_timer(neighbor_id)
            }
            OSPFTimerEvent::DDRetransmissionTimer(neighbor_id) => {
                self.handle_dd_retransmission_timer(neighbor_id)
            }
            OSPFTimerEvent::SPFDelay => self.handle_spf_delay_timer(),
        }
    }
    
    /// Handle hello timer expiration
    fn handle_hello_timer(&mut self) -> Vec<PacketEvent> {
        console_log!("Router {} hello timer expired at {:.1}s", self.router_id, self.current_time);
        
        // Generate Hello packets for all neighbors
        let hello_events = self.generate_hello_events();
        console_log!("Router {} scheduling {} hello packets", self.router_id, hello_events.len());
        
        hello_events
    }
    
    /// Handle dead timer expiration
    fn handle_dead_timer(&mut self, neighbor_id: u32) -> Vec<PacketEvent> {
        console_log!(
            "Router {} dead timer expired for neighbor {}",
            self.router_id, neighbor_id
        );
        
        self.neighbor_manager.remove_neighbor(neighbor_id);
        
        Vec::new()
    }
    
    /// Handle LSA refresh timer
    fn handle_lsa_refresh_timer(&mut self) -> Vec<PacketEvent> {
        console_log!("Router {} LSA refresh timer expired", self.router_id);
        
        // Generate new Router LSA
        let lsa = self.lsa_manager.regenerate_router_lsa();
        
        // Only flood if we have neighbors
        if self.neighbor_manager.get_neighbor_count() > 0 {
            self.flood_lsa(&lsa)
        } else {
            Vec::new()
        }
    }
    
    /// Handle retransmission timer
    fn handle_retransmission_timer(&mut self, neighbor_id: u32) -> Vec<PacketEvent> {
        console_log!(
            "Router {} retransmission timer expired for neighbor {}",
            self.router_id, neighbor_id
        );
        
        // Retransmission logic would go here
        Vec::new()
    }
    
    /// Handle DD retransmission timer
    fn handle_dd_retransmission_timer(&mut self, neighbor_id: u32) -> Vec<PacketEvent> {
        console_log!(
            "Router {} DD retransmission timer expired for neighbor {}",
            self.router_id, neighbor_id
        );
        
        // Check if we should skip retransmission
        if self.should_skip_dd_retransmission(neighbor_id) {
            self.timer_manager.stop_dd_retransmission_timer(neighbor_id);
            return Vec::new();
        }
        
        // Perform DD retransmission
        self.perform_dd_retransmission(neighbor_id)
    }
    
    /// Check if DD retransmission should be skipped
    fn should_skip_dd_retransmission(&self, neighbor_id: u32) -> bool {
        // Skip if neighbor is in Full state
        if let Some(neighbor_state) = self.neighbor_manager.get_neighbor_state(neighbor_id) {
            if neighbor_state == OSPFNeighborState::Full {
                console_log!(
                    "Router {} ignoring DD retransmission for neighbor {} in Full state",
                    self.router_id, neighbor_id
                );
                return true;
            }
        }
        false
    }
    
    /// Perform DD packet retransmission
    fn perform_dd_retransmission(&mut self, neighbor_id: u32) -> Vec<PacketEvent> {
        let mut events = Vec::new();
        
        // Get last DD packet and retransmit
        if let Some(dd_packet) = self.packet_processor.get_last_dd_packet(neighbor_id) {
            console_log!(
                "Router {} retransmitting DD packet to neighbor {}",
                self.router_id, neighbor_id
            );
            
            let event = self.packet_processor.create_dd_retransmit_event(neighbor_id, dd_packet);
            events.push(event);
            
            // Restart the DD retransmission timer
            self.timer_manager.start_dd_retransmission_timer(neighbor_id);
        }
        
        events
    }
    
    /// Handle SPF delay timer
    fn handle_spf_delay_timer(&mut self) -> Vec<PacketEvent> {
        console_log!(
            "Router {} SPF delay timer expired, calculation can proceed",
            self.router_id
        );
        
        self.spf_calculation_pending = false;
        
        // The actual SPF calculation will be triggered by the simulation layer
        Vec::new()
    }
    
    /// Generate hello events (placeholder)
    fn generate_hello_events(&self) -> Vec<PacketEvent> {
        // Implementation would go here
        Vec::new()
    }
}

// Macro for consistent logging
macro_rules! console_log {
    ($($t:tt)*) => {
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&format!($($t)*).into());
        
        #[cfg(not(target_arch = "wasm32"))]
        println!($($t)*);
    };
}
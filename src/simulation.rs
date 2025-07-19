use std::collections::{HashMap, BTreeMap};
use crate::network::NetworkTopology;
use crate::protocol::{ProtocolEngine, PacketEvent, ProtocolPacket};
use crate::ospf::{OSPFPacket, OSPFPacketType, OSPFPacketData, HelloPacket};
use crate::ospf_engine::OSPFEngine;
use crate::event_manager::{EventManager, SimulationEvent, SimulationEventType};
use crate::failure_manager::FailureManager;
use crate::stub_area::AreaType;
use crate::route_calculator::RouteCalculator;
use crate::router::{OSPFNeighborState, RoutingTableEntry};
use crate::ping_manager::{PingManager, PingResult};
use crate::enhanced_ping::{EnhancedPingManager, PingSessionConfig, PingSessionSummary};
use crate::device::{ICMPPacket, ICMPType};
use crate::terminal_manager::{TerminalManager, ManagerConfig};
use crate::terminal_device::{TerminalDeviceInfo, TerminalConfig};
use crate::ospf_options::OSPFOptions;
use crate::console_log;

/// Refactored Network Simulation
/// 
/// A simplified simulation engine that delegates responsibilities to specialized components:
/// - EventManager: Event logging and tracking
/// - FailureManager: Router and link failure simulation
/// - RouteCalculator: Route calculation and optimization
/// - ProtocolEngine: Packet scheduling and delivery
pub struct NetworkSimulation {
    pub topology: NetworkTopology,
    pub protocol_engine: ProtocolEngine,
    pub simulation_time: f64,
    pub running: bool,
    
    // Specialized components
    event_manager: EventManager,
    failure_manager: FailureManager,
    route_calculator: RouteCalculator,
    ping_manager: PingManager,
    enhanced_ping_manager: EnhancedPingManager,
    terminal_manager: TerminalManager,
    ospf_engines: BTreeMap<u32, OSPFEngine>,  // Use BTreeMap for deterministic iteration order
    spf_needed: Vec<u32>,  // Track routers needing SPF calculation
    pause_time: Option<f64>,  // Time when simulation was paused
}

impl NetworkSimulation {
    pub fn new() -> Self {
        NetworkSimulation {
            topology: NetworkTopology::new(),
            protocol_engine: ProtocolEngine::new(),
            simulation_time: 0.0,
            running: false,
            event_manager: EventManager::new(),
            failure_manager: FailureManager::new(),
            route_calculator: RouteCalculator::new(),
            ping_manager: PingManager::new(),
            enhanced_ping_manager: EnhancedPingManager::new(),
            terminal_manager: TerminalManager::new(),
            ospf_engines: BTreeMap::new(),
            spf_needed: Vec::new(),
            pause_time: None,
        }
    }

    pub fn add_router(&mut self, name: String, _x: f64, _y: f64) -> u32 {
        let router_id = self.topology.add_router(name.clone());
        self.event_manager.log_router_added(router_id, name);
        router_id
    }

    pub fn add_host(&mut self, name: String, ip_address: String, netmask: String, default_gateway: String) -> u32 {
        let host_id = self.topology.add_host(name.clone(), ip_address, netmask, default_gateway);
        // TODO: イベントログ追加
        host_id
    }

    pub fn connect_host_to_router(&mut self, host_id: u32, router_id: u32) -> Result<u32, String> {
        let link_id = self.topology.connect_host_to_router(host_id, router_id)?;
        // TODO: イベントログ追加
        Ok(link_id)
    }

    pub fn connect_routers(&mut self, router1_id: u32, router2_id: u32, cost: u32) -> Result<(), String> {
        let link_id = self.topology.connect_routers(router1_id, router2_id, cost)?;
        
        // Update OSPF engines with new link information
        if let Some(link) = self.topology.links.get(&link_id) {
            if let Some(engine1) = self.ospf_engines.get_mut(&router1_id) {
                engine1.add_router_link(router2_id, link.router1_interface_id, cost);
                
                // Initialize interface state for Network LSA generation
                if let Some(router) = self.topology.routers.get(&router1_id) {
                    if let Some(interface) = router.interfaces.get(&link.router1_interface_id) {
                        engine1.initialize_interface_state(
                            link.router1_interface_id,
                            interface.ip_address.clone(),
                            interface.netmask.clone()
                        );
                    }
                }
                
                console_log!("Router {} link configuration updated", router1_id);
                // Only regenerate LSA if we have neighbors and the topology actually changed
                if engine1.get_neighbor_count() > 0 && engine1.get_lsa_count() > 0 {
                    console_log!("Router {} regenerating LSA after link addition", router1_id);
                    let events = engine1.regenerate_router_lsa();
                    // Schedule the flooding events
                    for mut event in events {
                        event.timestamp = self.simulation_time + 0.1;
                        self.protocol_engine.schedule_event(event);
                    }
                }
            }
            if let Some(engine2) = self.ospf_engines.get_mut(&router2_id) {
                engine2.add_router_link(router1_id, link.router2_interface_id, cost);
                
                // Initialize interface state for Network LSA generation
                if let Some(router) = self.topology.routers.get(&router2_id) {
                    if let Some(interface) = router.interfaces.get(&link.router2_interface_id) {
                        engine2.initialize_interface_state(
                            link.router2_interface_id,
                            interface.ip_address.clone(),
                            interface.netmask.clone()
                        );
                    }
                }
                
                console_log!("Router {} link configuration updated", router2_id);
                // Only regenerate LSA if we have neighbors and the topology actually changed
                if engine2.get_neighbor_count() > 0 && engine2.get_lsa_count() > 0 {
                    console_log!("Router {} regenerating LSA after link addition", router2_id);
                    let events = engine2.regenerate_router_lsa();
                    // Schedule the flooding events
                    for mut event in events {
                        event.timestamp = self.simulation_time + 0.1;
                        self.protocol_engine.schedule_event(event);
                    }
                }
            }
        }
        
        self.event_manager.log_link_created(router1_id, router2_id, cost);
        Ok(())
    }

    pub fn delete_router(&mut self, router_id: u32) -> bool {
        // Stop simulation if running
        if self.running {
            self.running = false;
        }
        
        // Remove router from topology
        if self.topology.routers.remove(&router_id).is_some() {
            // Remove all links connected to this router
            let links_to_remove: Vec<u32> = self.topology.links
                .iter()
                .filter(|(_, link)| link.router1_id == router_id || link.router2_id == router_id)
                .map(|(id, _)| *id)
                .collect();
            
            for link_id in links_to_remove {
                self.topology.links.remove(&link_id);
            }
            
            // Remove OSPF engine
            self.ospf_engines.remove(&router_id);
            
            self.event_manager.log_router_added(router_id, format!("Deleted Router {}", router_id));
            true
        } else {
            false
        }
    }
    
    pub fn disconnect_routers(&mut self, router1_id: u32, router2_id: u32) -> bool {
        let link_to_remove = self.topology.links
            .iter()
            .find(|(_, link)| {
                (link.router1_id == router1_id && link.router2_id == router2_id) ||
                (link.router1_id == router2_id && link.router2_id == router1_id)
            })
            .map(|(id, _)| *id);
        
        if let Some(link_id) = link_to_remove {
            self.topology.links.remove(&link_id);
            
            // Notify OSPF engines about the link failure
            let mut lsa_events = Vec::new();
            
            if let Some(engine1) = self.ospf_engines.get_mut(&router1_id) {
                if engine1.remove_neighbor(router2_id) {
                    self.event_manager.log_neighbor_state_changed(
                        router1_id, router2_id, "Active".to_string(), "Down".to_string()
                    );
                }
                // Remove link and regenerate LSA
                let link_events = engine1.remove_link(router2_id);
                lsa_events.extend(link_events);
            }
            
            if let Some(engine2) = self.ospf_engines.get_mut(&router2_id) {
                if engine2.remove_neighbor(router1_id) {
                    self.event_manager.log_neighbor_state_changed(
                        router2_id, router1_id, "Active".to_string(), "Down".to_string()
                    );
                }
                // Remove link and regenerate LSA
                let link_events = engine2.remove_link(router1_id);
                lsa_events.extend(link_events);
            }
            
            // Schedule LSA flooding events
            for mut event in lsa_events {
                event.timestamp = self.simulation_time + 0.1;
                self.protocol_engine.schedule_event(event);
            }
            
            // Remove scheduled packet events between these routers
            self.protocol_engine.events.retain(|event| {
                !((event.from_router_id == router1_id && event.to_router_id == router2_id) ||
                  (event.from_router_id == router2_id && event.to_router_id == router1_id))
            });
            
            // Request SPF calculation for affected routers (with delay)
            self.request_spf_for_router(router1_id);
            self.request_spf_for_router(router2_id);
            
            self.event_manager.log_link_created(router1_id, router2_id, 0);
            true
        } else {
            false
        }
    }
    
    pub fn enable_ospf(&mut self, router_id: u32) -> Result<(), String> {
        self.topology.enable_ospf_on_router(router_id)?;
        
        // Create OSPF engine for this router
        let router_ip = format!("{}.{}.{}.{}", 1, 1, 1, router_id);
        let mut ospf_engine = OSPFEngine::new(router_ip.clone(), "0.0.0.0".to_string());
        
        // Add router links to OSPF engine and initialize DR election
        for link in self.topology.links.values() {
            if link.router1_id == router_id {
                ospf_engine.add_router_link(link.router2_id, link.router1_interface_id, link.cost);
                // Initialize DR election for this interface
                ospf_engine.initialize_interface_dr_election(link.router1_interface_id, link.network_type.clone(), 1);
                
                // Initialize interface state for Network LSA generation
                if let Some(router) = self.topology.routers.get(&router_id) {
                    if let Some(interface) = router.interfaces.get(&link.router1_interface_id) {
                        ospf_engine.initialize_interface_state(
                            link.router1_interface_id,
                            interface.ip_address.clone(),
                            interface.netmask.clone()
                        );
                    }
                }
            } else if link.router2_id == router_id {
                ospf_engine.add_router_link(link.router1_id, link.router2_interface_id, link.cost);
                // Initialize DR election for this interface
                ospf_engine.initialize_interface_dr_election(link.router2_interface_id, link.network_type.clone(), 1);
                
                // Initialize interface state for Network LSA generation
                if let Some(router) = self.topology.routers.get(&router_id) {
                    if let Some(interface) = router.interfaces.get(&link.router2_interface_id) {
                        ospf_engine.initialize_interface_state(
                            link.router2_interface_id,
                            interface.ip_address.clone(),
                            interface.netmask.clone()
                        );
                    }
                }
            }
        }
        
        // Set authentication configuration for all interfaces
        if let Some(router) = self.topology.routers.get(&router_id) {
            for (interface_id, interface) in &router.interfaces {
                ospf_engine.update_interface_auth(*interface_id, interface.auth_config.clone());
            }
        }
        
        // IMPORTANT: Generate initial Router LSA immediately
        // This ensures LSA exists in database for DD exchange
        console_log!("Router {} generating initial Router LSA with {} configured links", 
            router_id, ospf_engine.get_router_links().len());
        
        // Generate the initial LSA
        let initial_lsa = ospf_engine.generate_router_lsa();
        console_log!("Router {} initial LSA generated: seq={}, links={}", 
            router_id, initial_lsa.header.ls_sequence_number, 
            match &initial_lsa.data {
                crate::router::LSAData::Router(data) => data.links.len(),
                _ => 0,
            });
        
        self.ospf_engines.insert(router_id, ospf_engine);
        self.event_manager.log_ospf_enabled(router_id);
        
        // OSPFv2 compliance: Do NOT calculate initial routes
        // Routes should only be calculated after:
        // 1. Neighbor adjacencies reach Full state
        // 2. LSA database is populated
        // 3. SPF calculation is triggered (with delay)
        console_log!("OSPF enabled on router {}, routes will be calculated after protocol convergence", router_id);
        
        // Clear any existing routes for this router to ensure clean state
        if let Some(router) = self.topology.routers.get_mut(&router_id) {
            router.routing_table.clear();
            console_log!("Router {} routing table cleared for OSPFv2 compliance", router_id);
        }
        
        // OSPF engine will manage Hello timers internally
        
        Ok(())
    }

    pub fn start_simulation(&mut self) {
        self.running = true;
        
        // Check if this is a resume from pause
        let is_resuming = self.pause_time.is_some();
        
        if is_resuming {
            console_log!("Resuming simulation from {:.1}s", self.simulation_time);
            // Clear pause time
            self.pause_time = None;
        } else if self.simulation_time == 0.0 {
            console_log!("Starting fresh simulation");
        }
        
        let router_ids: Vec<u32> = self.topology.routers
            .iter()
            .filter(|(_, router)| router.ospf_state.is_some())
            .map(|(id, _)| *id)
            .collect();
        
        console_log!("Simulation with {} OSPF-enabled routers", router_ids.len());
        
        // If resuming, force immediate timer check for all OSPF engines
        if is_resuming {
            console_log!("Forcing timer check after resume");
            let mut timer_events = Vec::new();
            for (router_id, engine) in self.ospf_engines.iter_mut() {
                // Force timer processing at current simulation time
                let events = engine.update_time(self.simulation_time);
                if !events.is_empty() {
                    console_log!("Router {} generated {} events on resume", router_id, events.len());
                    timer_events.extend(events);
                }
            }
            
            // Schedule the timer events
            for mut event in timer_events {
                event.timestamp = self.simulation_time + 0.1;
                self.protocol_engine.schedule_event(event);
            }
        }
        
        console_log!("OSPF engines will manage Hello timers internally");
    }
    
    pub fn stop_simulation(&mut self) {
        self.running = false;
        self.pause_time = Some(self.simulation_time);
        console_log!("Simulation paused at {:.1}s", self.simulation_time);
    }
    
    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn step_simulation(&mut self, time_delta: f64) {
        if !self.running {
            return;
        }

        let target_time = self.simulation_time + time_delta;
        
        // Update component times
        self.event_manager.update_time(target_time);
        self.failure_manager.update_time(target_time);
        self.route_calculator.update_time(target_time);
        
        // Process scheduled events with a limit to prevent infinite loops
        let max_events_per_step = 500;  // Increased limit to handle Hello packets for all routers
        let mut events_processed = 0;
        
        while let Some(event) = self.protocol_engine.process_next_event() {
            if event.timestamp > target_time {
                self.protocol_engine.events.insert(0, event);
                break;
            }
            
            self.simulation_time = event.timestamp;
            self.process_packet_event(event);
            
            events_processed += 1;
            if events_processed >= max_events_per_step {
                console_log!("Warning: Processed {} events in one step, deferring remaining events", 
                    max_events_per_step);
                break;
            }
        }
        
        // Log event queue size if it's growing too large
        if self.protocol_engine.events.len() > 1000 {
            console_log!("Warning: Event queue has {} pending events", 
                self.protocol_engine.events.len());
        }
        
        self.simulation_time = target_time;
        
        // Update all OSPF engines' time after processing events
        let mut ospf_events = Vec::new();
        let mut spf_ready_routers = Vec::new();
        
        // First pass: Check which routers have pending SPF calculations
        // and add them to spf_needed list if not already there
        for (router_id, engine) in self.ospf_engines.iter() {
            if engine.is_spf_pending() && !self.spf_needed.contains(router_id) {
                self.spf_needed.push(*router_id);
                console_log!("Router {} added to spf_needed list (pending SPF detected)", router_id);
            }
        }
        
        // Collect router IDs that need SPF before mutating engines
        let spf_needed_snapshot: Vec<u32> = self.spf_needed.clone();
        
        for (router_id, engine) in self.ospf_engines.iter_mut() {
            let events = engine.update_time(target_time);
            if !events.is_empty() {
                console_log!("Router {} generated {} events from timer processing", router_id, events.len());
            }
            ospf_events.extend(events);
            
            // Check if SPF timer expired and calculation is ready
            let spf_pending = engine.is_spf_pending();
            let db_updated = engine.was_lsa_database_updated();
            let in_snapshot = spf_needed_snapshot.contains(router_id);
            
            console_log!("Router {} SPF check: pending={}, db_updated={}, in_snapshot={}", 
                router_id, spf_pending, db_updated, in_snapshot);
            
            if !spf_pending {
                // Check if we have a deferred SPF calculation and database was updated
                if in_snapshot && db_updated {
                    console_log!("Router {} added to SPF ready list", router_id);
                    spf_ready_routers.push(*router_id);
                }
            }
        }
        
        // Schedule OSPF timer events
        for mut event in ospf_events {
            event.timestamp = self.simulation_time + 0.1; // Small delay for packet processing
            self.protocol_engine.schedule_event(event);
        }
        
        // Run SPF calculations for routers where the delay timer has expired
        for router_id in spf_ready_routers {
            console_log!("Router {} running delayed SPF calculation", router_id);
            self.route_calculator.calculate_routes_for_router(
                router_id, &mut self.topology, &mut self.ospf_engines, &mut self.event_manager
            );
            self.clear_spf_needed(router_id);
            
            // Reset the database updated flag after SPF calculation
            if let Some(engine) = self.ospf_engines.get_mut(&router_id) {
                engine.reset_database_updated_flag();
            }
        }
        
        // Process terminal device packet queues and update statistics
        self.process_terminal_packet_queues();
        self.update_terminal_statistics();
        
        // Check ping timeouts
        self.check_ping_timeouts();
    }
    
    pub fn toggle_link_failure(&mut self, from_id: u32, to_id: u32) -> bool {
        console_log!("=== LINK FAILURE DEBUG: simulation.rs toggle_link_failure called ===");
        console_log!("  from_id: {}, to_id: {}, simulation_time: {:.2}s", from_id, to_id, self.simulation_time);
        
        let (success, events) = self.failure_manager.toggle_link_failure(
            from_id, to_id, &mut self.topology, &mut self.ospf_engines, &mut self.event_manager
        );
        
        console_log!("  failure_manager.toggle_link_failure returned: success={}, {} events", success, events.len());
        
        // Schedule flooding events
        for (i, mut event) in events.into_iter().enumerate() {
            event.timestamp = self.simulation_time + 0.1;
            console_log!("  Scheduling event {}: {} -> {}", i + 1, event.from_router_id, event.to_router_id);
            self.protocol_engine.schedule_event(event);
        }
        
        console_log!("=== LINK FAILURE DEBUG: simulation.rs toggle_link_failure completed ===");
        success
    }
    
    pub fn toggle_router_failure(&mut self, router_id: u32) -> bool {
        self.failure_manager.toggle_router_failure(
            router_id, &mut self.topology, &mut self.ospf_engines, &mut self.event_manager
        )
    }

    pub fn get_recent_events(&self, count: usize) -> Vec<SimulationEvent> {
        self.event_manager.get_recent_events(count)
    }
    
    pub fn clear_event_log(&mut self) {
        self.event_manager.clear_log();
    }
    
    pub fn get_ospf_neighbor_count(&self, router_id: u32) -> usize {
        self.ospf_engines.get(&router_id)
            .map(|engine| engine.get_neighbor_count())
            .unwrap_or(0)
    }
    
    pub fn get_ospf_lsa_count(&self, router_id: u32) -> usize {
        self.ospf_engines.get(&router_id)
            .map(|engine| engine.get_lsa_count())
            .unwrap_or(0)
    }
    
    // Expose simulation log for external access
    pub fn simulation_log(&self) -> &Vec<SimulationEvent> {
        self.event_manager.get_all_events()
    }
    
    // Private helper methods

    fn process_packet_event(&mut self, event: PacketEvent) {
        // Check for failed routers/links
        if self.is_packet_dropped(&event) {
            return;
        }
        
        // Log and process packet
        match &event.packet {
            ProtocolPacket::OSPF(ospf_packet) => {
                let packet_type = self.get_packet_type_string(&ospf_packet.packet_type);
                
                if event.from_router_id != event.to_router_id {
                    self.event_manager.log_packet_sent(
                        event.from_router_id, 
                        event.to_router_id, 
                        packet_type.clone()
                    );
                }
                
                let _delivery_time = event.timestamp + 0.01;
                let packet_details = self.get_packet_details(&ospf_packet.data);
                
                if event.from_router_id != event.to_router_id {
                    self.event_manager.log_packet_received(
                        event.to_router_id, 
                        packet_type, 
                        packet_details
                    );
                }
            },
            ProtocolPacket::ICMP(icmp_packet) => {
                // ICMPパケットの処理
                self.process_icmp_packet(icmp_packet.clone(), event.from_router_id, event.to_router_id);
                return; // ICMPはOSPFエンジンで処理しない
            }
        }
        
        // OSPFパケットの処理
        if let ProtocolPacket::OSPF(ospf_packet) = event.packet {
            self.process_ospf_packet(ospf_packet, event.from_router_id, event.to_router_id);
        }
    }
    
    fn is_packet_dropped(&self, event: &PacketEvent) -> bool {
        // Check if source router is failed
        if let Some(router) = self.topology.routers.get(&event.from_router_id) {
            if router.is_failed {
                console_log!("Dropping packet from failed router {}", event.from_router_id);
                return true;
            }
        }
        
        // Check if destination router is failed
        if let Some(router) = self.topology.routers.get(&event.to_router_id) {
            if router.is_failed {
                console_log!("Dropping packet to failed router {}", event.to_router_id);
                return true;
            }
        }
        
        // Check if the link is failed
        let link_failed = self.topology.links.values().any(|link| {
            ((link.router1_id == event.from_router_id && link.router2_id == event.to_router_id) ||
             (link.router1_id == event.to_router_id && link.router2_id == event.from_router_id)) &&
            link.is_failed
        });
        
        if link_failed {
            console_log!("Dropping packet on failed link between router {} and router {}", 
                event.from_router_id, event.to_router_id);
            return true;
        }
        
        false
    }
    
    fn get_packet_type_string(&self, packet_type: &OSPFPacketType) -> String {
        match packet_type {
            OSPFPacketType::Hello => "Hello",
            OSPFPacketType::DatabaseDescription => "Database Description",
            OSPFPacketType::LinkStateRequest => "Link State Request", 
            OSPFPacketType::LinkStateUpdate => "Link State Update",
            OSPFPacketType::LinkStateAcknowledgment => "Link State Acknowledgment",
        }.to_string()
    }
    
    fn get_packet_details(&self, data: &OSPFPacketData) -> String {
        match data {
            OSPFPacketData::Hello(hello) => {
                format!("Hello packet - Interval: {}s, Dead: {}s, Priority: {}, DR: {}, BDR: {}, Neighbors: [{}]",
                    hello.hello_interval,
                    hello.router_dead_interval,
                    hello.router_priority,
                    hello.designated_router,
                    hello.backup_designated_router,
                    hello.neighbors.join(", ")
                )
            },
            OSPFPacketData::DatabaseDescription(dd) => {
                let mut lsa_headers = Vec::new();
                for header in &dd.lsa_headers {
                    if let Some(id) = header.advertising_router.split('.').last().and_then(|s| s.parse::<u32>().ok()) {
                        lsa_headers.push(id);
                    }
                }
                format!("Database Description - MTU: {}, Flags: {:#04x}, Seq: {}, LSA headers: {} from routers {:?}",
                    dd.interface_mtu,
                    dd.flags,
                    dd.dd_sequence_number,
                    dd.lsa_headers.len(),
                    lsa_headers
                )
            },
            OSPFPacketData::LinkStateRequest(lsr) => {
                let mut requested_routers = Vec::new();
                for req in &lsr.requests {
                    if let Some(id) = req.link_state_id.split('.').last().and_then(|s| s.parse::<u32>().ok()) {
                        requested_routers.push(id);
                    }
                }
                format!("Link State Request - Requesting {} LSAs from routers {:?}", lsr.requests.len(), requested_routers)
            },
            OSPFPacketData::LinkStateUpdate(lsu) => {
                let mut lsa_sources = Vec::new();
                for lsa in &lsu.lsas {
                    if let Some(id) = lsa.header.advertising_router.split('.').last().and_then(|s| s.parse::<u32>().ok()) {
                        lsa_sources.push(id);
                    }
                }
                format!("Link State Update - Contains {} LSAs from routers {:?}", lsu.lsas.len(), lsa_sources)
            },
            OSPFPacketData::LinkStateAcknowledgment(lsack) => {
                format!("Link State Acknowledgment - Acknowledging {} LSAs", lsack.lsa_headers.len())
            },
        }
    }
    
    
    fn should_send_hello_to_neighbor(&self, router_id: u32, neighbor_id: u32) -> bool {
        // Check if the link is failed
        let link_failed = self.topology.links.values().any(|link| {
            ((link.router1_id == router_id && link.router2_id == neighbor_id) ||
             (link.router1_id == neighbor_id && link.router2_id == router_id)) &&
            link.is_failed
        });
        
        if link_failed {
            console_log!("  Skipping hello to router {} - link is failed", neighbor_id);
            return false;
        }
        
        // Check if neighbor router is failed
        if let Some(neighbor_router) = self.topology.routers.get(&neighbor_id) {
            if neighbor_router.is_failed {
                console_log!("  Skipping hello to router {} - router is failed", neighbor_id);
                return false;
            }
            
            if neighbor_router.ospf_state.is_none() {
                console_log!("  Skipping hello to router {} - OSPF not enabled", neighbor_id);
                return false;
            }
        }
        
        true
    }

    fn create_hello_packet(&self, router_id: u32) -> OSPFPacket {
        let router = &self.topology.routers[&router_id];
        let ospf_state = router.ospf_state.as_ref().unwrap();
        
        let hello_packet = if let Some(engine) = self.ospf_engines.get(&router_id) {
            engine.generate_hello_packet()
        } else {
            HelloPacket {
                network_mask: "255.255.255.252".to_string(),
                hello_interval: 10,
                options: OSPFOptions::new(),
                router_priority: 1,
                router_dead_interval: 40,
                designated_router: "0.0.0.0".to_string(),
                backup_designated_router: "0.0.0.0".to_string(),
                neighbors: Vec::new(),
            }
        };
        
        OSPFPacket {
            version: 2,
            packet_type: OSPFPacketType::Hello,
            router_id: ospf_state.router_id.clone(),
            area_id: ospf_state.area_id.clone(),
            checksum: 0,
            auth_type: crate::ospf_auth::AuthType::Null,
            auth_data: crate::ospf_auth::AuthData::None,
            data: OSPFPacketData::Hello(hello_packet),
        }
    }
    
    fn process_ospf_packet(&mut self, packet: OSPFPacket, from_router_id: u32, to_router_id: u32) {
        // RFC 2328 Section 9.2 - Area ID Validation
        // Check if packet's area ID matches the receiving router's area ID
        if let Some(engine) = self.ospf_engines.get(&to_router_id) {
            if packet.area_id != engine.get_area_id() {
                console_log!("Router {} discarding packet from {} - Area ID mismatch (packet area: {}, router area: {})", 
                    to_router_id, from_router_id, packet.area_id, engine.get_area_id());
                self.event_manager.log_packet_discarded(to_router_id, from_router_id, 
                    format!("Area ID mismatch: packet area {} != router area {}", 
                        packet.area_id, engine.get_area_id()));
                return;
            }
        }
        
        // Get interface ID before mutable borrow
        let interface_id = self.get_interface_id(from_router_id, to_router_id);
        
        // Verify authentication before processing packet
        if let Some(engine) = self.ospf_engines.get(&to_router_id) {
            if !engine.verify_packet_authentication(&packet, interface_id) {
                console_log!("Router {} discarding packet from {} - Authentication failed", 
                    to_router_id, from_router_id);
                self.event_manager.log_packet_discarded(to_router_id, from_router_id, 
                    "Authentication verification failed".to_string());
                return;
            }
        }
        
        let (new_events, _lsa_updated, lsa_count, lsa_database_changed, state_transitions) = 
            if let Some(engine) = self.ospf_engines.get_mut(&to_router_id) {
                let lsa_count_before = engine.get_lsa_count();
                
                let new_events = match &packet.data {
                    OSPFPacketData::Hello(hello) => {
                        engine.process_hello_packet(hello, from_router_id, interface_id)
                    }
                    OSPFPacketData::DatabaseDescription(dd) => {
                        engine.process_dd_packet(dd, from_router_id)
                    }
                    OSPFPacketData::LinkStateRequest(lsr) => {
                        engine.process_lsr_packet(lsr, from_router_id)
                    }
                    OSPFPacketData::LinkStateUpdate(lsu) => {
                        engine.process_lsu_packet(lsu, from_router_id, Some(interface_id))
                    }
                    OSPFPacketData::LinkStateAcknowledgment(lsack) => {
                        engine.process_lsack_packet(lsack, from_router_id)
                    }
                };
                
                let lsa_updated = matches!(&packet.data, OSPFPacketData::LinkStateUpdate(_)) 
                    || matches!(&packet.data, OSPFPacketData::DatabaseDescription(_));
                let lsa_count = engine.get_lsa_count();
                
                // Check if database was actually updated (not just count change)
                let lsa_database_changed = if matches!(&packet.data, OSPFPacketData::LinkStateUpdate(_)) {
                    engine.was_lsa_database_updated()
                } else {
                    lsa_count != lsa_count_before
                };
                
                let state_transitions = engine.get_neighbor_state_transitions();
                
                console_log!("Router {} packet processing result: lsa_count_before={}, lsa_count_after={}, db_changed={}",
                    to_router_id, lsa_count_before, lsa_count, lsa_database_changed);
                
                (new_events, lsa_updated, lsa_count, lsa_database_changed, state_transitions)
            } else {
                return;
            };
        
        // Schedule response packets
        for mut event in new_events {
            event.timestamp = self.simulation_time + 0.1;
            self.protocol_engine.schedule_event(event);
        }
        
        // Request SPF calculation when LSA database changes (with delay per RFC 2328 Section 16.1)
        // Don't calculate routes during DD exchange
        if matches!(&packet.data, OSPFPacketData::LinkStateUpdate(_)) {
            console_log!("Router {} received LSU: lsa_database_changed={}, lsa_count={}", 
                to_router_id, lsa_database_changed, lsa_count);
            
            if lsa_database_changed && lsa_count > 0 {
                console_log!("Router {} LSA database changed due to LSU, requesting SPF calculation", to_router_id);
                if let Some(engine) = self.ospf_engines.get_mut(&to_router_id) {
                    engine.request_spf_calculation();
                    if !self.spf_needed.contains(&to_router_id) {
                        self.spf_needed.push(to_router_id);
                        console_log!("Router {} added to spf_needed list", to_router_id);
                    }
                }
            }
        }
        
        // Process neighbor state transitions
        self.process_state_transitions(state_transitions, to_router_id, from_router_id);
    }
    
    fn get_interface_id(&self, from_router_id: u32, to_router_id: u32) -> u32 {
        self.topology.links.values()
            .find(|link| {
                (link.router1_id == from_router_id && link.router2_id == to_router_id) ||
                (link.router1_id == to_router_id && link.router2_id == from_router_id)
            })
            .map(|link| {
                if link.router1_id == to_router_id {
                    link.router1_interface_id
                } else {
                    link.router2_interface_id
                }
            })
            .unwrap_or(0)
    }
    
    fn process_state_transitions(&mut self, state_transitions: HashMap<u32, (OSPFNeighborState, OSPFNeighborState)>, 
        to_router_id: u32, from_router_id: u32) {
        for (neighbor_id, (prev_state, new_state)) in state_transitions {
            if prev_state == new_state {
                continue;
            }
            
            let prev_state_name = self.get_state_name(&prev_state);
            let new_state_name = self.get_state_name(&new_state);
            
            self.event_manager.log_neighbor_state_changed(
                to_router_id, neighbor_id, prev_state_name, new_state_name
            );
            
            // Request SPF calculation when adjacency is established or lost (with delay)
            match new_state {
                OSPFNeighborState::Full => {
                    console_log!("Router {} neighbor {} reached Full state, checking if LSA regeneration needed", 
                        to_router_id, neighbor_id);
                    
                    // Check if this router needs to regenerate its LSA
                    // This is important for link recovery scenarios
                    if let Some(engine) = self.ospf_engines.get_mut(&to_router_id) {
                        if engine.needs_lsa_regeneration() {
                            console_log!("Router {} regenerating LSA after neighbor {} reached Full state", 
                                to_router_id, neighbor_id);
                            let events = engine.regenerate_router_lsa();
                            // Schedule the flooding events
                            for mut event in events {
                                event.timestamp = self.simulation_time + 0.1;
                                self.protocol_engine.schedule_event(event);
                            }
                        }
                    }
                    
                    // Request delayed SPF for affected routers
                    self.request_spf_for_router(to_router_id);
                    self.request_spf_for_router(from_router_id);
                    
                    // Trigger route calculation for all other OSPF routers
                    let ospf_routers: Vec<u32> = self.ospf_engines.keys().cloned().collect();
                    for router_id in ospf_routers {
                        if router_id != to_router_id && router_id != from_router_id {
                            self.request_spf_for_router(router_id);
                        }
                    }
                }
                OSPFNeighborState::Down => {
                    self.request_spf_for_router(to_router_id);
                }
                _ => {}
            }
        }
    }
    
    fn get_state_name(&self, state: &OSPFNeighborState) -> String {
        match state {
            OSPFNeighborState::Down => "Down",
            OSPFNeighborState::Init => "Init",
            OSPFNeighborState::TwoWay => "TwoWay",
            OSPFNeighborState::ExStart => "ExStart",
            OSPFNeighborState::Exchange => "Exchange",
            OSPFNeighborState::Loading => "Loading",
            OSPFNeighborState::Full => "Full",
        }.to_string()
    }
    
    fn needs_spf_calculation(&self, router_id: u32) -> bool {
        self.spf_needed.contains(&router_id)
    }
    
    fn clear_spf_needed(&mut self, router_id: u32) {
        self.spf_needed.retain(|&id| id != router_id);
    }
    
    fn request_spf_for_router(&mut self, router_id: u32) {
        if let Some(engine) = self.ospf_engines.get_mut(&router_id) {
            // Only request if not already pending
            if !engine.is_spf_pending() {
                engine.request_spf_calculation();
                if !self.spf_needed.contains(&router_id) {
                    self.spf_needed.push(router_id);
                }
            }
        }
    }
    
    pub fn get_ospf_engine(&self, router_id: u32) -> Option<&OSPFEngine> {
        self.ospf_engines.get(&router_id)
    }
    
    pub fn get_ospf_engine_mut(&mut self, router_id: u32) -> Option<&mut OSPFEngine> {
        self.ospf_engines.get_mut(&router_id)
    }
    
    /// Configure area as stub area
    pub fn configure_stub_area(&mut self, router_id: u32, area_type: AreaType) -> Result<(), String> {
        if let Some(engine) = self.get_ospf_engine_mut(router_id) {
            let area_type_str = format!("{:?}", area_type);
            engine.configure_stub_area(area_type)?;
            
            let event = SimulationEvent::stub_area_configured(
                self.simulation_time, 
                router_id,
                area_type_str
            );
            self.event_manager.log_event(event);
            
            Ok(())
        } else {
            Err(format!("OSPF not enabled on router {}", router_id))
        }
    }
    
    /// Configure a virtual link between two ABRs
    pub fn configure_virtual_link(
        &mut self, 
        local_router_id: u32, 
        remote_router_id: u32, 
        transit_area_id: String,
    ) -> Result<u32, String> {
        // Get remote router ID string
        let remote_router_id_str = if let Some(router) = self.topology.routers.get(&remote_router_id) {
            if let Some(ospf_state) = &router.ospf_state {
                ospf_state.router_id.clone()
            } else {
                return Err(format!("OSPF not enabled on router {}", remote_router_id));
            }
        } else {
            return Err(format!("Router {} not found", remote_router_id));
        };
        
        // Get the highest interface ID across all routers to ensure uniqueness
        let mut max_interface_id = 1000u32; // Start from a high number for virtual links
        for router in self.topology.routers.values() {
            for interface_id in router.interfaces.keys() {
                if *interface_id >= max_interface_id {
                    max_interface_id = *interface_id + 1;
                }
            }
        }
        
        // Configure virtual link on local router
        if let Some(engine) = self.get_ospf_engine_mut(local_router_id) {
            let interface_id = engine.configure_virtual_link(
                remote_router_id_str.clone(),
                transit_area_id.clone(),
                max_interface_id,
            )?;
            
            console_log!(
                "Virtual link configured: {} -> {} through area {}",
                local_router_id, remote_router_id, transit_area_id
            );
            
            // Log event
            let event = SimulationEvent {
                timestamp: self.simulation_time,
                event_type: SimulationEventType::VirtualLinkConfigured {
                    local_router_id,
                    remote_router_id,
                    transit_area_id: transit_area_id.clone(),
                    interface_id,
                },
                description: format!("Virtual link configured: {} -> {} through area {}", 
                    local_router_id, remote_router_id, transit_area_id),
            };
            self.event_manager.log_event(event);
            
            Ok(interface_id)
        } else {
            Err(format!("OSPF not enabled on router {}", local_router_id))
        }
    }
    
    /// Remove a virtual link
    pub fn remove_virtual_link(
        &mut self, 
        local_router_id: u32, 
        remote_router_id: u32,
    ) -> Result<(), String> {
        // Get remote router ID string
        let remote_router_id_str = if let Some(router) = self.topology.routers.get(&remote_router_id) {
            if let Some(ospf_state) = &router.ospf_state {
                ospf_state.router_id.clone()
            } else {
                return Err(format!("OSPF not enabled on router {}", remote_router_id));
            }
        } else {
            return Err(format!("Router {} not found", remote_router_id));
        };
        
        if let Some(engine) = self.get_ospf_engine_mut(local_router_id) {
            if engine.remove_virtual_link(&remote_router_id_str) {
                console_log!(
                    "Virtual link removed: {} -> {}",
                    local_router_id, remote_router_id
                );
                
                // Log event
                let event = SimulationEvent {
                    timestamp: self.simulation_time,
                    event_type: SimulationEventType::VirtualLinkRemoved {
                        local_router_id,
                        remote_router_id,
                    },
                    description: format!("Virtual link removed: {} -> {}", local_router_id, remote_router_id),
                };
                self.event_manager.log_event(event);
                
                Ok(())
            } else {
                Err(format!("Virtual link {} -> {} not found", local_router_id, remote_router_id))
            }
        } else {
            Err(format!("OSPF not enabled on router {}", local_router_id))
        }
    }
    
    /// Get virtual link status for all routers
    pub fn get_virtual_link_status(&self) -> Vec<(u32, Vec<(String, String, String, bool)>)> {
        let mut status = Vec::new();
        
        for (router_id, engine) in &self.ospf_engines {
            let vlink_status = engine.get_virtual_link_status();
            if !vlink_status.is_empty() {
                status.push((*router_id, vlink_status));
            }
        }
        
        status
    }
    
    /// Configure route aggregation on a router
    pub fn configure_route_aggregation(
        &mut self,
        router_id: u32,
        network: String,
        mask: String,
        area_id: Option<String>,
        suppress: bool,
        metric: Option<u32>,
    ) -> Result<(), String> {
        if let Some(engine) = self.get_ospf_engine_mut(router_id) {
            engine.configure_route_aggregation(network.clone(), mask.clone(), area_id.clone(), suppress, metric)?;
            
            console_log!(
                "Route aggregation configured on router {}: {}/{} (suppress: {}, area: {:?})",
                router_id, network, mask, suppress, area_id
            );
            
            // Log event
            let event = SimulationEvent {
                timestamp: self.simulation_time,
                event_type: SimulationEventType::RouteAggregationConfigured {
                    router_id,
                    network,
                    mask,
                    area_id,
                    suppress,
                },
                description: format!("Route aggregation configured on router {}", router_id),
            };
            self.event_manager.log_event(event);
            
            Ok(())
        } else {
            Err(format!("OSPF not enabled on router {}", router_id))
        }
    }
    
    /// Remove route aggregation from a router
    pub fn remove_route_aggregation(
        &mut self,
        router_id: u32,
        network: String,
        mask: String,
    ) -> Result<(), String> {
        if let Some(engine) = self.get_ospf_engine_mut(router_id) {
            if engine.remove_route_aggregation(&network, &mask) {
                console_log!(
                    "Route aggregation removed from router {}: {}/{}",
                    router_id, network, mask
                );
                
                // Log event
                let event = SimulationEvent {
                    timestamp: self.simulation_time,
                    event_type: SimulationEventType::RouteAggregationRemoved {
                        router_id,
                        network,
                        mask,
                    },
                    description: format!("Route aggregation removed from router {}", router_id),
                };
                self.event_manager.log_event(event);
                
                Ok(())
            } else {
                Err(format!("Route aggregation {}/{} not found on router {}", network, mask, router_id))
            }
        } else {
            Err(format!("OSPF not enabled on router {}", router_id))
        }
    }
    
    /// Get route aggregation statistics for all routers
    pub fn get_aggregation_statistics(&self) -> Vec<(u32, crate::route_aggregation::AggregationStatistics)> {
        let mut stats = Vec::new();
        
        for (router_id, engine) in &self.ospf_engines {
            let router_stats = engine.get_aggregation_statistics();
            if router_stats.total_aggregates > 0 {
                stats.push((*router_id, router_stats));
            }
        }
        
        stats
    }
    
    /// Get aggregation configuration for all routers
    pub fn get_aggregation_config(&self) -> Vec<(u32, Vec<(String, String, bool, Option<String>, bool)>)> {
        let mut configs = Vec::new();
        
        for (router_id, engine) in &self.ospf_engines {
            let router_config = engine.get_aggregation_config();
            if !router_config.is_empty() {
                configs.push((*router_id, router_config));
            }
        }
        
        configs
    }
    
    /// Update routing information for aggregation calculation
    pub fn update_aggregation_calculations(&mut self) {
        for (router_id, engine) in &mut self.ospf_engines {
            // Get current routing table
            if let Some(router) = self.topology.routers.get(router_id) {
                let mut routes = HashMap::new();
                for entry in &router.routing_table {
                    let route_key = format!("{}/{}", entry.destination, entry.netmask);
                    routes.insert(route_key, entry.metric);
                }
                
                engine.update_aggregation_routes(routes);
            }
        }
    }

    pub fn update_interface_config(&mut self, router_id: u32, interface_id: u32, config: crate::router::InterfaceConfig) -> Result<(), String> {
        if let Some(router) = self.topology.routers.get_mut(&router_id) {
            router.update_interface_config(interface_id, config)?;
            
            // OSPFエンジンのタイマー設定と認証設定も更新
            if let Some(ospf_engine) = self.ospf_engines.get_mut(&router_id) {
                if let Some(interface) = router.interfaces.get(&interface_id) {
                    // OSPFパラメータの更新（RFC 2328準拠）
                    let interface_config = crate::router::InterfaceConfig {
                        ip_address: Some(interface.ip_address.clone()),
                        netmask: Some(interface.netmask.clone()),
                        cost: Some(interface.cost),
                        hello_interval: Some(interface.hello_interval),
                        dead_interval: Some(interface.dead_interval),
                        priority: Some(interface.priority),
                        mtu: Some(interface.mtu),
                        enabled: Some(interface.enabled),
                        auth_type: Some(interface.auth_config.auth_type.clone()),
                        auth_key: interface.auth_config.auth_key.clone(),
                        auth_key_id: interface.auth_config.key_id,
                        inf_trans_delay: Some(interface.inf_trans_delay),
                        rxmt_interval: Some(interface.rxmt_interval),
                    };
                    ospf_engine.update_interface_config(interface_id, &interface_config);
                    
                    // 認証設定の更新
                    ospf_engine.update_interface_auth(interface_id, interface.auth_config.clone());
                }
            }
            
            // インターフェース設定変更イベントを記録
            self.event_manager.log_interface_config_changed(
                router_id, 
                interface_id
            );
            
            Ok(())
        } else {
            Err(format!("Router {} not found", router_id))
        }
    }

    /// ホストからping要求を送信
    pub fn send_ping_from_host(&mut self, host_id: u32, destination_ip: String) -> Result<u16, String> {
        // ホストが存在するか確認
        let host = self.topology.hosts.get(&host_id)
            .ok_or_else(|| format!("Host {} not found", host_id))?;
        
        if host.is_failed {
            return Err("Host is failed".to_string());
        }

        // ping要求を作成
        let (identifier, mut icmp_packet) = self.ping_manager.create_ping_request(
            host_id,
            destination_ip.clone(),
            self.simulation_time
        );
        
        // 送信元IPアドレスを設定
        icmp_packet.source_ip = host.ip_address.clone();

        // 次ホップを決定（同一サブネットならdestination_ip、そうでなければdefault_gateway）
        let _next_hop = host.get_next_hop(&destination_ip);
        
        // 接続されたルーターにパケットを送信
        if let Some(router_id) = host.connected_router_id {
            let packet_event = PacketEvent {
                timestamp: self.simulation_time,
                from_router_id: host_id,
                to_router_id: router_id,
                packet: ProtocolPacket::ICMP(icmp_packet),
            };
            
            self.protocol_engine.schedule_event(packet_event);
            console_log!("Ping sent from host {} to {} via router {}", 
                host_id, destination_ip, router_id);
            
            Ok(identifier)
        } else {
            Err("Host is not connected to any router".to_string())
        }
    }

    /// ICMPパケットの処理
    fn process_icmp_packet(&mut self, mut packet: ICMPPacket, from_id: u32, to_id: u32) {
        // TTLをデクリメント（ルーター間転送の場合）
        if self.topology.routers.contains_key(&to_id) && 
           packet.packet_type == ICMPType::EchoRequest {
            if let None = packet.decrement_ttl() {
                // TTLが0になった場合、Time Exceededメッセージを送信
                console_log!("ICMP packet TTL expired at router {}", to_id);
                
                let time_exceeded = ICMPPacket::new_time_exceeded(packet.clone());
                let error_event = PacketEvent {
                    timestamp: self.simulation_time + 0.001,
                    from_router_id: to_id,
                    to_router_id: from_id,
                    packet: ProtocolPacket::ICMP(time_exceeded),
                };
                
                self.protocol_engine.schedule_event(error_event);
                
                // 拡張pingマネージャーに通知
                if let Ok(_) = self.enhanced_ping_manager.process_icmp_error(
                    ICMPType::TimeExceeded,
                    packet.identifier,
                    packet.sequence_number,
                    self.simulation_time,
                ) {
                    console_log!("TTL expiry recorded in ping session");
                }
                
                return;
            }
        }
        match packet.packet_type {
            ICMPType::EchoRequest => {
                // 端末デバイスで受信した場合
                if let Ok(Some(reply)) = self.terminal_manager.process_icmp_packet(to_id, packet.clone(), self.simulation_time) {
                    // 端末がEcho Replyを生成した場合、ルーターに送信
                    let reply_event = PacketEvent {
                        timestamp: self.simulation_time + 0.001, // 1ms遅延
                        from_router_id: to_id,
                        to_router_id: from_id,
                        packet: ProtocolPacket::ICMP(reply),
                    };
                    
                    self.protocol_engine.schedule_event(reply_event);
                    console_log!("Terminal {} sending echo reply to {}", to_id, from_id);
                    return;
                }
                
                // ホストで受信した場合
                if let Some(host) = self.topology.hosts.get(&to_id) {
                    if !host.is_failed && host.ip_address == packet.destination_ip {
                        // 宛先が自分のIPアドレスの場合、Echo Replyを返す
                        let mut reply = ICMPPacket::new_echo_reply(
                            packet.identifier,
                            packet.sequence_number
                        );
                        reply.ttl = packet.ttl;  // TTLを保持（ホップ数計算用）
                        
                        let reply_event = PacketEvent {
                            timestamp: self.simulation_time + 0.001, // 1ms遅延
                            from_router_id: to_id,
                            to_router_id: from_id,
                            packet: ProtocolPacket::ICMP(reply),
                        };
                        
                        self.protocol_engine.schedule_event(reply_event);
                        console_log!("Host {} sending echo reply to {}", to_id, from_id);
                        return;
                    }
                }
                
                // ルーターで受信した場合
                if let Some(router) = self.topology.routers.get(&to_id) {
                    // 宛先IPがルーター自身のインターフェースの場合
                    for interface in router.interfaces.values() {
                        if interface.ip_address == packet.destination_ip {
                            let mut reply = ICMPPacket::new_echo_reply(
                                packet.identifier,
                                packet.sequence_number
                            );
                            reply.ttl = packet.ttl;  // TTLを保持（ホップ数計算用）
                            
                            let reply_event = PacketEvent {
                                timestamp: self.simulation_time + 0.001,
                                from_router_id: to_id,
                                to_router_id: from_id,
                                packet: ProtocolPacket::ICMP(reply),
                            };
                            
                            self.protocol_engine.schedule_event(reply_event);
                            console_log!("Router {} sending echo reply to {}", to_id, from_id);
                            return;
                        }
                    }
                    
                    // ルーティングテーブルで次ホップを検索
                    let next_hop_info = self.find_next_hop_for_icmp(to_id, &packet.destination_ip);
                    if let Some(next_hop_router_id) = next_hop_info {
                        // パケットを次ホップに転送
                        let forward_event = PacketEvent {
                            timestamp: self.simulation_time + 0.001,
                            from_router_id: to_id,
                            to_router_id: next_hop_router_id,
                            packet: ProtocolPacket::ICMP(packet.clone()),
                        };
                        
                        self.protocol_engine.schedule_event(forward_event);
                        console_log!("Router {} forwarding ICMP packet to {} via router {}", 
                            to_id, packet.destination_ip, next_hop_router_id);
                    } else {
                        console_log!("Router {} has no route to {}", to_id, packet.destination_ip);
                    }
                }
            },
            ICMPType::EchoReply => {
                // 端末デバイスで受信した場合
                if let Ok(_) = self.terminal_manager.process_icmp_packet(to_id, packet.clone(), self.simulation_time) {
                    console_log!("Terminal {} received echo reply", to_id);
                    return;
                }
                
                // ホストで受信した場合
                if self.topology.hosts.contains_key(&to_id) {
                    self.ping_manager.process_echo_reply(packet.identifier, self.simulation_time);
                    
                    // 拡張pingマネージャーにも通知
                    if let Ok(_) = self.enhanced_ping_manager.process_echo_reply(
                        packet.identifier,
                        packet.sequence_number,
                        packet.ttl,
                        self.simulation_time,
                    ) {
                        console_log!("Echo reply recorded in enhanced ping session");
                    }
                    
                    return;
                }
                
                // ルーターで受信した場合は転送
                if self.topology.routers.contains_key(&to_id) {
                    let next_hop_info = self.find_next_hop_for_icmp(to_id, &packet.source_ip);
                    if let Some(next_hop_router_id) = next_hop_info {
                        let forward_event = PacketEvent {
                            timestamp: self.simulation_time + 0.001,
                            from_router_id: to_id,
                            to_router_id: next_hop_router_id,
                            packet: ProtocolPacket::ICMP(packet.clone()),
                        };
                        
                        self.protocol_engine.schedule_event(forward_event);
                        console_log!("Router {} forwarding ICMP reply to {} via router {}", 
                            to_id, packet.source_ip, next_hop_router_id);
                    }
                }
            },
            _ => {
                console_log!("Unhandled ICMP packet type: {:?}", packet.packet_type);
            }
        }
    }

    /// 最近のping結果を取得
    pub fn get_recent_ping_results(&self, count: usize) -> Vec<PingResult> {
        self.ping_manager.get_recent_results(count)
    }
    

    /// ICMPパケットのための次ホップを検索
    fn find_next_hop_for_icmp(&self, router_id: u32, destination_ip: &str) -> Option<u32> {
        let router = self.topology.routers.get(&router_id)?;
        
        // ルーティングテーブルから最適なルートを検索
        let mut best_match: Option<(&RoutingTableEntry, u32)> = None;
        
        for entry in &router.routing_table {
            if self.is_ip_in_network(destination_ip, &entry.destination, &entry.netmask) {
                // プレフィックス長を計算（ネットマスクのビット数）
                let prefix_len = self.get_prefix_length(&entry.netmask);
                
                match best_match {
                    None => best_match = Some((entry, prefix_len)),
                    Some((_, best_len)) => {
                        if prefix_len > best_len {
                            best_match = Some((entry, prefix_len));
                        }
                    }
                }
            }
        }
        
        if let Some((best_entry, _)) = best_match {
            // next_hopがルーターIDの形式かチェック
            if let Ok(next_router_id) = best_entry.next_hop.parse::<u32>() {
                return Some(next_router_id);
            }
            
            // next_hopがIPアドレスの場合、そのIPを持つルーターを検索
            for (rid, r) in &self.topology.routers {
                for interface in r.interfaces.values() {
                    if interface.ip_address == best_entry.next_hop {
                        return Some(*rid);
                    }
                }
            }
            
            // ホストの場合もチェック
            for (_hid, h) in &self.topology.hosts {
                if h.ip_address == destination_ip {
                    if let Some(connected_router) = h.connected_router_id {
                        return Some(connected_router);
                    }
                }
            }
        }
        
        None
    }

    /// IPアドレスが特定のネットワークに属するかチェック
    fn is_ip_in_network(&self, ip: &str, network: &str, netmask: &str) -> bool {
        let ip_parts: Vec<u8> = ip.split('.').filter_map(|s| s.parse().ok()).collect();
        let net_parts: Vec<u8> = network.split('.').filter_map(|s| s.parse().ok()).collect();
        let mask_parts: Vec<u8> = netmask.split('.').filter_map(|s| s.parse().ok()).collect();
        
        if ip_parts.len() != 4 || net_parts.len() != 4 || mask_parts.len() != 4 {
            return false;
        }
        
        for i in 0..4 {
            if (ip_parts[i] & mask_parts[i]) != (net_parts[i] & mask_parts[i]) {
                return false;
            }
        }
        
        true
    }

    /// ネットマスクのプレフィックス長を取得
    fn get_prefix_length(&self, netmask: &str) -> u32 {
        let mask_parts: Vec<u8> = netmask.split('.').filter_map(|s| s.parse().ok()).collect();
        if mask_parts.len() != 4 {
            return 0;
        }
        
        let mut prefix_len = 0;
        for part in mask_parts {
            prefix_len += part.count_ones();
        }
        
        prefix_len
    }
    
    // ==========================================
    // Terminal Device Management Methods
    // ==========================================
    
    /// 新しい端末デバイスを追加
    pub fn add_terminal(
        &mut self,
        name: String,
        ip_address: String,
        netmask: String,
        default_gateway: String,
    ) -> Result<u32, String> {
        let terminal_id = self.terminal_manager.add_terminal(
            name.clone(),
            ip_address.clone(),
            netmask,
            default_gateway,
        )?;
        
        // イベントログに記録
        self.event_manager.log_event(crate::event_manager::SimulationEvent {
            timestamp: self.simulation_time,
            event_type: crate::event_manager::SimulationEventType::RouterAdded {
                router_id: terminal_id,
                name: format!("Terminal: {}", name),
            },
            description: format!("Terminal device '{}' added with IP {}", name, ip_address),
        });
        
        console_log!("Terminal device {} added with ID {} (IP: {})", name, terminal_id, ip_address);
        Ok(terminal_id)
    }
    
    /// 端末デバイスを削除
    pub fn remove_terminal(&mut self, terminal_id: u32) -> Result<(), String> {
        self.terminal_manager.remove_terminal(terminal_id)?;
        
        console_log!("Terminal device {} removed", terminal_id);
        Ok(())
    }
    
    /// 端末をルーターに接続
    pub fn connect_terminal_to_router(
        &mut self,
        terminal_id: u32,
        router_id: u32,
    ) -> Result<(), String> {
        // ルーターが存在するかチェック
        if !self.topology.routers.contains_key(&router_id) {
            return Err(format!("Router {} not found", router_id));
        }
        
        // 適当なインターフェースIDを割り当て
        let interface_id = self.topology.get_next_interface_id();
        
        self.terminal_manager.connect_terminal_to_router(
            terminal_id,
            router_id,
            interface_id,
        )?;
        
        console_log!(
            "Terminal {} connected to router {} (interface {})",
            terminal_id, router_id, interface_id
        );
        
        Ok(())
    }
    
    /// 端末をルーターから切断
    pub fn disconnect_terminal(&mut self, terminal_id: u32) -> Result<(), String> {
        self.terminal_manager.disconnect_terminal(terminal_id)
    }
    
    /// 端末からpingを送信
    pub fn send_ping_from_terminal(
        &mut self,
        terminal_id: u32,
        destination_ip: String,
    ) -> Result<u16, String> {
        let identifier = self.terminal_manager.send_ping_from_terminal(
            terminal_id,
            destination_ip.clone(),
            self.simulation_time,
        )?;
        
        console_log!(
            "Ping sent from terminal {} to {} (ID: {})",
            terminal_id, destination_ip, identifier
        );
        
        Ok(identifier)
    }
    
    /// 端末でICMPパケットを処理
    pub fn process_terminal_icmp_packet(
        &mut self,
        terminal_id: u32,
        packet: ICMPPacket,
    ) -> Result<Option<ICMPPacket>, String> {
        self.terminal_manager.process_icmp_packet(
            terminal_id,
            packet,
            self.simulation_time,
        )
    }
    
    /// 端末の障害状態を設定
    pub fn set_terminal_failed(&mut self, terminal_id: u32, failed: bool) -> Result<(), String> {
        self.terminal_manager.set_terminal_failed(terminal_id, failed)?;
        
        let action = if failed { "failed" } else { "recovered" };
        console_log!("Terminal {} {}", terminal_id, action);
        
        Ok(())
    }
    
    /// 端末の設定を更新
    pub fn update_terminal_config(
        &mut self,
        terminal_id: u32,
        config: TerminalConfig,
    ) -> Result<(), String> {
        self.terminal_manager.update_terminal_config(terminal_id, config)
    }
    
    /// 端末にARPエントリを追加
    pub fn add_terminal_arp_entry(
        &mut self,
        terminal_id: u32,
        ip: String,
        mac: String,
    ) -> Result<(), String> {
        self.terminal_manager.add_arp_entry_to_terminal(terminal_id, ip, mac)
    }
    
    /// 端末にルートエントリを追加
    pub fn add_terminal_route(
        &mut self,
        terminal_id: u32,
        destination: String,
        netmask: String,
        gateway: String,
        metric: u32,
    ) -> Result<(), String> {
        self.terminal_manager.add_route_to_terminal(
            terminal_id,
            destination,
            netmask,
            gateway,
            metric,
            self.simulation_time,
        )
    }
    
    /// 端末デバイス情報を取得
    pub fn get_terminal_info(&self, terminal_id: u32) -> Result<TerminalDeviceInfo, String> {
        self.terminal_manager.get_terminal_info(terminal_id)
    }
    
    /// すべての端末の情報を取得
    pub fn get_all_terminals_info(&self) -> Vec<TerminalDeviceInfo> {
        self.terminal_manager.get_all_terminals_info()
    }
    
    /// 指定IPアドレスを持つ端末を検索
    pub fn find_terminal_by_ip(&self, ip_address: &str) -> Option<u32> {
        self.terminal_manager.find_terminal_by_ip(ip_address)
    }
    
    /// 端末マネージャーの統計情報を取得
    pub fn get_terminal_manager_statistics(&self) -> &crate::terminal_manager::ManagerStatistics {
        self.terminal_manager.get_statistics()
    }
    
    /// 端末マネージャーの設定を更新
    pub fn update_terminal_manager_config(&mut self, config: ManagerConfig) {
        self.terminal_manager.update_config(config);
    }
    
    /// すべての端末の送信待ちパケットを処理（シミュレーションステップで呼び出される）
    fn process_terminal_packet_queues(&mut self) {
        let packets = self.terminal_manager.process_all_packet_queues(self.simulation_time);
        
        for (terminal_id, packet, router_id) in packets {
            // 端末からルーターへのパケット配信をスケジュール
            let packet_event = PacketEvent {
                timestamp: self.simulation_time + 0.001, // 1ms後に配信
                from_router_id: terminal_id,
                to_router_id: router_id,
                packet: ProtocolPacket::ICMP(packet),
            };
            
            self.protocol_engine.schedule_event(packet_event);
        }
    }
    
    /// 端末マネージャーの統計を更新（シミュレーションステップで呼び出される）
    fn update_terminal_statistics(&mut self) {
        self.terminal_manager.update_statistics(self.simulation_time);
    }
    
    // ==========================================
    // Enhanced Ping Management Methods
    // ==========================================
    
    /// 拡張pingセッションを開始
    pub fn start_enhanced_ping(
        &mut self,
        source_id: u32,
        source_ip: String,
        destination_ip: String,
        config: PingSessionConfig,
    ) -> Result<u32, String> {
        let session_id = self.enhanced_ping_manager.start_ping_session(
            source_id,
            source_ip.clone(),
            destination_ip.clone(),
            config,
            self.simulation_time,
        )?;
        
        console_log!(
            "Started enhanced ping session {} from {} to {}",
            session_id, source_ip, destination_ip
        );
        
        Ok(session_id)
    }
    
    /// 次のpingを生成して送信
    pub fn send_next_ping(&mut self, session_id: u32) -> Result<bool, String> {
        if let Some(packet) = self.enhanced_ping_manager.generate_next_ping(
            session_id,
            self.simulation_time,
        )? {
            // パケットを送信元から送信
            if let Some(session) = self.enhanced_ping_manager.get_session_info(session_id) {
                let packet_event = PacketEvent {
                    timestamp: self.simulation_time + 0.001,
                    from_router_id: session.source_id,
                    to_router_id: session.source_id,  // 最初は自分自身から開始
                    packet: ProtocolPacket::ICMP(packet),
                };
                
                self.protocol_engine.schedule_event(packet_event);
                Ok(true)
            } else {
                Err("Session not found".to_string())
            }
        } else {
            Ok(false)  // これ以上送信するパケットがない
        }
    }
    
    /// pingセッションを停止
    pub fn stop_ping_session(&mut self, session_id: u32) -> Result<PingSessionSummary, String> {
        let summary = self.enhanced_ping_manager.stop_session(session_id)?;
        
        console_log!(
            "Ping session {} stopped: {} sent, {} received, {:.1}% loss",
            session_id, summary.packets_sent, summary.packets_received, summary.loss_percentage
        );
        
        Ok(summary)
    }
    
    /// アクティブなpingセッションを取得
    pub fn get_active_ping_sessions(&self) -> Vec<u32> {
        self.enhanced_ping_manager.get_active_sessions()
            .iter()
            .map(|session| session.session_id)
            .collect()
    }
    
    /// pingセッションの詳細情報を取得
    pub fn get_ping_session_details(&self, session_id: u32) -> Option<serde_json::Value> {
        self.enhanced_ping_manager.get_session_info(session_id)
            .and_then(|session| serde_json::to_value(session).ok())
    }
    
    /// ping統計情報を取得
    pub fn get_ping_statistics(&self) -> serde_json::Value {
        serde_json::to_value(self.enhanced_ping_manager.get_global_statistics())
            .unwrap_or(serde_json::Value::Null)
    }
    
    /// Traceroute機能を開始
    pub fn start_traceroute(
        &mut self,
        source_id: u32,
        source_ip: String,
        destination_ip: String,
        max_hops: u8,
    ) -> u32 {
        let session_id = self.enhanced_ping_manager.start_traceroute(
            source_id,
            source_ip.clone(),
            destination_ip.clone(),
            max_hops,
            3,  // probes per hop
            3.0,  // timeout seconds
        );
        
        console_log!(
            "Started traceroute session {} from {} to {} (max {} hops)",
            session_id, source_ip, destination_ip, max_hops
        );
        
        session_id
    }
    
    /// シミュレーションステップでpingタイムアウトをチェック
    fn check_ping_timeouts(&mut self) {
        self.enhanced_ping_manager.check_timeouts(self.simulation_time);
    }
    
    // ==========================================
    // NBMA Network Support Methods
    // ==========================================
    
    /// Configure an interface as NBMA
    pub fn configure_nbma_interface(
        &mut self,
        router_id: u32,
        interface_id: u32,
        network_type: String,
        hello_interval: u32,
        dead_interval: u32,
        priority: u8,
    ) -> Result<(), String> {
        use crate::network_type::OSPFNetworkType;
        use crate::nbma_support::NBMAInterfaceConfig;
        
        // Parse network type
        let net_type = match network_type.as_str() {
            "NBMA" => OSPFNetworkType::NBMA,
            "Point-to-Multipoint" => OSPFNetworkType::PointToMultipoint,
            _ => return Err("Invalid network type for NBMA configuration".to_string()),
        };
        
        // Create NBMA configuration
        let config = NBMAInterfaceConfig {
            network_type: net_type,
            static_neighbors: Vec::new(),
            hello_interval,
            dead_interval,
            priority,
        };
        
        // Configure in OSPF engine
        if let Some(engine) = self.ospf_engines.get_mut(&router_id) {
            engine.configure_nbma_interface(interface_id, config)
        } else {
            Err("Router not found or OSPF not enabled".to_string())
        }
    }
    
    /// Add a static neighbor for NBMA interface
    pub fn add_nbma_neighbor(
        &mut self,
        router_id: u32,
        interface_id: u32,
        neighbor_ip: String,
        priority: u8,
        poll_interval: u32,
    ) -> Result<(), String> {
        use crate::nbma_support::NBMANeighborConfig;
        
        let neighbor = NBMANeighborConfig {
            neighbor_ip,
            priority,
            poll_interval,
            enabled: true,
        };
        
        if let Some(engine) = self.ospf_engines.get_mut(&router_id) {
            engine.add_nbma_neighbor(interface_id, neighbor)
        } else {
            Err("Router not found or OSPF not enabled".to_string())
        }
    }
    
    /// Remove a static neighbor from NBMA interface
    pub fn remove_nbma_neighbor(
        &mut self,
        router_id: u32,
        interface_id: u32,
        neighbor_ip: String,
    ) -> Result<(), String> {
        if let Some(engine) = self.ospf_engines.get_mut(&router_id) {
            engine.remove_nbma_neighbor(interface_id, &neighbor_ip)
        } else {
            Err("Router not found or OSPF not enabled".to_string())
        }
    }
    
    /// Get NBMA configuration for a router interface
    pub fn get_nbma_config(&self, router_id: u32, interface_id: u32) -> String {
        if let Some(engine) = self.ospf_engines.get(&router_id) {
            if let Some(config) = engine.get_nbma_config(interface_id) {
                serde_json::to_string(config).unwrap_or_default()
            } else {
                "{}".to_string()
            }
        } else {
            "{}".to_string()
        }
    }
    
    /// Get NBMA statistics
    pub fn get_nbma_statistics(&self) -> String {
        let mut total_stats = crate::nbma_support::NBMAStatistics {
            total_interfaces: 0,
            total_static_neighbors: 0,
            active_neighbors: 0,
            nbma_interfaces: 0,
            p2mp_interfaces: 0,
        };
        
        for engine in self.ospf_engines.values() {
            let stats = engine.get_nbma_statistics();
            total_stats.total_interfaces += stats.total_interfaces;
            total_stats.total_static_neighbors += stats.total_static_neighbors;
            total_stats.active_neighbors += stats.active_neighbors;
            total_stats.nbma_interfaces += stats.nbma_interfaces;
            total_stats.p2mp_interfaces += stats.p2mp_interfaces;
        }
        
        serde_json::to_string(&total_stats).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulation_creation() {
        let sim = NetworkSimulation::new();
        assert_eq!(sim.simulation_time, 0.0);
        assert!(!sim.running);
        assert_eq!(sim.topology.routers.len(), 0);
        assert_eq!(sim.ospf_engines.len(), 0);
    }
    
    #[test]
    fn test_router_management() {
        let mut sim = NetworkSimulation::new();
        
        // Add router
        let router_id = sim.add_router("TestRouter".to_string(), 100.0, 100.0);
        assert_eq!(sim.topology.routers.len(), 1);
        
        // Enable OSPF
        assert!(sim.enable_ospf(router_id).is_ok());
        assert_eq!(sim.ospf_engines.len(), 1);
        
        // Delete router
        assert!(sim.delete_router(router_id));
        assert_eq!(sim.topology.routers.len(), 0);
        assert_eq!(sim.ospf_engines.len(), 0);
    }
}
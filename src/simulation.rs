use std::collections::{HashMap, BTreeMap};
use crate::network::NetworkTopology;
use crate::protocol::{ProtocolEngine, PacketEvent, ProtocolPacket};
use crate::ospf::{OSPFPacket, OSPFPacketType, OSPFPacketData, HelloPacket};
use crate::ospf_engine::OSPFEngine;
use crate::event_manager::{EventManager, SimulationEvent};
use crate::failure_manager::FailureManager;
use crate::route_calculator::RouteCalculator;
use crate::router::{OSPFNeighborState, RoutingTableEntry};
use crate::ping_manager::{PingManager, PingResult};
use crate::device::{ICMPPacket, ICMPType};
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
            } else if link.router2_id == router_id {
                ospf_engine.add_router_link(link.router1_id, link.router2_interface_id, link.cost);
                // Initialize DR election for this interface
                ospf_engine.initialize_interface_dr_election(link.router2_interface_id, link.network_type.clone(), 1);
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
                options: 0,
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
        let interface_id = if matches!(&packet.data, OSPFPacketData::Hello(_)) {
            self.get_interface_id(from_router_id, to_router_id)
        } else {
            0
        };
        
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
                        engine.process_lsu_packet(lsu, from_router_id)
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

    pub fn update_interface_config(&mut self, router_id: u32, interface_id: u32, config: crate::router::InterfaceConfig) -> Result<(), String> {
        if let Some(router) = self.topology.routers.get_mut(&router_id) {
            router.update_interface_config(interface_id, config)?;
            
            // OSPFエンジンのタイマー設定も更新
            if let Some(ospf_engine) = self.ospf_engines.get_mut(&router_id) {
                if let Some(interface) = router.interfaces.get(&interface_id) {
                    // OSPFタイマーの更新（hello_interval, dead_intervalなど）
                    ospf_engine.update_interface_timers(interface_id, interface.hello_interval, interface.dead_interval);
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
    fn process_icmp_packet(&mut self, packet: ICMPPacket, from_id: u32, to_id: u32) {
        match packet.packet_type {
            ICMPType::EchoRequest => {
                // ホストで受信した場合
                if let Some(host) = self.topology.hosts.get(&to_id) {
                    if !host.is_failed && host.ip_address == packet.destination_ip {
                        // 宛先が自分のIPアドレスの場合、Echo Replyを返す
                        let reply = ICMPPacket::new_echo_reply(
                            packet.identifier,
                            packet.sequence_number
                        );
                        
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
                            let reply = ICMPPacket::new_echo_reply(
                                packet.identifier,
                                packet.sequence_number
                            );
                            
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
                // ホストで受信した場合
                if self.topology.hosts.contains_key(&to_id) {
                    self.ping_manager.process_echo_reply(packet.identifier, self.simulation_time);
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
            for (hid, h) in &self.topology.hosts {
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
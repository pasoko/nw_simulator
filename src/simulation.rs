use std::collections::HashMap;
use crate::network::NetworkTopology;
use crate::protocol::{ProtocolEngine, PacketEvent, ProtocolPacket};
use crate::ospf::{OSPFPacket, OSPFPacketType, OSPFPacketData, HelloPacket};
use crate::ospf_engine::OSPFEngine;
use crate::event_manager::{EventManager, SimulationEvent};
use crate::failure_manager::FailureManager;
use crate::route_calculator::RouteCalculator;
use crate::router::OSPFNeighborState;
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
    ospf_engines: HashMap<u32, OSPFEngine>,
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
            ospf_engines: HashMap::new(),
        }
    }

    pub fn add_router(&mut self, name: String, _x: f64, _y: f64) -> u32 {
        let router_id = self.topology.add_router(name.clone());
        self.event_manager.log_router_added(router_id, name);
        router_id
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
                engine1.remove_link(router2_id);
                if engine1.get_neighbor_count() > 0 {
                    let events = engine1.regenerate_router_lsa();
                    console_log!("Router {} regenerating LSA after link failure", router1_id);
                    lsa_events.extend(events);
                }
            }
            
            if let Some(engine2) = self.ospf_engines.get_mut(&router2_id) {
                if engine2.remove_neighbor(router1_id) {
                    self.event_manager.log_neighbor_state_changed(
                        router2_id, router1_id, "Active".to_string(), "Down".to_string()
                    );
                }
                // Remove link and regenerate LSA
                engine2.remove_link(router1_id);
                if engine2.get_neighbor_count() > 0 {
                    let events = engine2.regenerate_router_lsa();
                    console_log!("Router {} regenerating LSA after link failure", router2_id);
                    lsa_events.extend(events);
                }
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
            
            // Recalculate routes for affected routers
            self.route_calculator.calculate_routes_for_router(
                router1_id, &mut self.topology, &self.ospf_engines, &mut self.event_manager
            );
            self.route_calculator.calculate_routes_for_router(
                router2_id, &mut self.topology, &self.ospf_engines, &mut self.event_manager
            );
            
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
        
        // Add router links to OSPF engine
        for link in self.topology.links.values() {
            if link.router1_id == router_id {
                ospf_engine.add_router_link(link.router2_id, link.router1_interface_id, link.cost);
            } else if link.router2_id == router_id {
                ospf_engine.add_router_link(link.router1_id, link.router2_interface_id, link.cost);
            }
        }
        
        // Don't generate LSA here - wait until neighbors are discovered
        // LSAs should only be generated when we have active neighbors to flood to
        console_log!("Router {} OSPF enabled with {} configured links, LSA generation deferred until neighbors discovered", 
            router_id, ospf_engine.get_router_links().len());
        
        self.ospf_engines.insert(router_id, ospf_engine);
        self.event_manager.log_ospf_enabled(router_id);
        
        // Calculate initial routes after OSPF is enabled
        console_log!("OSPF enabled on router {}, calculating initial routes", router_id);
        self.route_calculator.calculate_routes_for_router(
            router_id, &mut self.topology, &self.ospf_engines, &mut self.event_manager
        );
        
        // OSPF engine will manage Hello timers internally
        
        Ok(())
    }

    pub fn start_simulation(&mut self) {
        self.running = true;
        self.simulation_time = 0.0;
        
        let router_ids: Vec<u32> = self.topology.routers
            .iter()
            .filter(|(_, router)| router.ospf_state.is_some())
            .map(|(id, _)| *id)
            .collect();
        
        console_log!("Starting simulation with {} OSPF-enabled routers", router_ids.len());
        
        // No need to schedule initial hello packets - OSPF engines will handle this
        console_log!("OSPF engines will manage Hello timers internally");
    }
    
    pub fn stop_simulation(&mut self) {
        self.running = false;
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
        for (_router_id, engine) in self.ospf_engines.iter_mut() {
            let events = engine.update_time(target_time);
            if !events.is_empty() {
                console_log!("Router {} generated {} events from timer processing", _router_id, events.len());
            }
            ospf_events.extend(events);
        }
        
        // Schedule OSPF timer events
        for mut event in ospf_events {
            event.timestamp = self.simulation_time + 0.1; // Small delay for packet processing
            self.protocol_engine.schedule_event(event);
        }
    }
    
    pub fn toggle_link_failure(&mut self, from_id: u32, to_id: u32) -> bool {
        let (success, events) = self.failure_manager.toggle_link_failure(
            from_id, to_id, &mut self.topology, &mut self.ospf_engines, &mut self.event_manager
        );
        
        // Schedule flooding events
        for mut event in events {
            event.timestamp = self.simulation_time + 0.1;
            self.protocol_engine.schedule_event(event);
        }
        
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
                
                self.process_ospf_packet(ospf_packet.clone(), event.from_router_id, event.to_router_id);
            }
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
                format!("Database Description - MTU: {}, Flags: {:#04x}, Seq: {}, LSA headers: {}",
                    dd.interface_mtu,
                    dd.flags,
                    dd.dd_sequence_number,
                    dd.lsa_headers.len()
                )
            },
            OSPFPacketData::LinkStateRequest(lsr) => {
                format!("Link State Request - Requesting {} LSAs", lsr.requests.len())
            },
            OSPFPacketData::LinkStateUpdate(lsu) => {
                format!("Link State Update - Contains {} LSAs", lsu.lsas.len())
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
            auth_type: 0,
            authentication: 0,
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
                let lsa_database_changed = lsa_count != lsa_count_before;
                let state_transitions = engine.get_neighbor_state_transitions();
                
                (new_events, lsa_updated, lsa_count, lsa_database_changed, state_transitions)
            } else {
                return;
            };
        
        // Schedule response packets
        for mut event in new_events {
            event.timestamp = self.simulation_time + 0.1;
            self.protocol_engine.schedule_event(event);
        }
        
        // Trigger route calculation only for LinkStateUpdate packets
        // Don't calculate routes during DD exchange
        if matches!(&packet.data, OSPFPacketData::LinkStateUpdate(_)) && lsa_database_changed && lsa_count > 0 {
            console_log!("Router {} LSA database changed due to LSU, running SPF calculation", to_router_id);
            self.route_calculator.calculate_routes_for_router(
                to_router_id, &mut self.topology, &self.ospf_engines, &mut self.event_manager
            );
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
            
            // Recalculate routes when adjacency is established or lost
            match new_state {
                OSPFNeighborState::Full => {
                    self.route_calculator.calculate_routes_for_router(
                        to_router_id, &mut self.topology, &self.ospf_engines, &mut self.event_manager
                    );
                    self.route_calculator.calculate_routes_for_router(
                        from_router_id, &mut self.topology, &self.ospf_engines, &mut self.event_manager
                    );
                    
                    // Trigger route calculation for all other OSPF routers
                    let ospf_routers: Vec<u32> = self.ospf_engines.keys().cloned().collect();
                    for router_id in ospf_routers {
                        if router_id != to_router_id && router_id != from_router_id {
                            self.route_calculator.calculate_routes_for_router(
                                router_id, &mut self.topology, &self.ospf_engines, &mut self.event_manager
                            );
                        }
                    }
                }
                OSPFNeighborState::Down => {
                    self.route_calculator.calculate_routes_for_router(
                        to_router_id, &mut self.topology, &self.ospf_engines, &mut self.event_manager
                    );
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
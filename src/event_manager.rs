use serde::{Serialize, Deserialize};
use crate::console_log;

/// Simulation Event Types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationEvent {
    pub timestamp: f64,
    pub event_type: SimulationEventType,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SimulationEventType {
    RouterAdded { router_id: u32, name: String },
    LinkCreated { from_router: u32, to_router: u32, cost: u32 },
    OSPFEnabled { router_id: u32 },
    PacketSent { from_router: u32, to_router: u32, packet_type: String },
    PacketReceived { router_id: u32, packet_type: String },
    RoutingTableUpdated { router_id: u32 },
    NeighborStateChanged { router_id: u32, neighbor_id: u32, new_state: String },
    LinkFailure { from_router: u32, to_router: u32 },
    LinkRecovery { from_router: u32, to_router: u32 },
    RouterFailure { router_id: u32 },
    RouterRecovery { router_id: u32 },
    PacketDiscarded { router_id: u32, from_router: u32, reason: String },
    LSARegenerated { router_id: u32, lsa_count: usize },
    SPFCalculationStarted { router_id: u32 },
    SPFCalculationCompleted { router_id: u32, route_count: usize },
    NeighborDeadTimerExpired { router_id: u32, neighbor_id: u32 },
    InterfaceConfigChanged { router_id: u32, interface_id: u32 },
    StubAreaConfigured { router_id: u32, area_type: String },
    VirtualLinkConfigured { local_router_id: u32, remote_router_id: u32, transit_area_id: String, interface_id: u32 },
    VirtualLinkRemoved { local_router_id: u32, remote_router_id: u32 },
    VirtualLinkStateChanged { local_router_id: u32, remote_router_id: u32, old_state: String, new_state: String },
}

impl SimulationEvent {
    /// Create a new stub area configured event
    pub fn stub_area_configured(timestamp: f64, router_id: u32, area_type: String) -> Self {
        SimulationEvent {
            timestamp,
            event_type: SimulationEventType::StubAreaConfigured { router_id, area_type: area_type.clone() },
            description: format!("Router {} configured as stub area type: {}", router_id, area_type),
        }
    }
}

/// Event Management System
/// 
/// Handles all simulation event logging, storage, and retrieval.
/// Provides a centralized way to track what happens during simulation.
pub struct EventManager {
    simulation_log: Vec<SimulationEvent>,
    current_time: f64,
    max_log_size: usize,
}

impl EventManager {
    pub fn new() -> Self {
        EventManager {
            simulation_log: Vec::new(),
            current_time: 0.0,
            max_log_size: 10000, // Prevent memory issues
        }
    }
    
    pub fn update_time(&mut self, time: f64) {
        self.current_time = time;
    }
    
    pub fn log_event(&mut self, event: SimulationEvent) {
        console_log!("[{:.2}s] {}", event.timestamp, event.description);
        
        self.simulation_log.push(event);
        
        // Prevent memory issues by limiting log size
        if self.simulation_log.len() > self.max_log_size {
            let remove_count = self.max_log_size / 4; // Remove 25% of old entries
            self.simulation_log.drain(0..remove_count);
            console_log!("Event log trimmed - removed {} old entries", remove_count);
        }
    }
    
    pub fn log_router_added(&mut self, router_id: u32, name: String) {
        self.log_event(SimulationEvent {
            timestamp: self.current_time,
            event_type: SimulationEventType::RouterAdded { 
                router_id, 
                name: name.clone() 
            },
            description: format!("Router '{}' added with ID {}", name, router_id),
        });
    }
    
    pub fn log_link_created(&mut self, from_router: u32, to_router: u32, cost: u32) {
        self.log_event(SimulationEvent {
            timestamp: self.current_time,
            event_type: SimulationEventType::LinkCreated { 
                from_router, 
                to_router, 
                cost 
            },
            description: format!("Link created between routers {} and {} with cost {}", 
                from_router, to_router, cost),
        });
    }
    
    pub fn log_ospf_enabled(&mut self, router_id: u32) {
        self.log_event(SimulationEvent {
            timestamp: self.current_time,
            event_type: SimulationEventType::OSPFEnabled { router_id },
            description: format!("OSPF enabled on router {} at simulation time {}", 
                router_id, self.current_time),
        });
    }
    
    pub fn log_packet_sent(&mut self, from_router: u32, to_router: u32, packet_type: String) {
        self.log_event(SimulationEvent {
            timestamp: self.current_time,
            event_type: SimulationEventType::PacketSent {
                from_router,
                to_router,
                packet_type: packet_type.clone(),
            },
            description: format!("OSPF {} packet sent from router {} to router {}", 
                packet_type, from_router, to_router),
        });
    }
    
    pub fn log_packet_received(&mut self, router_id: u32, packet_type: String, details: String) {
        self.log_event(SimulationEvent {
            timestamp: self.current_time,
            event_type: SimulationEventType::PacketReceived {
                router_id,
                packet_type: packet_type.clone(),
            },
            description: format!("Router {} received OSPF {} - {}", 
                router_id, packet_type, details),
        });
    }
    
    pub fn log_routing_table_updated(&mut self, router_id: u32, details: String) {
        self.log_event(SimulationEvent {
            timestamp: self.current_time,
            event_type: SimulationEventType::RoutingTableUpdated { router_id },
            description: details,
        });
    }
    
    pub fn log_neighbor_state_changed(&mut self, router_id: u32, neighbor_id: u32, 
        old_state: String, new_state: String) {
        self.log_event(SimulationEvent {
            timestamp: self.current_time,
            event_type: SimulationEventType::NeighborStateChanged {
                router_id,
                neighbor_id,
                new_state: new_state.clone(),
            },
            description: format!("Router {} neighbor {} state changed: {} → {}", 
                router_id, neighbor_id, old_state, new_state),
        });
    }
    
    pub fn log_link_failure(&mut self, from_router: u32, to_router: u32) {
        self.log_event(SimulationEvent {
            timestamp: self.current_time,
            event_type: SimulationEventType::LinkFailure { from_router, to_router },
            description: format!("Link Failure: Link between Router {} and Router {}", 
                from_router, to_router),
        });
    }
    
    pub fn log_link_recovery(&mut self, from_router: u32, to_router: u32) {
        self.log_event(SimulationEvent {
            timestamp: self.current_time,
            event_type: SimulationEventType::LinkRecovery { from_router, to_router },
            description: format!("Link Recovery: Link between Router {} and Router {}", 
                from_router, to_router),
        });
    }
    
    pub fn log_router_failure(&mut self, router_id: u32, router_name: String) {
        self.log_event(SimulationEvent {
            timestamp: self.current_time,
            event_type: SimulationEventType::RouterFailure { router_id },
            description: format!("Router Failure: Router {} ({})", router_id, router_name),
        });
    }
    
    pub fn log_router_recovery(&mut self, router_id: u32, router_name: String) {
        self.log_event(SimulationEvent {
            timestamp: self.current_time,
            event_type: SimulationEventType::RouterRecovery { router_id },
            description: format!("Router Recovery: Router {} ({})", router_id, router_name),
        });
    }
    
    pub fn log_packet_discarded(&mut self, router_id: u32, from_router: u32, reason: String) {
        self.log_event(SimulationEvent {
            timestamp: self.current_time,
            event_type: SimulationEventType::PacketDiscarded { router_id, from_router, reason: reason.clone() },
            description: format!("Packet Discarded: Router {} dropped packet from Router {} - {}", 
                router_id, from_router, reason),
        });
    }
    
    pub fn log_lsa_regenerated(&mut self, router_id: u32, lsa_count: usize) {
        self.log_event(SimulationEvent {
            timestamp: self.current_time,
            event_type: SimulationEventType::LSARegenerated { router_id, lsa_count },
            description: format!("LSA Regenerated: Router {} regenerated LSA (total {} LSAs in database)", 
                router_id, lsa_count),
        });
    }
    
    pub fn log_spf_calculation_started(&mut self, router_id: u32) {
        self.log_event(SimulationEvent {
            timestamp: self.current_time,
            event_type: SimulationEventType::SPFCalculationStarted { router_id },
            description: format!("SPF Calculation Started: Router {} starting SPF calculation", 
                router_id),
        });
    }
    
    pub fn log_spf_calculation_completed(&mut self, router_id: u32, route_count: usize) {
        self.log_event(SimulationEvent {
            timestamp: self.current_time,
            event_type: SimulationEventType::SPFCalculationCompleted { router_id, route_count },
            description: format!("SPF Calculation Completed: Router {} calculated {} routes", 
                router_id, route_count),
        });
    }
    
    pub fn log_neighbor_dead_timer_expired(&mut self, router_id: u32, neighbor_id: u32) {
        self.log_event(SimulationEvent {
            timestamp: self.current_time,
            event_type: SimulationEventType::NeighborDeadTimerExpired { router_id, neighbor_id },
            description: format!("Dead Timer Expired: Router {} neighbor {} dead timer expired", 
                router_id, neighbor_id),
        });
    }
    
    pub fn log_interface_config_changed(&mut self, router_id: u32, interface_id: u32) {
        self.log_event(SimulationEvent {
            timestamp: self.current_time,
            event_type: SimulationEventType::InterfaceConfigChanged { router_id, interface_id },
            description: format!("Interface Config Changed: Router {} interface {} configuration updated", 
                router_id, interface_id),
        });
    }
    
    pub fn get_recent_events(&self, count: usize) -> Vec<SimulationEvent> {
        let start = self.simulation_log.len().saturating_sub(count);
        self.simulation_log[start..].to_vec()
    }
    
    pub fn get_all_events(&self) -> &Vec<SimulationEvent> {
        &self.simulation_log
    }
    
    pub fn get_events_by_type(&self, event_type: SimulationEventType) -> Vec<&SimulationEvent> {
        self.simulation_log.iter()
            .filter(|event| std::mem::discriminant(&event.event_type) == std::mem::discriminant(&event_type))
            .collect()
    }
    
    pub fn get_events_for_router(&self, router_id: u32) -> Vec<&SimulationEvent> {
        self.simulation_log.iter()
            .filter(|event| self.event_involves_router(event, router_id))
            .collect()
    }
    
    pub fn clear_log(&mut self) {
        self.simulation_log.clear();
        console_log!("Event log cleared");
    }
    
    pub fn get_log_size(&self) -> usize {
        self.simulation_log.len()
    }
    
    pub fn get_event_statistics(&self) -> EventStatistics {
        let mut stats = EventStatistics::default();
        
        for event in &self.simulation_log {
            match &event.event_type {
                SimulationEventType::RouterAdded { .. } => stats.routers_added += 1,
                SimulationEventType::LinkCreated { .. } => stats.links_created += 1,
                SimulationEventType::OSPFEnabled { .. } => stats.ospf_enabled += 1,
                SimulationEventType::PacketSent { .. } => stats.packets_sent += 1,
                SimulationEventType::PacketReceived { .. } => stats.packets_received += 1,
                SimulationEventType::RoutingTableUpdated { .. } => stats.route_updates += 1,
                SimulationEventType::NeighborStateChanged { .. } => stats.neighbor_changes += 1,
                SimulationEventType::LinkFailure { .. } => stats.link_failures += 1,
                SimulationEventType::LinkRecovery { .. } => stats.link_recoveries += 1,
                SimulationEventType::RouterFailure { .. } => stats.router_failures += 1,
                SimulationEventType::RouterRecovery { .. } => stats.router_recoveries += 1,
                SimulationEventType::PacketDiscarded { .. } => stats.packets_discarded += 1,
                SimulationEventType::LSARegenerated { .. } => stats.lsa_regenerations += 1,
                SimulationEventType::SPFCalculationStarted { .. } => stats.spf_calculations += 1,
                SimulationEventType::SPFCalculationCompleted { .. } => stats.spf_completions += 1,
                SimulationEventType::NeighborDeadTimerExpired { .. } => stats.dead_timer_expirations += 1,
                SimulationEventType::InterfaceConfigChanged { .. } => {}, // No specific stat for interface config changes
                SimulationEventType::StubAreaConfigured { .. } => {}, // No specific counter for this yet
                SimulationEventType::VirtualLinkConfigured { .. } => {}, // No specific counter for virtual links yet
                SimulationEventType::VirtualLinkRemoved { .. } => {},
                SimulationEventType::VirtualLinkStateChanged { .. } => {},
            }
        }
        
        stats
    }
    
    fn event_involves_router(&self, event: &SimulationEvent, router_id: u32) -> bool {
        match &event.event_type {
            SimulationEventType::RouterAdded { router_id: id, .. } => *id == router_id,
            SimulationEventType::OSPFEnabled { router_id: id } => *id == router_id,
            SimulationEventType::PacketSent { from_router, to_router, .. } => 
                *from_router == router_id || *to_router == router_id,
            SimulationEventType::PacketReceived { router_id: id, .. } => *id == router_id,
            SimulationEventType::RoutingTableUpdated { router_id: id } => *id == router_id,
            SimulationEventType::NeighborStateChanged { router_id: id, .. } => *id == router_id,
            SimulationEventType::LinkCreated { from_router, to_router, .. } |
            SimulationEventType::LinkFailure { from_router, to_router } |
            SimulationEventType::LinkRecovery { from_router, to_router } => 
                *from_router == router_id || *to_router == router_id,
            SimulationEventType::RouterFailure { router_id: id } |
            SimulationEventType::RouterRecovery { router_id: id } => *id == router_id,
            SimulationEventType::PacketDiscarded { router_id: id, from_router, .. } => 
                *id == router_id || *from_router == router_id,
            SimulationEventType::LSARegenerated { router_id: id, .. } => *id == router_id,
            SimulationEventType::SPFCalculationStarted { router_id: id } => *id == router_id,
            SimulationEventType::SPFCalculationCompleted { router_id: id, .. } => *id == router_id,
            SimulationEventType::NeighborDeadTimerExpired { router_id: id, .. } => *id == router_id,
            SimulationEventType::InterfaceConfigChanged { router_id: id, .. } => *id == router_id,
            SimulationEventType::StubAreaConfigured { router_id: id, .. } => *id == router_id,
            SimulationEventType::VirtualLinkConfigured { local_router_id: id, .. } => *id == router_id,
            SimulationEventType::VirtualLinkRemoved { local_router_id: id, .. } => *id == router_id,
            SimulationEventType::VirtualLinkStateChanged { local_router_id: id, .. } => *id == router_id,
        }
    }
}

#[derive(Debug, Default)]
pub struct EventStatistics {
    pub routers_added: usize,
    pub links_created: usize,
    pub ospf_enabled: usize,
    pub packets_sent: usize,
    pub packets_received: usize,
    pub route_updates: usize,
    pub neighbor_changes: usize,
    pub link_failures: usize,
    pub link_recoveries: usize,
    pub router_failures: usize,
    pub router_recoveries: usize,
    pub packets_discarded: usize,
    pub lsa_regenerations: usize,
    pub spf_calculations: usize,
    pub spf_completions: usize,
    pub dead_timer_expirations: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_logging() {
        let mut manager = EventManager::new();
        
        // Log some events
        manager.log_router_added(1, "Router1".to_string());
        manager.log_ospf_enabled(1);
        
        assert_eq!(manager.get_log_size(), 2);
        
        let recent = manager.get_recent_events(1);
        assert_eq!(recent.len(), 1);
        
        let stats = manager.get_event_statistics();
        assert_eq!(stats.routers_added, 1);
        assert_eq!(stats.ospf_enabled, 1);
    }
    
    #[test]
    fn test_router_events_filter() {
        let mut manager = EventManager::new();
        
        manager.log_router_added(1, "Router1".to_string());
        manager.log_router_added(2, "Router2".to_string());
        manager.log_link_created(1, 2, 10);
        
        let router_1_events = manager.get_events_for_router(1);
        assert_eq!(router_1_events.len(), 2); // router added + link created
        
        let router_2_events = manager.get_events_for_router(2);
        assert_eq!(router_2_events.len(), 2); // router added + link created
    }
}
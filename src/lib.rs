use wasm_bindgen::prelude::*;

mod router;
mod network;
mod ospf;
mod protocol;
mod simulation;
mod ospf_engine;
mod spf;
mod ui_state;

use simulation::NetworkSimulation;
use ui_state::{UIState, RouterUI, ConnectionUI};
use serde_json;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[macro_export]
macro_rules! console_log {
    ($($t:tt)*) => (crate::log(&format_args!($($t)*).to_string()))
}

// Set up panic hook for better debugging
use std::panic;

fn set_panic_hook() {
    panic::set_hook(Box::new(|info| {
        console_log!("PANIC: {}", info);
    }));
}


#[wasm_bindgen]
pub struct NetworkSimulator {
    simulation: NetworkSimulation,
    ui_state: UIState,
}


#[wasm_bindgen]
impl NetworkSimulator {
    #[wasm_bindgen(constructor)]
    pub fn new() -> NetworkSimulator {
        set_panic_hook();
        console_log!("NetworkSimulator initialized with panic hook");
        NetworkSimulator {
            simulation: NetworkSimulation::new(),
            ui_state: UIState::new(),
        }
    }

    pub fn add_router(&mut self, name: String, x: f64, y: f64) -> u32 {
        let id = self.simulation.add_router(name.clone(), x, y);
        self.ui_state.set_router_position(id, x, y);
        console_log!("Router {} added with id {}", name, id);
        id
    }

    pub fn connect_routers(&mut self, from_id: u32, to_id: u32, cost: u32) {
        match self.simulation.connect_routers(from_id, to_id, cost) {
            Ok(()) => console_log!("Connected router {} to router {} with cost {}", from_id, to_id, cost),
            Err(e) => console_log!("Error connecting routers: {}", e),
        }
    }
    
    pub fn delete_router(&mut self, router_id: u32) -> bool {
        if self.simulation.delete_router(router_id) {
            self.ui_state.remove_router_position(&router_id);
            console_log!("Router {} deleted", router_id);
            true
        } else {
            console_log!("Failed to delete router {}", router_id);
            false
        }
    }
    
    pub fn disconnect_routers(&mut self, from_id: u32, to_id: u32) -> bool {
        if self.simulation.disconnect_routers(from_id, to_id) {
            console_log!("Disconnected router {} from router {}", from_id, to_id);
            true
        } else {
            console_log!("Failed to disconnect routers {} and {}", from_id, to_id);
            false
        }
    }
    
    pub fn update_router_position(&mut self, router_id: u32, x: f64, y: f64) -> bool {
        if self.simulation.topology.routers.contains_key(&router_id) {
            self.ui_state.set_router_position(router_id, x, y);
            console_log!("Updated position for router {} to ({}, {})", router_id, x, y);
            true
        } else {
            console_log!("Failed to update position for router {}", router_id);
            false
        }
    }

    pub fn enable_ospf(&mut self, router_id: u32) {
        match self.simulation.enable_ospf(router_id) {
            Ok(()) => console_log!("OSPF enabled on router {}", router_id),
            Err(e) => console_log!("Error enabling OSPF: {}", e),
        }
    }

    pub fn start_simulation(&mut self) {
        self.simulation.start_simulation();
        console_log!("Simulation started");
    }

    pub fn stop_simulation(&mut self) {
        self.simulation.stop_simulation();
        console_log!("Simulation stopped");
    }

    pub fn step_simulation(&mut self, time_delta: f64) {
        self.simulation.step_simulation(time_delta);
    }

    pub fn get_routers_json(&self) -> String {
        let routers: Vec<RouterUI> = self.simulation.topology.routers.iter().map(|(id, state)| {
            let (x, y) = self.ui_state.get_router_position(id)
                .copied()
                .unwrap_or((0.0, 0.0));
            RouterUI {
                id: *id,
                name: state.name.clone(),
                x,
                y,
                ospf_enabled: state.ospf_state.is_some(),
            }
        }).collect();
        serde_json::to_string(&routers).unwrap_or_default()
    }

    pub fn get_connections_json(&self) -> String {
        let connections: Vec<ConnectionUI> = self.simulation.topology.links.values().map(|link| {
            ConnectionUI {
                from_router_id: link.router1_id,
                from_interface_id: link.router1_interface_id,
                to_router_id: link.router2_id,
                to_interface_id: link.router2_interface_id,
                cost: link.cost,
            }
        }).collect();
        serde_json::to_string(&connections).unwrap_or_default()
    }

    pub fn get_recent_events_json(&self, count: usize) -> String {
        let events = self.simulation.get_recent_events(count);
        serde_json::to_string(&events).unwrap_or_default()
    }
    
    pub fn get_router_summary_json(&self, router_id: u32) -> String {
        if let Some(router) = self.simulation.topology.routers.get(&router_id) {
            let neighbor_count = self.simulation.get_ospf_neighbor_count(router_id);
            let route_count = router.routing_table.len();
            
            // Get latest OSPF event for this router
            let recent_events = self.simulation.get_recent_events(20);
            let latest_ospf_event = recent_events.iter()
                .filter(|e| match &e.event_type {
                    crate::simulation::SimulationEventType::OSPFEnabled { router_id: rid } => *rid == router_id,
                    crate::simulation::SimulationEventType::NeighborStateChanged { router_id: rid, .. } => *rid == router_id,
                    crate::simulation::SimulationEventType::RoutingTableUpdated { router_id: rid } => *rid == router_id,
                    _ => false
                })
                .last()
                .map(|e| e.description.clone())
                .unwrap_or_else(|| "No recent OSPF events".to_string());
            
            let summary = serde_json::json!({
                "id": router_id,
                "name": router.name,
                "ospf_enabled": router.ospf_state.is_some(),
                "neighbor_count": neighbor_count,
                "route_count": route_count,
                "latest_event": latest_ospf_event
            });
            
            serde_json::to_string(&summary).unwrap_or_default()
        } else {
            "{}".to_string()
        }
    }
    
    pub fn get_all_events_json(&self) -> String {
        serde_json::to_string(&self.simulation.simulation_log).unwrap_or_default()
    }
    
    pub fn get_router_details_json(&self, router_id: u32) -> String {
        if let Some(router) = self.simulation.topology.routers.get(&router_id) {
            // Get neighbor count from OSPF engine through public method
            let ospf_neighbor_count = self.simulation.get_ospf_neighbor_count(router_id);
            let lsa_count = self.simulation.get_ospf_lsa_count(router_id);
            
            let details = serde_json::json!({
                "id": router.id,
                "name": router.name,
                "interfaces": router.interfaces,
                "routing_table": router.routing_table,
                "ospf_enabled": router.ospf_state.is_some(),
                "ospf_neighbors": ospf_neighbor_count,
                "lsa_database_size": lsa_count
            });
            serde_json::to_string(&details).unwrap_or_default()
        } else {
            "{}".to_string()
        }
    }
    
    pub fn get_simulation_stats_json(&self) -> String {
        let stats = serde_json::json!({
            "total_routers": self.simulation.topology.routers.len(),
            "total_links": self.simulation.topology.links.len(),
            "ospf_enabled_routers": self.simulation.topology.routers.values()
                .filter(|r| r.ospf_state.is_some()).count(),
            "simulation_time": self.simulation.simulation_time,
            "total_events": self.simulation.simulation_log.len(),
        });
        serde_json::to_string(&stats).unwrap_or_default()
    }
}
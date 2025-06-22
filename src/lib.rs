use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};

mod router;
mod network;
mod ospf;
mod protocol;
mod simulation;
mod ospf_engine;
mod spf;

use simulation::NetworkSimulation;
use serde_json;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen]
pub struct NetworkSimulator {
    simulation: NetworkSimulation,
    router_positions: std::collections::HashMap<u32, (f64, f64)>,
}

#[wasm_bindgen]
#[derive(Serialize, Deserialize)]
pub struct Router {
    id: u32,
    name: String,
    x: f64,
    y: f64,
    ospf_enabled: bool,
}

#[derive(Serialize, Deserialize)]
pub struct Connection {
    from_router_id: u32,
    to_router_id: u32,
    cost: u32,
}

#[wasm_bindgen]
impl NetworkSimulator {
    #[wasm_bindgen(constructor)]
    pub fn new() -> NetworkSimulator {
        console_log!("NetworkSimulator initialized");
        NetworkSimulator {
            simulation: NetworkSimulation::new(),
            router_positions: std::collections::HashMap::new(),
        }
    }

    pub fn add_router(&mut self, name: String, x: f64, y: f64) -> u32 {
        let id = self.simulation.add_router(name.clone(), x, y);
        self.router_positions.insert(id, (x, y));
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
            self.router_positions.remove(&router_id);
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
        let routers: Vec<Router> = self.simulation.topology.routers.iter().map(|(id, state)| {
            let (x, y) = self.router_positions.get(id).unwrap_or(&(0.0, 0.0));
            Router {
                id: *id,
                name: state.name.clone(),
                x: *x,
                y: *y,
                ospf_enabled: state.ospf_state.is_some(),
            }
        }).collect();
        serde_json::to_string(&routers).unwrap_or_default()
    }

    pub fn get_connections_json(&self) -> String {
        let connections: Vec<Connection> = self.simulation.topology.links.values().map(|link| {
            Connection {
                from_router_id: link.router1_id,
                to_router_id: link.router2_id,
                cost: link.cost,
            }
        }).collect();
        serde_json::to_string(&connections).unwrap_or_default()
    }

    pub fn get_recent_events_json(&self, count: usize) -> String {
        let events = self.simulation.get_recent_events(count);
        serde_json::to_string(&events).unwrap_or_default()
    }
    
    pub fn get_all_events_json(&self) -> String {
        serde_json::to_string(&self.simulation.simulation_log).unwrap_or_default()
    }
    
    pub fn get_router_details_json(&self, router_id: u32) -> String {
        if let Some(router) = self.simulation.topology.routers.get(&router_id) {
            let details = serde_json::json!({
                "id": router.id,
                "name": router.name,
                "interfaces": router.interfaces,
                "routing_table": router.routing_table,
                "ospf_enabled": router.ospf_state.is_some(),
                "ospf_neighbors": router.ospf_state.as_ref()
                    .map(|s| s.neighbors.len()).unwrap_or(0)
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

#[wasm_bindgen]
impl Router {
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> u32 {
        self.id
    }

    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn x(&self) -> f64 {
        self.x
    }

    #[wasm_bindgen(getter)]
    pub fn y(&self) -> f64 {
        self.y
    }

    #[wasm_bindgen(getter)]
    pub fn ospf_enabled(&self) -> bool {
        self.ospf_enabled
    }
}
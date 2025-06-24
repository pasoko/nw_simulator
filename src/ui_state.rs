use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// UI-specific state management
/// Keeps track of visual properties that are separate from business logic
#[derive(Default)]
pub struct UIState {
    /// Router positions on the canvas (router_id -> (x, y))
    router_positions: HashMap<u32, (f64, f64)>,
}

impl UIState {
    pub fn new() -> Self {
        Self {
            router_positions: HashMap::new(),
        }
    }

    /// Add or update a router's position
    pub fn set_router_position(&mut self, router_id: u32, x: f64, y: f64) {
        self.router_positions.insert(router_id, (x, y));
    }

    /// Get a router's position
    pub fn get_router_position(&self, router_id: &u32) -> Option<&(f64, f64)> {
        self.router_positions.get(router_id)
    }

    /// Remove a router's position
    pub fn remove_router_position(&mut self, router_id: &u32) {
        self.router_positions.remove(router_id);
    }

}

/// Router representation for UI/API
#[derive(Serialize, Deserialize)]
pub struct RouterUI {
    pub id: u32,
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub ospf_enabled: bool,
}

/// Connection representation for UI/API
#[derive(Serialize, Deserialize)]
pub struct ConnectionUI {
    pub from_router_id: u32,
    pub from_interface_id: u32,
    pub to_router_id: u32,
    pub to_interface_id: u32,
    pub cost: u32,
}
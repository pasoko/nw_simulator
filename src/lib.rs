// WebAssembly エントリポイント
// 
// このモジュールは、RustベースのOSPFネットワークシミュレーターをWebAssemblyとして
// ブラウザで実行可能にするためのメインインターフェースを提供します。
// NetworkSimulatorクラスがJavaScript側に公開され、ルーター管理、リンク接続、
// OSPFプロトコル制御、シミュレーション実行などのAPIを提供します。

#![allow(dead_code)]
use wasm_bindgen::prelude::*;

mod router;
mod network;
mod network_type;
mod ospf;
mod protocol;
mod simulation;
mod ospf_engine;
mod ospf_neighbor;
mod ospf_lsa_manager;
mod ospf_packet_processor;
mod ospf_timer;
mod ospf_checksum;
mod ospf_dr_election;
mod event_manager;
mod failure_manager;
mod route_calculator;
mod spf;
mod ui_state;
mod serialization;

// New refactored OSPF modules (Phase 1)
pub mod ospf_refactored;

// WebAssembly interface for refactored code
pub mod wasm_interface;

#[cfg(test)]
mod ospf_test;

#[cfg(test)]
mod ospf_dd_retransmit_test;

#[cfg(test)]
mod ospf_dd_retransmit_simple_test;

#[cfg(test)]
mod ospf_area_test;

#[cfg(test)]
mod ospf_spf_delay_test;

#[cfg(test)]
mod ospf_dd_full_state_test;

#[cfg(test)]
mod ospf_dr_election_test;

#[cfg(test)]
mod lib_test;

#[cfg(test)]
mod router_test;

#[cfg(test)]
mod network_test;

#[cfg(test)]
mod ospfv2_compliance_test;

#[cfg(test)]
mod link_failure_spf_test;

use simulation::NetworkSimulation;
use ui_state::UIState;
use serialization::SerializationHelper;

// Re-export WASM interface types
pub use wasm_interface::{RefactoredOSPFEngine, FeatureFlagController};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// Public wrapper for the external log function
pub fn console_log_impl(s: &str) {
    #[cfg(target_arch = "wasm32")]
    log(s);
    #[cfg(not(target_arch = "wasm32"))]
    println!("{}", s);
}

#[macro_export]
macro_rules! console_log {
    ($($t:tt)*) => ($crate::console_log_impl(&format_args!($($t)*).to_string()))
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
    /// Optional refactored engine for gradual migration
    refactored_engine: Option<RefactoredOSPFEngine>,
    /// Feature flags for controlling migration
    feature_flags: FeatureFlagController,
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
            refactored_engine: None,
            feature_flags: FeatureFlagController::new(),
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
        SerializationHelper::routers_to_json(&self.simulation, &self.ui_state)
    }

    pub fn get_connections_json(&self) -> String {
        SerializationHelper::connections_to_json(&self.simulation)
    }

    pub fn get_recent_events_json(&self, count: usize) -> String {
        SerializationHelper::recent_events_to_json(&self.simulation, count)
    }
    
    pub fn get_router_summary_json(&self, router_id: u32) -> String {
        SerializationHelper::router_summary_to_json(&self.simulation, router_id)
    }
    
    pub fn get_all_events_json(&self) -> String {
        SerializationHelper::all_events_to_json(&self.simulation)
    }
    
    pub fn get_router_details_json(&self, router_id: u32) -> String {
        SerializationHelper::router_details_to_json(&self.simulation, router_id)
    }
    
    pub fn get_simulation_stats_json(&self) -> String {
        SerializationHelper::simulation_stats_to_json(&self.simulation)
    }
    
    pub fn toggle_link_failure(&mut self, from_id: u32, to_id: u32) -> bool {
        self.simulation.toggle_link_failure(from_id, to_id)
    }
    
    pub fn toggle_router_failure(&mut self, router_id: u32) -> bool {
        self.simulation.toggle_router_failure(router_id)
    }
    
    // Methods for refactored engine integration
    
    /// Enable the refactored OSPF engine with configuration
    pub fn enable_refactored_engine(&mut self, config_json: String) -> Result<(), JsValue> {
        match RefactoredOSPFEngine::new(config_json) {
            Ok(engine) => {
                self.refactored_engine = Some(engine);
                console_log!("Refactored OSPF engine enabled");
                Ok(())
            }
            Err(e) => {
                console_log!("Failed to enable refactored engine: {:?}", e);
                Err(e)
            }
        }
    }
    
    /// Get feature flags controller
    pub fn get_feature_flags(&self) -> String {
        self.feature_flags.get_flags()
    }
    
    /// Enable specific refactored features
    pub fn enable_refactored_hello(&mut self) {
        self.feature_flags.enable_refactored_hello();
        console_log!("Refactored hello processing enabled");
    }
    
    /// Enable all refactored features
    pub fn enable_all_refactored(&mut self) {
        self.feature_flags.enable_all_refactored();
        console_log!("All refactored features enabled");
    }
    
    /// Process packet through refactored engine if enabled
    pub fn process_packet_refactored(
        &mut self,
        packet_type: u8,
        packet_data: String,
        from_router: u32,
        interface_id: u32,
    ) -> Result<String, JsValue> {
        match &mut self.refactored_engine {
            Some(engine) => {
                engine.process_packet(packet_type, packet_data, from_router, interface_id)
            }
            None => {
                Err(JsValue::from_str("Refactored engine not enabled"))
            }
        }
    }
}
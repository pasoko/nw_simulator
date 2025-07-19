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
mod network_lsa;
mod summary_lsa;
mod as_external_lsa;
mod device;
mod ospf;
mod ospf_auth;
mod ospf_options;
mod ospf_interface_state;
mod ospf_tos;
mod ospf_lsa_age_manager;
mod opaque_lsa;
mod stub_area;
mod virtual_link;
mod route_aggregation;
mod terminal_device;
mod terminal_manager;
mod enhanced_ping;
mod nbma_support;
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
mod ping_manager;
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
mod stub_area_test;

#[cfg(test)]
mod ospf_dd_retransmit_test;

#[cfg(test)]
mod interface_naming_test;

#[cfg(test)]
mod ospf_lsa_retention_test;

#[cfg(test)]
mod ospf_maxage_lsa_test;

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

#[cfg(test)]
mod device_test;

#[cfg(test)]
mod ping_test;

#[cfg(test)]
mod ping_routing_test;

#[cfg(test)]
mod router_config_test;

#[cfg(test)]
mod ospf_auth_test;

#[cfg(test)]
mod ospf_auth_integration_test;

#[cfg(test)]
mod network_lsa_test;

#[cfg(test)]
mod summary_lsa_test;

#[cfg(test)]
mod as_external_lsa_test;

#[cfg(test)]
mod ospf_auth_packet_test;

#[cfg(test)]
mod ospf_options_test;

#[cfg(test)]
mod ospf_interface_state_test;

#[cfg(test)]
mod ospf_tos_test;

#[cfg(test)]
mod ospf_lsa_age_test;

#[cfg(test)]
mod opaque_lsa_test;

#[cfg(test)]
mod virtual_link_test;

#[cfg(test)]
mod route_aggregation_test;

#[cfg(test)]
mod terminal_device_test;

#[cfg(test)]
mod enhanced_ping_test;

#[cfg(test)]
mod nbma_support_test;

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

    pub fn add_host(&mut self, name: String, ip: String, netmask: String, gateway: String, x: f64, y: f64) -> u32 {
        let id = self.simulation.add_host(name.clone(), ip, netmask, gateway);
        self.ui_state.set_router_position(id, x, y);  // UIではルーターと同じ位置管理を使用
        console_log!("Host {} added with id {}", name, id);
        id
    }

    pub fn connect_host_to_router(&mut self, host_id: u32, router_id: u32) -> Result<u32, JsValue> {
        match self.simulation.connect_host_to_router(host_id, router_id) {
            Ok(link_id) => {
                console_log!("Connected host {} to router {}", host_id, router_id);
                Ok(link_id)
            }
            Err(e) => {
                console_log!("Error connecting host to router: {}", e);
                Err(JsValue::from_str(&e))
            }
        }
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

    pub fn get_hosts_json(&self) -> String {
        SerializationHelper::hosts_to_json(&self.simulation, &self.ui_state)
    }

    pub fn get_host_details_json(&self, host_id: u32) -> String {
        SerializationHelper::host_details_to_json(&self.simulation, host_id)
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

    /// Update interface configuration
    pub fn update_interface_config(
        &mut self,
        router_id: u32,
        interface_id: u32,
        config_json: String,
    ) -> Result<(), JsValue> {
        let config: router::InterfaceConfig = serde_json::from_str(&config_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid config JSON: {}", e)))?;
        
        self.simulation.update_interface_config(router_id, interface_id, config)
            .map_err(|e| JsValue::from_str(&e))
    }

    /// Send ping from host
    pub fn send_ping(&mut self, host_id: u32, destination_ip: String) -> Result<u32, JsValue> {
        match self.simulation.send_ping_from_host(host_id, destination_ip) {
            Ok(identifier) => {
                console_log!("Ping sent with identifier {}", identifier);
                Ok(identifier as u32)
            }
            Err(e) => {
                console_log!("Failed to send ping: {}", e);
                Err(JsValue::from_str(&e))
            }
        }
    }

    /// Get recent ping results
    pub fn get_ping_results_json(&self, count: usize) -> String {
        let results = self.simulation.get_recent_ping_results(count);
        serde_json::to_string(&results).unwrap_or_default()
    }
    
    // ==========================================
    // Terminal Device Management Methods
    // ==========================================
    
    /// Add a new terminal device
    pub fn add_terminal(&mut self, name: String, ip_address: String, netmask: String, default_gateway: String, x: f64, y: f64) -> Result<u32, JsValue> {
        match self.simulation.add_terminal(name.clone(), ip_address.clone(), netmask, default_gateway) {
            Ok(terminal_id) => {
                self.ui_state.set_router_position(terminal_id, x, y);
                console_log!("Terminal {} added with id {} at ({}, {})", name, terminal_id, x, y);
                Ok(terminal_id)
            }
            Err(e) => {
                console_log!("Error adding terminal: {}", e);
                Err(JsValue::from_str(&e))
            }
        }
    }
    
    /// Remove a terminal device
    pub fn remove_terminal(&mut self, terminal_id: u32) -> bool {
        match self.simulation.remove_terminal(terminal_id) {
            Ok(()) => {
                self.ui_state.remove_router_position(&terminal_id);
                console_log!("Terminal {} removed", terminal_id);
                true
            }
            Err(e) => {
                console_log!("Error removing terminal: {}", e);
                false
            }
        }
    }
    
    /// Connect terminal to router
    pub fn connect_terminal_to_router(&mut self, terminal_id: u32, router_id: u32) -> Result<(), JsValue> {
        match self.simulation.connect_terminal_to_router(terminal_id, router_id) {
            Ok(()) => {
                console_log!("Connected terminal {} to router {}", terminal_id, router_id);
                Ok(())
            }
            Err(e) => {
                console_log!("Error connecting terminal to router: {}", e);
                Err(JsValue::from_str(&e))
            }
        }
    }
    
    /// Disconnect terminal from router
    pub fn disconnect_terminal(&mut self, terminal_id: u32) -> Result<(), JsValue> {
        match self.simulation.disconnect_terminal(terminal_id) {
            Ok(()) => {
                console_log!("Disconnected terminal {}", terminal_id);
                Ok(())
            }
            Err(e) => {
                console_log!("Error disconnecting terminal: {}", e);
                Err(JsValue::from_str(&e))
            }
        }
    }
    
    /// Send ping from terminal
    pub fn send_ping_from_terminal(&mut self, terminal_id: u32, destination_ip: String) -> Result<u32, JsValue> {
        match self.simulation.send_ping_from_terminal(terminal_id, destination_ip.clone()) {
            Ok(identifier) => {
                console_log!("Ping sent from terminal {} to {} (ID: {})", terminal_id, destination_ip, identifier);
                Ok(identifier as u32)
            }
            Err(e) => {
                console_log!("Failed to send ping from terminal: {}", e);
                Err(JsValue::from_str(&e))
            }
        }
    }
    
    /// Set terminal failed/recovered
    pub fn set_terminal_failed(&mut self, terminal_id: u32, failed: bool) -> bool {
        match self.simulation.set_terminal_failed(terminal_id, failed) {
            Ok(()) => {
                let status = if failed { "failed" } else { "recovered" };
                console_log!("Terminal {} {}", terminal_id, status);
                true
            }
            Err(e) => {
                console_log!("Error setting terminal failure state: {}", e);
                false
            }
        }
    }
    
    /// Get terminal device information
    pub fn get_terminal_info_json(&self, terminal_id: u32) -> String {
        match self.simulation.get_terminal_info(terminal_id) {
            Ok(info) => serde_json::to_string(&info).unwrap_or_default(),
            Err(_) => "{}".to_string(),
        }
    }
    
    /// Get all terminals information
    pub fn get_all_terminals_json(&self) -> String {
        let terminals = self.simulation.get_all_terminals_info();
        serde_json::to_string(&terminals).unwrap_or_default()
    }
    
    /// Find terminal by IP address
    pub fn find_terminal_by_ip(&self, ip_address: String) -> Option<u32> {
        self.simulation.find_terminal_by_ip(&ip_address)
    }
    
    /// Get terminal manager statistics
    pub fn get_terminal_manager_statistics_json(&self) -> String {
        let stats = self.simulation.get_terminal_manager_statistics();
        serde_json::to_string(&stats).unwrap_or_default()
    }
    
    /// Update terminal position
    pub fn update_terminal_position(&mut self, terminal_id: u32, x: f64, y: f64) -> bool {
        // Check if terminal exists in the simulation
        if self.simulation.get_terminal_info(terminal_id).is_ok() {
            self.ui_state.set_router_position(terminal_id, x, y);
            console_log!("Updated position for terminal {} to ({}, {})", terminal_id, x, y);
            true
        } else {
            console_log!("Failed to update position for terminal {}", terminal_id);
            false
        }
    }
    
    // ==========================================
    // Enhanced Ping Management Methods
    // ==========================================
    
    /// Start enhanced ping session
    pub fn start_enhanced_ping(
        &mut self,
        source_id: u32,
        source_ip: String,
        destination_ip: String,
        count: u32,
        interval_seconds: f64,
        packet_size: u32,
        ttl: u8,
    ) -> Result<u32, JsValue> {
        use crate::enhanced_ping::PingSessionConfig;
        
        let config = PingSessionConfig {
            packet_size: packet_size as usize,
            initial_ttl: ttl,
            timeout_seconds: 3.0,
            interval_seconds,
            count,
            dont_fragment: false,
            tos: 0,
        };
        
        match self.simulation.start_enhanced_ping(source_id, source_ip, destination_ip, config) {
            Ok(session_id) => {
                console_log!("Started enhanced ping session {}", session_id);
                Ok(session_id)
            }
            Err(e) => {
                console_log!("Failed to start enhanced ping: {}", e);
                Err(JsValue::from_str(&e))
            }
        }
    }
    
    /// Send next ping in session
    pub fn send_next_ping(&mut self, session_id: u32) -> bool {
        match self.simulation.send_next_ping(session_id) {
            Ok(sent) => {
                if sent {
                    console_log!("Sent next ping for session {}", session_id);
                }
                sent
            }
            Err(e) => {
                console_log!("Error sending ping: {}", e);
                false
            }
        }
    }
    
    /// Stop ping session
    pub fn stop_ping_session(&mut self, session_id: u32) -> String {
        match self.simulation.stop_ping_session(session_id) {
            Ok(summary) => {
                serde_json::to_string(&summary).unwrap_or_default()
            }
            Err(e) => {
                console_log!("Error stopping ping session: {}", e);
                "{}".to_string()
            }
        }
    }
    
    /// Get active ping sessions
    pub fn get_active_ping_sessions(&self) -> String {
        let sessions = self.simulation.get_active_ping_sessions();
        serde_json::to_string(&sessions).unwrap_or_default()
    }
    
    /// Get ping session details
    pub fn get_ping_session_details(&self, session_id: u32) -> String {
        match self.simulation.get_ping_session_details(session_id) {
            Some(details) => details.to_string(),
            None => "{}".to_string(),
        }
    }
    
    /// Get global ping statistics
    pub fn get_ping_statistics(&self) -> String {
        self.simulation.get_ping_statistics().to_string()
    }
    
    /// Start traceroute
    pub fn start_traceroute(&mut self, source_id: u32, source_ip: String, destination_ip: String, max_hops: u8) -> u32 {
        self.simulation.start_traceroute(source_id, source_ip, destination_ip, max_hops)
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
    ) -> Result<(), JsValue> {
        self.simulation.configure_nbma_interface(
            router_id,
            interface_id,
            network_type,
            hello_interval,
            dead_interval,
            priority,
        ).map_err(|e| JsValue::from_str(&e))
    }
    
    /// Add a static neighbor for NBMA interface
    pub fn add_nbma_neighbor(
        &mut self,
        router_id: u32,
        interface_id: u32,
        neighbor_ip: String,
        priority: u8,
        poll_interval: u32,
    ) -> Result<(), JsValue> {
        self.simulation.add_nbma_neighbor(
            router_id,
            interface_id,
            neighbor_ip,
            priority,
            poll_interval,
        ).map_err(|e| JsValue::from_str(&e))
    }
    
    /// Remove a static neighbor from NBMA interface
    pub fn remove_nbma_neighbor(
        &mut self,
        router_id: u32,
        interface_id: u32,
        neighbor_ip: String,
    ) -> Result<(), JsValue> {
        self.simulation.remove_nbma_neighbor(
            router_id,
            interface_id,
            neighbor_ip,
        ).map_err(|e| JsValue::from_str(&e))
    }
    
    /// Get NBMA configuration for a router interface
    pub fn get_nbma_config(&self, router_id: u32, interface_id: u32) -> String {
        self.simulation.get_nbma_config(router_id, interface_id)
    }
    
    /// Get NBMA statistics
    pub fn get_nbma_statistics(&self) -> String {
        self.simulation.get_nbma_statistics()
    }
}
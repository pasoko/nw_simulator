// WebAssembly Interface for Refactored OSPF Implementation
//
// This module provides a clean WASM interface that can gradually
// migrate from the old implementation to the new refactored one.

use wasm_bindgen::prelude::*;
use js_sys;
use crate::ospf_refactored::{
    events::{EventBus, OSPFEvent},
    packet_processor::UnifiedPacketProcessor,
    packets::{OSPFPacket, HelloPacket, DatabaseDescriptionPacket, 
              LinkStateRequestPacket, LinkStateUpdatePacket, LinkStateAckPacket},
};
use std::sync::{Arc, Mutex};
use std::net::Ipv4Addr;
use serde::{Serialize, Deserialize};

/// Configuration for the refactored OSPF engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OSPFConfig {
    pub router_id: String,
    pub area_id: String,
    pub hello_interval: u16,
    pub dead_interval: u16,
    pub use_refactored_engine: bool,
}

impl Default for OSPFConfig {
    fn default() -> Self {
        Self {
            router_id: "0.0.0.0".to_string(),
            area_id: "0.0.0.0".to_string(),
            hello_interval: 10,
            dead_interval: 40,
            use_refactored_engine: false,
        }
    }
}

/// Refactored OSPF Engine exposed to WebAssembly
#[wasm_bindgen]
pub struct RefactoredOSPFEngine {
    processor: Arc<Mutex<UnifiedPacketProcessor>>,
    event_bus: Arc<EventBus>,
    config: OSPFConfig,
}

#[wasm_bindgen]
impl RefactoredOSPFEngine {
    /// Create a new refactored OSPF engine
    #[wasm_bindgen(constructor)]
    pub fn new(config_json: String) -> Result<RefactoredOSPFEngine, JsValue> {
        let config: OSPFConfig = serde_json::from_str(&config_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid config: {}", e)))?;
        
        let router_id = config.router_id.parse::<Ipv4Addr>()
            .map_err(|e| JsValue::from_str(&format!("Invalid router ID: {}", e)))?;
        
        let area_id = config.area_id.parse::<Ipv4Addr>()
            .map_err(|e| JsValue::from_str(&format!("Invalid area ID: {}", e)))?;
        
        let event_bus = Arc::new(EventBus::new());
        let processor = Arc::new(Mutex::new(
            UnifiedPacketProcessor::new(router_id, area_id, event_bus.clone())
        ));
        
        Ok(RefactoredOSPFEngine {
            processor,
            event_bus,
            config,
        })
    }
    
    /// Process an incoming OSPF packet
    pub fn process_packet(
        &mut self,
        packet_type: u8,
        packet_data: String,
        from_router: u32,
        interface_id: u32,
    ) -> Result<String, JsValue> {
        // Parse packet based on type
        let packet = match packet_type {
            1 => {
                let hello: HelloPacket = serde_json::from_str(&packet_data)
                    .map_err(|e| JsValue::from_str(&format!("Invalid hello packet: {}", e)))?;
                OSPFPacket::Hello(hello)
            }
            2 => {
                let dd: DatabaseDescriptionPacket = serde_json::from_str(&packet_data)
                    .map_err(|e| JsValue::from_str(&format!("Invalid DD packet: {}", e)))?;
                OSPFPacket::DatabaseDescription(dd)
            }
            3 => {
                let lsr: LinkStateRequestPacket = serde_json::from_str(&packet_data)
                    .map_err(|e| JsValue::from_str(&format!("Invalid LSR packet: {}", e)))?;
                OSPFPacket::LinkStateRequest(lsr)
            }
            4 => {
                let lsu: LinkStateUpdatePacket = serde_json::from_str(&packet_data)
                    .map_err(|e| JsValue::from_str(&format!("Invalid LSU packet: {}", e)))?;
                OSPFPacket::LinkStateUpdate(lsu)
            }
            5 => {
                let lsack: LinkStateAckPacket = serde_json::from_str(&packet_data)
                    .map_err(|e| JsValue::from_str(&format!("Invalid LSAck packet: {}", e)))?;
                OSPFPacket::LinkStateAck(lsack)
            }
            _ => {
                return Err(JsValue::from_str(&format!("Unknown packet type: {}", packet_type)));
            }
        };
        
        // Process packet
        let mut processor = self.processor.lock().unwrap();
        let events = processor.process_packet(packet, from_router, interface_id)
            .map_err(|e| JsValue::from_str(&format!("Packet processing error: {}", e)))?;
        
        // Convert events to JSON
        let events_json = serde_json::to_string(&events)
            .map_err(|e| JsValue::from_str(&format!("Event serialization error: {}", e)))?;
        
        Ok(events_json)
    }
    
    /// Generate a hello packet
    pub fn generate_hello(&self, _interface_id: u32) -> Result<String, JsValue> {
        let config = &self.config;
        let router_id = config.router_id.parse::<Ipv4Addr>()
            .map_err(|e| JsValue::from_str(&format!("Invalid router ID: {}", e)))?;
        let area_id = config.area_id.parse::<Ipv4Addr>()
            .map_err(|e| JsValue::from_str(&format!("Invalid area ID: {}", e)))?;
        
        let hello = HelloPacket::new(
            router_id,
            area_id,
            Ipv4Addr::new(255, 255, 255, 0), // Default netmask
            config.hello_interval,
            1, // Default priority
            config.dead_interval as u32,
        );
        
        serde_json::to_string(&hello)
            .map_err(|e| JsValue::from_str(&format!("Hello serialization error: {}", e)))
    }
    
    /// Get pending events from the event bus
    pub fn get_pending_events(&self) -> Result<String, JsValue> {
        // Process events (this actually runs the event handlers)
        let _ = self.event_bus.process_events();
        
        // For now, return empty array since we don't have a way to get events
        // In a real implementation, we'd need to add a method to EventBus to retrieve events
        let js_events: Vec<SimpleEvent> = Vec::new();
        
        serde_json::to_string(&js_events)
            .map_err(|e| JsValue::from_str(&format!("Event serialization error: {}", e)))
    }
    
    /// Get current configuration
    pub fn get_config(&self) -> String {
        serde_json::to_string(&self.config).unwrap_or_else(|_| "{}".to_string())
    }
    
    /// Update configuration
    pub fn update_config(&mut self, config_json: String) -> Result<(), JsValue> {
        let new_config: OSPFConfig = serde_json::from_str(&config_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid config: {}", e)))?;
        
        self.config = new_config;
        Ok(())
    }
}

/// Simplified event structure for JavaScript consumption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleEvent {
    pub event_type: String,
    pub timestamp: f64,
    pub details: serde_json::Value,
}

impl SimpleEvent {
    fn from_ospf_event(event: OSPFEvent) -> Self {
        let (event_type, details) = match &event {
            OSPFEvent::NeighborStateChanged { neighbor_id, from_state, to_state, .. } => {
                ("NeighborStateChanged".to_string(), serde_json::json!({
                    "neighbor_id": neighbor_id,
                    "from_state": format!("{:?}", from_state),
                    "to_state": format!("{:?}", to_state),
                }))
            }
            OSPFEvent::LSAReceived { lsa_type, lsa_id, advertising_router, .. } => {
                ("LSAReceived".to_string(), serde_json::json!({
                    "lsa_type": lsa_type,
                    "lsa_id": lsa_id,
                    "advertising_router": advertising_router,
                }))
            }
            OSPFEvent::SPFRequired { area_id, reason } => {
                ("SPFRequired".to_string(), serde_json::json!({
                    "area_id": area_id,
                    "reason": reason,
                }))
            }
            _ => ("Unknown".to_string(), serde_json::json!({})),
        };
        
        SimpleEvent {
            event_type,
            timestamp: js_sys::Date::now() / 1000.0,
            details,
        }
    }
}

/// Feature flag controller for gradual migration
#[wasm_bindgen]
pub struct FeatureFlagController {
    use_refactored_hello: bool,
    use_refactored_dd: bool,
    use_refactored_lsr: bool,
    use_refactored_lsu: bool,
    use_refactored_lsack: bool,
}

#[wasm_bindgen]
impl FeatureFlagController {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            use_refactored_hello: false,
            use_refactored_dd: false,
            use_refactored_lsr: false,
            use_refactored_lsu: false,
            use_refactored_lsack: false,
        }
    }
    
    /// Enable refactored hello packet processing
    pub fn enable_refactored_hello(&mut self) {
        self.use_refactored_hello = true;
    }
    
    /// Enable refactored DD packet processing
    pub fn enable_refactored_dd(&mut self) {
        self.use_refactored_dd = true;
    }
    
    /// Enable all refactored packet processing
    pub fn enable_all_refactored(&mut self) {
        self.use_refactored_hello = true;
        self.use_refactored_dd = true;
        self.use_refactored_lsr = true;
        self.use_refactored_lsu = true;
        self.use_refactored_lsack = true;
    }
    
    /// Get current feature flags as JSON
    pub fn get_flags(&self) -> String {
        serde_json::json!({
            "hello": self.use_refactored_hello,
            "dd": self.use_refactored_dd,
            "lsr": self.use_refactored_lsr,
            "lsu": self.use_refactored_lsu,
            "lsack": self.use_refactored_lsack,
        }).to_string()
    }
}
// Error Context Implementation
//
// Provides rich context information for errors to aid in debugging
// and recovery decisions.

use std::fmt;
use serde::{Serialize, Deserialize};
use crate::ospf_refactored::packets::PacketType;

// Helper function for getting timestamp that works in both WASM and native
fn get_timestamp() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now() / 1000.0
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
    }
}

/// Context information for errors
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ErrorContext {
    /// Router ID where error occurred
    pub router_id: Option<u32>,
    /// Neighbor ID involved in error
    pub neighbor_id: Option<u32>,
    /// Interface ID where error occurred
    pub interface_id: Option<u32>,
    /// Packet type being processed
    pub packet_type: Option<PacketType>,
    /// Current state when error occurred
    pub state: Option<String>,
    /// Timestamp of error
    pub timestamp: f64,
    /// Operation being performed
    pub operation: Option<String>,
    /// Additional context
    pub additional_info: Option<serde_json::Value>,
}

impl ErrorContext {
    /// Create a new error context for a router
    pub fn new(router_id: u32) -> Self {
        Self {
            router_id: Some(router_id),
            timestamp: get_timestamp(),
            ..Default::default()
        }
    }
    
    /// Add neighbor information
    pub fn with_neighbor(mut self, neighbor_id: u32) -> Self {
        self.neighbor_id = Some(neighbor_id);
        self
    }
    
    /// Add interface information
    pub fn with_interface(mut self, interface_id: u32) -> Self {
        self.interface_id = Some(interface_id);
        self
    }
    
    /// Add packet type information
    pub fn with_packet_type(mut self, packet_type: PacketType) -> Self {
        self.packet_type = Some(packet_type);
        self
    }
    
    /// Add state information
    pub fn with_state(mut self, state: impl Into<String>) -> Self {
        self.state = Some(state.into());
        self
    }
    
    /// Add operation information
    pub fn with_operation(mut self, operation: impl Into<String>) -> Self {
        self.operation = Some(operation.into());
        self
    }
    
    /// Add additional information
    pub fn with_info(mut self, key: &str, value: impl Serialize) -> Self {
        let mut info = match self.additional_info {
            Some(serde_json::Value::Object(map)) => map,
            _ => serde_json::Map::new(),
        };
        
        if let Ok(value) = serde_json::to_value(value) {
            info.insert(key.to_string(), value);
        }
        
        self.additional_info = Some(serde_json::Value::Object(info));
        self
    }
    
    /// Get a concise description
    pub fn description(&self) -> String {
        let mut parts = Vec::new();
        
        if let Some(router_id) = self.router_id {
            parts.push(format!("router={}", router_id));
        }
        
        if let Some(neighbor_id) = self.neighbor_id {
            parts.push(format!("neighbor={}", neighbor_id));
        }
        
        if let Some(interface_id) = self.interface_id {
            parts.push(format!("interface={}", interface_id));
        }
        
        if let Some(packet_type) = &self.packet_type {
            parts.push(format!("packet={:?}", packet_type));
        }
        
        if let Some(operation) = &self.operation {
            parts.push(format!("op={}", operation));
        }
        
        parts.join(", ")
    }
}

impl fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

/// Builder for creating error contexts
pub struct ErrorContextBuilder {
    context: ErrorContext,
}

impl ErrorContextBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            context: ErrorContext::default(),
        }
    }
    
    /// Set router ID
    pub fn router(mut self, router_id: u32) -> Self {
        self.context.router_id = Some(router_id);
        self
    }
    
    /// Set neighbor ID
    pub fn neighbor(mut self, neighbor_id: u32) -> Self {
        self.context.neighbor_id = Some(neighbor_id);
        self
    }
    
    /// Set interface ID
    pub fn interface(mut self, interface_id: u32) -> Self {
        self.context.interface_id = Some(interface_id);
        self
    }
    
    /// Set packet type
    pub fn packet(mut self, packet_type: PacketType) -> Self {
        self.context.packet_type = Some(packet_type);
        self
    }
    
    /// Set operation
    pub fn operation(mut self, operation: impl Into<String>) -> Self {
        self.context.operation = Some(operation.into());
        self
    }
    
    /// Build the context
    pub fn build(mut self) -> ErrorContext {
        self.context.timestamp = get_timestamp();
        self.context
    }
}

/// Trait for types that can provide error context
pub trait HasErrorContext {
    /// Get the error context
    fn error_context(&self) -> ErrorContext;
}

/// Macro for creating error context
#[macro_export]
macro_rules! error_context {
    (router: $router:expr) => {
        $crate::ospf_refactored::error_handling::ErrorContext::new($router)
    };
    (router: $router:expr, neighbor: $neighbor:expr) => {
        $crate::ospf_refactored::error_handling::ErrorContext::new($router)
            .with_neighbor($neighbor)
    };
    (router: $router:expr, neighbor: $neighbor:expr, interface: $interface:expr) => {
        $crate::ospf_refactored::error_handling::ErrorContext::new($router)
            .with_neighbor($neighbor)
            .with_interface($interface)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_context_builder() {
        let context = ErrorContextBuilder::new()
            .router(1)
            .neighbor(2)
            .interface(1)
            .packet(PacketType::Hello)
            .operation("process_hello")
            .build();
        
        assert_eq!(context.router_id, Some(1));
        assert_eq!(context.neighbor_id, Some(2));
        assert_eq!(context.interface_id, Some(1));
        assert_eq!(context.packet_type, Some(PacketType::Hello));
        assert_eq!(context.operation, Some("process_hello".to_string()));
    }
    
    #[test]
    fn test_error_context_description() {
        let context = ErrorContext::new(1)
            .with_neighbor(2)
            .with_packet_type(PacketType::Hello)
            .with_operation("validation");
        
        let desc = context.description();
        assert!(desc.contains("router=1"));
        assert!(desc.contains("neighbor=2"));
        assert!(desc.contains("packet=Hello"));
        assert!(desc.contains("op=validation"));
    }
    
    #[test]
    fn test_error_context_with_info() {
        let context = ErrorContext::new(1)
            .with_info("sequence", 12345)
            .with_info("retry_count", 3);
        
        if let Some(serde_json::Value::Object(map)) = &context.additional_info {
            assert_eq!(map.get("sequence"), Some(&serde_json::json!(12345)));
            assert_eq!(map.get("retry_count"), Some(&serde_json::json!(3)));
        } else {
            panic!("Additional info should be an object");
        }
    }
}
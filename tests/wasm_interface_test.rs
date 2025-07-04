// Tests for WebAssembly interface
//
// These tests verify that the WASM interface correctly wraps
// the refactored OSPF implementation.

#[cfg(test)]
mod tests {
    use nw_simulator::wasm_interface::{OSPFConfig, RefactoredOSPFEngine, FeatureFlagController};
    use serde_json;
    
    #[test]
    fn test_refactored_engine_creation() {
        let config = OSPFConfig {
            router_id: "1.1.1.1".to_string(),
            area_id: "0.0.0.0".to_string(),
            hello_interval: 10,
            dead_interval: 40,
            use_refactored_engine: true,
        };
        
        let config_json = serde_json::to_string(&config).unwrap();
        let engine = RefactoredOSPFEngine::new(config_json);
        
        assert!(engine.is_ok());
    }
    
    #[test]
    fn test_hello_generation() {
        let config = OSPFConfig {
            router_id: "1.1.1.1".to_string(),
            area_id: "0.0.0.0".to_string(),
            hello_interval: 10,
            dead_interval: 40,
            use_refactored_engine: true,
        };
        let config_json = serde_json::to_string(&config).unwrap();
        let engine = RefactoredOSPFEngine::new(config_json).unwrap();
        
        let hello_json = engine.generate_hello(1);
        assert!(hello_json.is_ok());
        
        // Verify it's valid JSON
        let hello: serde_json::Value = serde_json::from_str(&hello_json.unwrap()).unwrap();
        assert_eq!(hello["header"]["version"], 2);
        assert_eq!(hello["header"]["packet_type"], 1);
    }
    
    #[test]
    fn test_packet_processing() {
        let config = OSPFConfig {
            router_id: "1.1.1.1".to_string(),
            area_id: "0.0.0.0".to_string(),
            hello_interval: 10,
            dead_interval: 40,
            use_refactored_engine: true,
        };
        
        let config_json = serde_json::to_string(&config).unwrap();
        let mut engine = RefactoredOSPFEngine::new(config_json).unwrap();
        
        // Create a hello packet
        let hello_packet = serde_json::json!({
            "header": {
                "version": 2,
                "packet_type": 1,
                "packet_length": 44,
                "router_id": "2.2.2.2",
                "area_id": "0.0.0.0",
                "checksum": 0,
                "auth_type": 0,
                "authentication": [0, 0, 0, 0, 0, 0, 0, 0]
            },
            "network_mask": "255.255.255.0",
            "hello_interval": 10,
            "options": 2,
            "priority": 1,
            "router_dead_interval": 40,
            "designated_router": "0.0.0.0",
            "backup_designated_router": "0.0.0.0",
            "neighbors": []
        });
        
        let result = engine.process_packet(
            1, // Hello packet type
            hello_packet.to_string(),
            2, // From router ID 2
            1  // Interface ID 1
        );
        
        assert!(result.is_ok());
        
        // Parse events
        let events: Vec<serde_json::Value> = serde_json::from_str(&result.unwrap()).unwrap();
        assert!(!events.is_empty());
    }
    
    #[test]
    fn test_feature_flags() {
        let mut flags = FeatureFlagController::new();
        
        // Check initial state
        let initial_flags = flags.get_flags();
        let parsed: serde_json::Value = serde_json::from_str(&initial_flags).unwrap();
        assert_eq!(parsed["hello"], false);
        
        // Enable hello
        flags.enable_refactored_hello();
        let updated_flags = flags.get_flags();
        let parsed: serde_json::Value = serde_json::from_str(&updated_flags).unwrap();
        assert_eq!(parsed["hello"], true);
        
        // Enable all
        flags.enable_all_refactored();
        let all_flags = flags.get_flags();
        let parsed: serde_json::Value = serde_json::from_str(&all_flags).unwrap();
        assert_eq!(parsed["dd"], true);
        assert_eq!(parsed["lsr"], true);
        assert_eq!(parsed["lsu"], true);
        assert_eq!(parsed["lsack"], true);
    }
    
    #[test]
    fn test_config_update() {
        let initial_config = OSPFConfig::default();
        let config_json = serde_json::to_string(&initial_config).unwrap();
        let mut engine = RefactoredOSPFEngine::new(config_json).unwrap();
        
        // Get initial config
        let current_config = engine.get_config();
        let parsed: OSPFConfig = serde_json::from_str(&current_config).unwrap();
        assert_eq!(parsed.hello_interval, 10);
        
        // Update config
        let new_config = OSPFConfig {
            router_id: "3.3.3.3".to_string(),
            area_id: "0.0.0.1".to_string(),
            hello_interval: 5,
            dead_interval: 20,
            use_refactored_engine: true,
        };
        
        let new_config_json = serde_json::to_string(&new_config).unwrap();
        assert!(engine.update_config(new_config_json).is_ok());
        
        // Verify update
        let updated_config = engine.get_config();
        let parsed: OSPFConfig = serde_json::from_str(&updated_config).unwrap();
        assert_eq!(parsed.hello_interval, 5);
        assert_eq!(parsed.router_id, "3.3.3.3");
    }
}
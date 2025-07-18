// Test Compatibility Layer
//
// This module provides compatibility for existing tests during the refactoring transition.
// It allows tests to work with both old and new implementations.

#[cfg(test)]
pub mod test_helpers {
    use crate::ospf_engine::OSPFEngine;
    use crate::ospf_engine_refactored::OSPFEngineRefactored;
    use crate::ospf_options::OSPFOptions;
    
    /// Trait to abstract common operations between old and new engines
    pub trait OSPFEngineOperations {
        fn get_router_id(&self) -> &str;
        fn process_hello(&mut self, packet: &crate::ospf::HelloPacket, from: u32, interface: u32) 
            -> Vec<crate::event_manager::PacketEvent>;
        fn update_time(&mut self, time: f64) -> Vec<crate::event_manager::PacketEvent>;
        fn get_neighbor_count(&self) -> usize;
        fn get_lsa_count(&self) -> usize;
    }
    
    /// Implement the trait for the original engine
    impl OSPFEngineOperations for OSPFEngine {
        fn get_router_id(&self) -> &str {
            &self.router_id
        }
        
        fn process_hello(&mut self, packet: &crate::ospf::HelloPacket, from: u32, interface: u32) 
            -> Vec<crate::event_manager::PacketEvent> {
            self.process_hello_packet(packet, from, interface)
        }
        
        fn update_time(&mut self, time: f64) -> Vec<crate::event_manager::PacketEvent> {
            self.update_time(time)
        }
        
        fn get_neighbor_count(&self) -> usize {
            self.neighbor_manager.get_neighbor_count()
        }
        
        fn get_lsa_count(&self) -> usize {
            self.lsa_manager.get_lsa_count()
        }
    }
    
    /// Implement the trait for the refactored engine
    impl OSPFEngineOperations for OSPFEngineRefactored {
        fn get_router_id(&self) -> &str {
            &self.router_id
        }
        
        fn process_hello(&mut self, packet: &crate::ospf::HelloPacket, from: u32, interface: u32) 
            -> Vec<crate::event_manager::PacketEvent> {
            self.process_hello_packet(packet, from, interface)
        }
        
        fn update_time(&mut self, time: f64) -> Vec<crate::event_manager::PacketEvent> {
            self.update_time(time)
        }
        
        fn get_neighbor_count(&self) -> usize {
            self.neighbor_manager.get_neighbor_count()
        }
        
        fn get_lsa_count(&self) -> usize {
            self.lsa_manager.get_lsa_count()
        }
    }
    
    /// Test harness that can work with either engine implementation
    pub struct OSPFTestHarness<T: OSPFEngineOperations> {
        pub engine: T,
    }
    
    impl<T: OSPFEngineOperations> OSPFTestHarness<T> {
        pub fn new(engine: T) -> Self {
            Self { engine }
        }
        
        /// Run a standard hello packet test
        pub fn test_hello_packet_processing(&mut self) -> bool {
            let packet = crate::ospf::HelloPacket {
                network_mask: "255.255.255.0".to_string(),
                hello_interval: 10,
                options: OSPFOptions::standard_area_options(),
                router_priority: 1,
                router_dead_interval: 40,
                designated_router: "0.0.0.0".to_string(),
                backup_designated_router: "0.0.0.0".to_string(),
                neighbors: vec![],
            };
            
            let events = self.engine.process_hello(&packet, 2, 1);
            
            // Basic validation
            self.engine.get_neighbor_count() > 0
        }
        
        /// Run a timer test
        pub fn test_timer_processing(&mut self) -> bool {
            let initial_time = 0.0;
            let events1 = self.engine.update_time(initial_time);
            
            let later_time = 10.0;
            let events2 = self.engine.update_time(later_time);
            
            // Should have generated some hello timer events
            events2.len() > 0
        }
    }
}

#[cfg(test)]
mod compatibility_tests {
    use super::test_helpers::*;
    use crate::ospf_engine::OSPFEngine;
    use crate::ospf_neighbor::OSPFNeighborManager;
    use crate::ospf_lsa_manager::OSPFLSAManager;
    use crate::ospf_packet_processor::OSPFPacketProcessor;
    use crate::ospf_timer::OSPFTimerManager;
    use std::collections::HashMap;
    
    fn create_test_engine() -> OSPFEngine {
        OSPFEngine::new(
            "1.1.1.1".to_string(),
            0,
            OSPFNeighborManager::new("1.1.1.1".to_string()),
            OSPFLSAManager::new("1.1.1.1".to_string(), 0),
            OSPFPacketProcessor::new("1.1.1.1".to_string()),
            OSPFTimerManager::new(),
        )
    }
    
    #[test]
    fn test_compatibility_hello_processing() {
        let engine = create_test_engine();
        let mut harness = OSPFTestHarness::new(engine);
        
        assert!(harness.test_hello_packet_processing());
    }
    
    #[test]
    fn test_compatibility_timer_processing() {
        let engine = create_test_engine();
        let mut harness = OSPFTestHarness::new(engine);
        
        assert!(harness.test_timer_processing());
    }
}
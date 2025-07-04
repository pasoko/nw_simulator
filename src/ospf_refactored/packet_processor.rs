// Unified Packet Processor
//
// This module integrates the new packet definitions with the event system
// and state machines, providing a clean interface for packet processing.

use crate::ospf_refactored::packets::{OSPFPacket, PacketType, PacketError};
use crate::ospf_refactored::packets::hello::{HelloPacket, HelloPacketHandler};
use crate::ospf_refactored::packets::dd::{DatabaseDescriptionPacket, DDPacketHandler};
use crate::ospf_refactored::packets::lsr::{LinkStateRequestPacket, LSRPacketHandler};
use crate::ospf_refactored::packets::lsu::{LinkStateUpdatePacket, LSUPacketHandler};
use crate::ospf_refactored::packets::lsack::{LinkStateAckPacket, LSAckPacketHandler};
use crate::ospf_refactored::events::{OSPFEvent, EventBus, EventResult};
use crate::ospf_refactored::state::{NeighborState, StateContext, NeighborTransitionValidator};
use crate::ospf_refactored::error_handling::{
    ErrorContext, ErrorLogger, LogLevel, RetryConfig, RetryPolicy, 
    RecoveryCoordinator, RecoveryAction, ErrorMetrics
};
use std::net::Ipv4Addr;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

/// Unified packet processor that coordinates packet handling
pub struct UnifiedPacketProcessor {
    router_id: Ipv4Addr,
    area_id: Ipv4Addr,
    event_bus: Arc<EventBus>,
    hello_handler: HelloPacketHandler,
    dd_handler: DDPacketHandler,
    lsr_handler: LSRPacketHandler,
    lsu_handler: LSUPacketHandler,
    lsack_handler: LSAckPacketHandler,
    neighbor_states: HashMap<u32, NeighborState>,
    transition_validator: NeighborTransitionValidator,
    /// Error recovery coordinator
    recovery_coordinator: RecoveryCoordinator,
    /// Error metrics tracking
    error_metrics: ErrorMetrics,
    /// Retry configuration
    retry_config: RetryConfig,
}

impl UnifiedPacketProcessor {
    /// Create a new unified packet processor
    pub fn new(
        router_id: Ipv4Addr,
        area_id: Ipv4Addr,
        event_bus: Arc<EventBus>,
    ) -> Self {
        Self {
            router_id,
            area_id,
            event_bus,
            hello_handler: HelloPacketHandler::new(router_id, area_id, 10, 40),
            dd_handler: DDPacketHandler::new(router_id),
            lsr_handler: LSRPacketHandler::new(router_id),
            lsu_handler: LSUPacketHandler::new(router_id),
            lsack_handler: LSAckPacketHandler::new(router_id),
            neighbor_states: HashMap::new(),
            transition_validator: NeighborTransitionValidator::new(),
            recovery_coordinator: RecoveryCoordinator::new(),
            error_metrics: ErrorMetrics::default(),
            retry_config: RetryConfig::default(),
        }
    }
    
    /// Process an incoming OSPF packet with error handling
    pub fn process_packet(
        &mut self,
        packet: OSPFPacket,
        from_router: u32,
        interface_id: u32,
    ) -> Result<Vec<OSPFEvent>, PacketError> {
        // Create error context
        let context = ErrorContext::new(u32::from_be_bytes(self.router_id.octets()))
            .with_neighbor(from_router)
            .with_interface(interface_id)
            .with_packet_type(packet.packet_type())
            .with_operation("process_packet");
        
        // Process with error handling
        match self.process_packet_internal(packet, from_router, interface_id) {
            Ok(events) => {
                // Reset error metrics on success
                self.error_metrics.record_recovery();
                Ok(events)
            }
            Err(error) => {
                // Log the error
                error.log_error(LogLevel::Error, "packet_processing");
                
                // Update metrics
                self.error_metrics.record_error(&format!("{:?}", error));
                
                // Check circuit breaker
                if self.error_metrics.should_circuit_break(10) {
                    "Circuit breaker tripped".log_error(LogLevel::Critical, "packet_processing");
                    return Err(PacketError::ProcessingError("Circuit breaker open".to_string()));
                }
                
                // Attempt recovery
                match self.recovery_coordinator.handle_error(&format!("{:?}", error), &context) {
                    Ok(recovery_events) => {
                        // Recovery succeeded, return recovery events
                        Ok(recovery_events)
                    }
                    Err(recovery_error) => {
                        recovery_error.log_error(LogLevel::Error, "recovery");
                        Err(error)
                    }
                }
            }
        }
    }
    
    /// Internal packet processing without error recovery
    fn process_packet_internal(
        &mut self,
        packet: OSPFPacket,
        from_router: u32,
        interface_id: u32,
    ) -> Result<Vec<OSPFEvent>, PacketError> {
        // Validate common header
        self.validate_common_header(&packet)?;
        
        // Process based on packet type
        let events = match packet {
            OSPFPacket::Hello(hello) => {
                self.process_hello_packet(hello, from_router, interface_id)?
            }
            OSPFPacket::DatabaseDescription(dd) => {
                self.process_dd_packet(dd, from_router, interface_id)?
            }
            OSPFPacket::LinkStateRequest(lsr) => {
                self.process_lsr_packet(lsr, from_router, interface_id)?
            }
            OSPFPacket::LinkStateUpdate(lsu) => {
                self.process_lsu_packet(lsu, from_router, interface_id)?
            }
            OSPFPacket::LinkStateAck(lsack) => {
                self.process_lsack_packet(lsack, from_router, interface_id)?
            }
        };
        
        // Publish events to event bus
        for event in &events {
            self.event_bus.publish(event.clone())
                .map_err(|e| PacketError::ProcessingError(format!("Event publish failed: {:?}", e)))?;
        }
        
        Ok(events)
    }
    
    /// Validate common OSPF header
    fn validate_common_header(&self, packet: &OSPFPacket) -> Result<(), PacketError> {
        let header = packet.header();
        
        // Check version
        if header.version != 2 {
            return Err(PacketError::InvalidFormat(
                format!("Unsupported OSPF version: {}", header.version)
            ));
        }
        
        // Check area ID
        if header.area_id != self.area_id {
            return Err(PacketError::ProcessingError(
                format!("Area ID mismatch: expected {:?}, got {:?}", self.area_id, header.area_id)
            ));
        }
        
        // TODO: Verify checksum
        
        Ok(())
    }
    
    /// Process Hello packet
    fn process_hello_packet(
        &mut self,
        packet: HelloPacket,
        from_router: u32,
        interface_id: u32,
    ) -> Result<Vec<OSPFEvent>, PacketError> {
        let mut events = Vec::new();
        
        // Get current neighbor state
        let current_state = self.neighbor_states.get(&from_router).copied()
            .unwrap_or(NeighborState::Down);
        
        // Process through hello handler
        let handler_events = self.hello_handler.process_hello(&packet, from_router)
            .map_err(|e| PacketError::ProcessingError(format!("Hello processing failed: {:?}", e)))?;
        
        events.extend(handler_events);
        
        // Check for state changes based on hello
        let is_bidirectional = packet.is_bidirectional(self.router_id);
        let new_state = self.determine_new_neighbor_state(current_state, is_bidirectional)?;
        
        if new_state != current_state {
            // Validate transition
            self.transition_validator.validate_transition(current_state, new_state)
                .map_err(|e| PacketError::ProcessingError(format!("Invalid transition: {:?}", e)))?;
            
            // Update state
            self.neighbor_states.insert(from_router, new_state);
            
            // Generate state change event
            events.push(OSPFEvent::NeighborStateChanged {
                router_id: u32::from_be_bytes(self.router_id.octets()),
                neighbor_id: from_router,
                from_state: current_state,
                to_state: new_state,
                interface_id,
            });
        }
        
        Ok(events)
    }
    
    /// Process Database Description packet
    fn process_dd_packet(
        &mut self,
        packet: DatabaseDescriptionPacket,
        from_router: u32,
        interface_id: u32,
    ) -> Result<Vec<OSPFEvent>, PacketError> {
        let mut events = Vec::new();
        
        // Check neighbor state
        let current_state = self.neighbor_states.get(&from_router).copied()
            .unwrap_or(NeighborState::Down);
        
        match current_state {
            NeighborState::ExStart => {
                // Handle master/slave negotiation
                let handler_events = self.dd_handler.handle_dd_packet(&packet, from_router)
                    .map_err(|e| PacketError::ProcessingError(format!("DD handling failed: {:?}", e)))?;
                
                events.extend(handler_events);
                
                // Check if negotiation complete
                if self.dd_handler.is_negotiation_complete(from_router) {
                    self.neighbor_states.insert(from_router, NeighborState::Exchange);
                    
                    events.push(OSPFEvent::NeighborStateChanged {
                        router_id: u32::from_be_bytes(self.router_id.octets()),
                        neighbor_id: from_router,
                        from_state: current_state,
                        to_state: NeighborState::Exchange,
                        interface_id,
                    });
                }
            }
            NeighborState::Exchange => {
                // Process LSA headers
                let handler_events = self.dd_handler.handle_dd_packet(&packet, from_router)
                    .map_err(|e| PacketError::ProcessingError(format!("DD handling failed: {:?}", e)))?;
                
                events.extend(handler_events);
                
                // Check if exchange complete
                if self.dd_handler.is_exchange_complete(from_router) {
                    let new_state = if self.dd_handler.has_lsas_to_request(from_router) {
                        NeighborState::Loading
                    } else {
                        NeighborState::Full
                    };
                    
                    self.neighbor_states.insert(from_router, new_state);
                    
                    events.push(OSPFEvent::NeighborStateChanged {
                        router_id: u32::from_be_bytes(self.router_id.octets()),
                        neighbor_id: from_router,
                        from_state: current_state,
                        to_state: new_state,
                        interface_id,
                    });
                }
            }
            _ => {
                return Err(PacketError::ProcessingError(
                    format!("DD packet unexpected in state {:?}", current_state)
                ));
            }
        }
        
        Ok(events)
    }
    
    /// Process Link State Request packet
    fn process_lsr_packet(
        &mut self,
        packet: LinkStateRequestPacket,
        from_router: u32,
        interface_id: u32,
    ) -> Result<Vec<OSPFEvent>, PacketError> {
        // Process LSR and generate LSU response
        let events = self.lsr_handler.handle_lsr_packet(&packet, from_router)
            .map_err(|e| PacketError::ProcessingError(format!("LSR handling failed: {:?}", e)))?;
        
        Ok(events)
    }
    
    /// Process Link State Update packet
    fn process_lsu_packet(
        &mut self,
        packet: LinkStateUpdatePacket,
        from_router: u32,
        interface_id: u32,
    ) -> Result<Vec<OSPFEvent>, PacketError> {
        let mut events = Vec::new();
        
        // Process LSAs
        let handler_events = self.lsu_handler.handle_lsu_packet(&packet, from_router)
            .map_err(|e| PacketError::ProcessingError(format!("LSU handling failed: {:?}", e)))?;
        
        events.extend(handler_events);
        
        // Check if we're in Loading state and all LSAs received
        let current_state = self.neighbor_states.get(&from_router).copied();
        if current_state == Some(NeighborState::Loading) {
            if self.lsu_handler.all_lsas_received(from_router) {
                self.neighbor_states.insert(from_router, NeighborState::Full);
                
                events.push(OSPFEvent::NeighborStateChanged {
                    router_id: u32::from_be_bytes(self.router_id.octets()),
                    neighbor_id: from_router,
                    from_state: NeighborState::Loading,
                    to_state: NeighborState::Full,
                    interface_id,
                });
            }
        }
        
        Ok(events)
    }
    
    /// Process Link State Acknowledgment packet
    fn process_lsack_packet(
        &mut self,
        packet: LinkStateAckPacket,
        from_router: u32,
        _interface_id: u32,
    ) -> Result<Vec<OSPFEvent>, PacketError> {
        // Process acknowledgments
        let events = self.lsack_handler.handle_lsack_packet(&packet, from_router)
            .map_err(|e| PacketError::ProcessingError(format!("LSAck handling failed: {:?}", e)))?;
        
        Ok(events)
    }
    
    /// Determine new neighbor state based on hello packet
    fn determine_new_neighbor_state(
        &self,
        current_state: NeighborState,
        is_bidirectional: bool,
    ) -> Result<NeighborState, PacketError> {
        match current_state {
            NeighborState::Down => Ok(NeighborState::Init),
            NeighborState::Init => {
                if is_bidirectional {
                    Ok(NeighborState::TwoWay)
                } else {
                    Ok(NeighborState::Init)
                }
            }
            _ => Ok(current_state), // No change for other states
        }
    }
    
    /// Get error metrics
    pub fn get_error_metrics(&self) -> &ErrorMetrics {
        &self.error_metrics
    }
    
    /// Get recovery history
    pub fn get_recovery_history(&self) -> Vec<serde_json::Value> {
        self.recovery_coordinator.get_history()
            .iter()
            .map(|entry| serde_json::json!({
                "timestamp": entry.timestamp,
                "error_type": entry.error_type,
                "action": format!("{:?}", entry.action),
                "success": entry.success,
            }))
            .collect()
    }
    
    /// Configure retry policy
    pub fn set_retry_config(&mut self, config: RetryConfig) {
        self.retry_config = config;
    }
    
    /// Get current retry configuration
    pub fn get_retry_config(&self) -> &RetryConfig {
        &self.retry_config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ospf_refactored::packets::OSPFHeader;
    
    fn create_test_processor() -> UnifiedPacketProcessor {
        let router_id = Ipv4Addr::new(1, 1, 1, 1);
        let area_id = Ipv4Addr::new(0, 0, 0, 0);
        let event_bus = Arc::new(EventBus::new());
        
        UnifiedPacketProcessor::new(router_id, area_id, event_bus)
    }
    
    fn create_test_hello() -> HelloPacket {
        HelloPacket::new(
            Ipv4Addr::new(2, 2, 2, 2),
            Ipv4Addr::new(0, 0, 0, 0),
            Ipv4Addr::new(255, 255, 255, 0),
            10,
            1,
            40,
        )
    }
    
    #[test]
    fn test_hello_processing() {
        let mut processor = create_test_processor();
        let hello = create_test_hello();
        let packet = OSPFPacket::Hello(hello);
        
        let events = processor.process_packet(packet, 2, 1).unwrap();
        
        // Should generate neighbor state change event
        assert!(!events.is_empty());
        
        // Verify state was updated
        assert_eq!(processor.neighbor_states.get(&2), Some(&NeighborState::Init));
    }
    
    #[test]
    fn test_bidirectional_hello() {
        let mut processor = create_test_processor();
        
        // First hello to establish Init
        let hello1 = create_test_hello();
        let packet1 = OSPFPacket::Hello(hello1);
        processor.process_packet(packet1, 2, 1).unwrap();
        
        // Second hello with our router ID (bidirectional)
        let mut hello2 = create_test_hello();
        hello2.add_neighbor(Ipv4Addr::new(1, 1, 1, 1));
        let packet2 = OSPFPacket::Hello(hello2);
        
        let events = processor.process_packet(packet2, 2, 1).unwrap();
        
        // Should transition to TwoWay
        assert_eq!(processor.neighbor_states.get(&2), Some(&NeighborState::TwoWay));
        
        // Should have state change event
        assert!(events.iter().any(|e| matches!(e, 
            OSPFEvent::NeighborStateChanged { to_state: NeighborState::TwoWay, .. }
        )));
    }
}
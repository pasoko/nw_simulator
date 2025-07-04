// OSPF Engine Event Adapter
//
// This module bridges the new event-driven architecture with the existing OSPF engine.
// It allows gradual migration while maintaining backward compatibility.

use crate::ospf_engine::OSPFEngine;
use crate::ospf_refactored::events::{OSPFEvent, EventBus, EventProcessor, EventResult, EventError};
use crate::ospf_refactored::events::{SPFTriggerReason, PacketType, TimerContext};
use crate::ospf_neighbor::OSPFNeighborState;
use crate::event_manager::PacketEvent;
use std::sync::{Arc, Mutex};

/// Adapter that connects the event system to the existing OSPF engine
pub struct OSPFEngineEventAdapter {
    engine: Arc<Mutex<OSPFEngine>>,
    event_bus: Arc<EventBus>,
}

impl OSPFEngineEventAdapter {
    /// Create a new event adapter
    pub fn new(engine: Arc<Mutex<OSPFEngine>>) -> Self {
        let adapter = Self {
            engine: engine.clone(),
            event_bus: Arc::new(EventBus::new()),
        };
        
        // Register event processors
        adapter.register_processors();
        
        adapter
    }
    
    /// Register all event processors
    fn register_processors(&self) {
        // Register neighbor state processor
        let neighbor_processor = Arc::new(Mutex::new(NeighborStateProcessor {
            engine: self.engine.clone(),
        }));
        self.event_bus.register_processor(neighbor_processor);
        
        // Register timer processor
        let timer_processor = Arc::new(Mutex::new(TimerEventProcessor {
            engine: self.engine.clone(),
        }));
        self.event_bus.register_processor(timer_processor);
        
        // Register packet processor
        let packet_processor = Arc::new(Mutex::new(PacketEventProcessor {
            engine: self.engine.clone(),
        }));
        self.event_bus.register_processor(packet_processor);
    }
    
    /// Convert traditional packet events to new event system
    pub fn convert_packet_events(&self, packet_events: Vec<PacketEvent>) -> Vec<OSPFEvent> {
        packet_events.into_iter()
            .filter_map(|pe| self.convert_packet_event(pe))
            .collect()
    }
    
    /// Convert a single packet event
    fn convert_packet_event(&self, packet_event: PacketEvent) -> Option<OSPFEvent> {
        match &packet_event.event_type {
            crate::event_manager::EventType::PacketSent { packet_type, .. } => {
                let packet_type = match packet_type.as_str() {
                    "Hello" => PacketType::Hello,
                    "DD" | "Database Description" => PacketType::DatabaseDescription,
                    "LS Request" => PacketType::LinkStateRequest,
                    "LS Update" => PacketType::LinkStateUpdate,
                    "LS Ack" => PacketType::LinkStateAck,
                    _ => return None,
                };
                
                Some(OSPFEvent::PacketSendRequired {
                    packet_type,
                    destination: packet_event.to_router,
                    interface_id: 0, // Would need to be determined
                })
            }
            _ => None,
        }
    }
    
    /// Process events through the event bus
    pub fn process_events(&self) -> Result<usize, EventError> {
        self.event_bus.process_events()
    }
    
    /// Publish an event to the bus
    pub fn publish_event(&self, event: OSPFEvent) -> Result<(), EventError> {
        self.event_bus.publish(event)
    }
}

/// Processor for neighbor state change events
struct NeighborStateProcessor {
    engine: Arc<Mutex<OSPFEngine>>,
}

impl EventProcessor for NeighborStateProcessor {
    fn process_event(&mut self, event: &OSPFEvent) -> EventResult {
        match event {
            OSPFEvent::NeighborStateChanged { 
                neighbor_id, 
                from_state, 
                to_state, 
                .. 
            } => {
                let mut engine = self.engine.lock().unwrap();
                
                // Log state change
                console_log!(
                    "Router {} neighbor {} state changed: {:?} → {:?}",
                    engine.router_id, neighbor_id, from_state, to_state
                );
                
                // Generate events based on state change
                let mut events = Vec::new();
                
                // If reached ExStart, might need to send DD packet
                if *to_state == crate::ospf_refactored::state::NeighborState::ExStart {
                    events.push(OSPFEvent::PacketSendRequired {
                        packet_type: PacketType::DatabaseDescription,
                        destination: *neighbor_id,
                        interface_id: 0, // Would be determined from neighbor
                    });
                }
                
                Ok(events)
            }
            _ => Ok(vec![]),
        }
    }
    
    fn handled_event_types(&self) -> Vec<&'static str> {
        vec!["NeighborStateChanged"]
    }
}

/// Processor for timer events
struct TimerEventProcessor {
    engine: Arc<Mutex<OSPFEngine>>,
}

impl EventProcessor for TimerEventProcessor {
    fn process_event(&mut self, event: &OSPFEvent) -> EventResult {
        match event {
            OSPFEvent::TimerExpired { timer_type, context } => {
                let mut engine = self.engine.lock().unwrap();
                
                // Convert to traditional timer handling
                use crate::ospf_timer::OSPFTimerEvent;
                use crate::ospf_refactored::events::TimerType;
                
                let timer_event = match timer_type {
                    TimerType::Hello => OSPFTimerEvent::HelloTimer,
                    TimerType::Dead => {
                        if let Some(neighbor_id) = context.neighbor_id {
                            OSPFTimerEvent::DeadTimer(neighbor_id)
                        } else {
                            return Ok(vec![]);
                        }
                    }
                    TimerType::Retransmit => {
                        if let Some(neighbor_id) = context.neighbor_id {
                            OSPFTimerEvent::RetransmissionTimer(neighbor_id)
                        } else {
                            return Ok(vec![]);
                        }
                    }
                    TimerType::LSRefresh => OSPFTimerEvent::LSARefresh,
                    TimerType::SPFDelay => OSPFTimerEvent::SPFDelay,
                    TimerType::Acknowledgment => return Ok(vec![]), // Not directly mapped
                };
                
                // Process through existing timer handling
                // This would trigger the appropriate action in the engine
                
                Ok(vec![])
            }
            _ => Ok(vec![]),
        }
    }
    
    fn handled_event_types(&self) -> Vec<&'static str> {
        vec!["TimerExpired"]
    }
}

/// Processor for packet send events
struct PacketEventProcessor {
    engine: Arc<Mutex<OSPFEngine>>,
}

impl EventProcessor for PacketEventProcessor {
    fn process_event(&mut self, event: &OSPFEvent) -> EventResult {
        match event {
            OSPFEvent::PacketSendRequired { 
                packet_type, 
                destination, 
                .. 
            } => {
                let engine = self.engine.lock().unwrap();
                
                console_log!(
                    "Router {} processing packet send event: {:?} to {}",
                    engine.router_id, packet_type, destination
                );
                
                // In a full implementation, this would trigger actual packet creation
                
                Ok(vec![])
            }
            _ => Ok(vec![]),
        }
    }
    
    fn handled_event_types(&self) -> Vec<&'static str> {
        vec!["PacketSendRequired"]
    }
}

// Logging macro
macro_rules! console_log {
    ($($t:tt)*) => {
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&format!($($t)*).into());
        
        #[cfg(not(target_arch = "wasm32"))]
        println!($($t)*);
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_event_adapter_creation() {
        // Would need mock OSPFEngine for testing
        // let engine = Arc::new(Mutex::new(create_mock_engine()));
        // let adapter = OSPFEngineEventAdapter::new(engine);
        // assert_eq!(adapter.event_bus.queue_size(), 0);
    }
}
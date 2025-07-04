// Event Bus Implementation
//
// A central event bus that manages event distribution to registered handlers.
// This implements a pub-sub pattern for loose coupling between components.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, RwLock};
use super::{OSPFEvent, EventResult, EventError, EventProcessor};

/// Event handler function type
pub type EventHandlerFn = Box<dyn Fn(&OSPFEvent) -> EventResult + Send + Sync>;

/// Event bus for distributing OSPF events to handlers
pub struct EventBus {
    /// Registered event processors
    processors: RwLock<Vec<Arc<Mutex<dyn EventProcessor>>>>,
    
    /// Event queue for processing
    event_queue: Mutex<VecDeque<OSPFEvent>>,
    
    /// Maximum events to process in one cycle (prevents infinite loops)
    max_events_per_cycle: usize,
    
    /// Event history for debugging
    #[cfg(debug_assertions)]
    event_history: Mutex<VecDeque<OSPFEvent>>,
}

impl EventBus {
    /// Create a new event bus
    pub fn new() -> Self {
        Self {
            processors: RwLock::new(Vec::new()),
            event_queue: Mutex::new(VecDeque::new()),
            max_events_per_cycle: 1000,
            #[cfg(debug_assertions)]
            event_history: Mutex::new(VecDeque::with_capacity(100)),
        }
    }
    
    /// Register an event processor
    pub fn register_processor(&self, processor: Arc<Mutex<dyn EventProcessor>>) {
        let mut processors = self.processors.write().unwrap();
        processors.push(processor);
    }
    
    /// Publish an event to the bus
    pub fn publish(&self, event: OSPFEvent) -> Result<(), EventError> {
        let mut queue = self.event_queue.lock().unwrap();
        
        // Check queue size to prevent memory issues
        if queue.len() > self.max_events_per_cycle {
            return Err(EventError::EventLoopDetected);
        }
        
        queue.push_back(event.clone());
        
        #[cfg(debug_assertions)]
        {
            let mut history = self.event_history.lock().unwrap();
            history.push_back(event);
            if history.len() > 100 {
                history.pop_front();
            }
        }
        
        Ok(())
    }
    
    /// Process all pending events
    pub fn process_events(&self) -> Result<usize, EventError> {
        let mut processed_count = 0;
        let mut cycle_count = 0;
        
        loop {
            // Get next event from queue
            let event = {
                let mut queue = self.event_queue.lock().unwrap();
                queue.pop_front()
            };
            
            let Some(event) = event else {
                break; // No more events
            };
            
            // Process event with all registered processors
            let processors = self.processors.read().unwrap();
            for processor in processors.iter() {
                let mut proc = processor.lock().unwrap();
                
                // Check if this processor handles this event type
                let event_type = event_type_name(&event);
                if proc.handled_event_types().contains(&event_type) {
                    match proc.process_event(&event) {
                        Ok(new_events) => {
                            // Add new events to queue
                            for new_event in new_events {
                                self.publish(new_event)?;
                            }
                        }
                        Err(e) => {
                            // TODO: Add proper error logging
                        // console_log!("Event processing error: {:?}", e);
                            // Continue processing other events
                        }
                    }
                }
            }
            
            processed_count += 1;
            cycle_count += 1;
            
            // Prevent infinite loops
            if cycle_count > self.max_events_per_cycle {
                // TODO: Add proper warning logging
                // console_log!("Event processing cycle limit reached");
                break;
            }
        }
        
        Ok(processed_count)
    }
    
    /// Get the current queue size
    pub fn queue_size(&self) -> usize {
        self.event_queue.lock().unwrap().len()
    }
    
    /// Clear all pending events
    pub fn clear_queue(&self) {
        self.event_queue.lock().unwrap().clear();
    }
    
    #[cfg(debug_assertions)]
    /// Get event history for debugging
    pub fn get_event_history(&self) -> Vec<OSPFEvent> {
        self.event_history.lock().unwrap().iter().cloned().collect()
    }
}

/// Get the type name of an event for handler matching
fn event_type_name(event: &OSPFEvent) -> &'static str {
    match event {
        OSPFEvent::NeighborStateChanged { .. } => "NeighborStateChanged",
        OSPFEvent::DRElectionRequired { .. } => "DRElectionRequired",
        OSPFEvent::LSAReceived { .. } => "LSAReceived",
        OSPFEvent::TimerExpired { .. } => "TimerExpired",
        OSPFEvent::SPFCalculationRequired { .. } => "SPFCalculationRequired",
        OSPFEvent::PacketSendRequired { .. } => "PacketSendRequired",
        OSPFEvent::InterfaceStateChanged { .. } => "InterfaceStateChanged",
        OSPFEvent::LSAFloodRequired { .. } => "LSAFloodRequired",
        OSPFEvent::SPFRequired { .. } => "SPFRequired",
        OSPFEvent::LSAAcknowledged { .. } => "LSAAcknowledged",
        OSPFEvent::AllLSAsAcknowledged { .. } => "AllLSAsAcknowledged",
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ospf::NeighborState;
    
    struct TestProcessor {
        handled_types: Vec<&'static str>,
        events_received: Arc<Mutex<Vec<OSPFEvent>>>,
    }
    
    impl EventProcessor for TestProcessor {
        fn process_event(&mut self, event: &OSPFEvent) -> EventResult {
            self.events_received.lock().unwrap().push(event.clone());
            Ok(vec![])
        }
        
        fn handled_event_types(&self) -> Vec<&'static str> {
            self.handled_types.clone()
        }
    }
    
    #[test]
    fn test_event_bus_basic() {
        let bus = EventBus::new();
        
        let events_received = Arc::new(Mutex::new(Vec::new()));
        let processor = Arc::new(Mutex::new(TestProcessor {
            handled_types: vec!["NeighborStateChanged"],
            events_received: events_received.clone(),
        }));
        
        bus.register_processor(processor);
        
        let event = OSPFEvent::NeighborStateChanged {
            router_id: 1,
            neighbor_id: 2,
            from_state: NeighborState::Down,
            to_state: NeighborState::Init,
            interface_id: 1,
        };
        
        bus.publish(event.clone()).unwrap();
        assert_eq!(bus.queue_size(), 1);
        
        let processed = bus.process_events().unwrap();
        assert_eq!(processed, 1);
        assert_eq!(bus.queue_size(), 0);
        
        let received = events_received.lock().unwrap();
        assert_eq!(received.len(), 1);
    }
    
    #[test]
    fn test_event_filtering() {
        let bus = EventBus::new();
        
        let events_received = Arc::new(Mutex::new(Vec::new()));
        let processor = Arc::new(Mutex::new(TestProcessor {
            handled_types: vec!["TimerExpired"], // Only handles timer events
            events_received: events_received.clone(),
        }));
        
        bus.register_processor(processor);
        
        // Publish an event that won't be handled
        let event = OSPFEvent::NeighborStateChanged {
            router_id: 1,
            neighbor_id: 2,
            from_state: NeighborState::Down,
            to_state: NeighborState::Init,
            interface_id: 1,
        };
        
        bus.publish(event).unwrap();
        bus.process_events().unwrap();
        
        let received = events_received.lock().unwrap();
        assert_eq!(received.len(), 0); // Should not receive the event
    }
}
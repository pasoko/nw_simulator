// Event Handler Traits and Base Implementations
//
// This module defines the traits and base implementations for event handlers
// in the OSPF event system.

use super::{OSPFEvent, EventResult, EventError};
use std::sync::{Arc, Mutex};

/// Trait for components that can handle OSPF events
pub trait EventHandler: Send + Sync {
    /// Handle an event and return any new events that should be published
    fn handle(&mut self, event: &OSPFEvent) -> EventResult;
    
    /// Get a unique identifier for this handler
    fn id(&self) -> &str;
    
    /// Check if this handler should process the given event
    fn should_handle(&self, event: &OSPFEvent) -> bool;
}

/// A composite handler that delegates to multiple sub-handlers
pub struct CompositeEventHandler {
    handlers: Vec<Arc<Mutex<dyn EventHandler>>>,
    id: String,
}

impl CompositeEventHandler {
    pub fn new(id: String) -> Self {
        Self {
            handlers: Vec::new(),
            id,
        }
    }
    
    pub fn add_handler(&mut self, handler: Arc<Mutex<dyn EventHandler>>) {
        self.handlers.push(handler);
    }
}

impl EventHandler for CompositeEventHandler {
    fn handle(&mut self, event: &OSPFEvent) -> EventResult {
        let mut all_events = Vec::new();
        
        for handler in &self.handlers {
            let mut h = handler.lock().unwrap();
            if h.should_handle(event) {
                match h.handle(event) {
                    Ok(events) => all_events.extend(events),
                    Err(e) => {
                        // TODO: Add proper error logging
                        // console_log!("Handler {} failed: {:?}", h.id(), e);
                        // Continue with other handlers
                    }
                }
            }
        }
        
        Ok(all_events)
    }
    
    fn id(&self) -> &str {
        &self.id
    }
    
    fn should_handle(&self, event: &OSPFEvent) -> bool {
        // Composite handler checks if any sub-handler can handle
        for handler in &self.handlers {
            let h = handler.lock().unwrap();
            if h.should_handle(event) {
                return true;
            }
        }
        false
    }
}

/// A logging event handler for debugging
pub struct LoggingEventHandler {
    id: String,
    // TODO: Add proper logging support
    // log_level: log::Level,
}

impl LoggingEventHandler {
    pub fn new(id: String) -> Self {
        Self { id }
    }
}

impl EventHandler for LoggingEventHandler {
    fn handle(&mut self, event: &OSPFEvent) -> EventResult {
        // TODO: Add proper logging with level support
        // console_log!("[{}] Event: {:?}", self.id, event);
        Ok(vec![]) // Logging handler doesn't generate new events
    }
    
    fn id(&self) -> &str {
        &self.id
    }
    
    fn should_handle(&self, _event: &OSPFEvent) -> bool {
        true // Log all events
    }
}

/// A filtering event handler that only processes specific event types
pub struct FilteringEventHandler<H: EventHandler> {
    inner: H,
    event_types: Vec<String>,
}

impl<H: EventHandler> FilteringEventHandler<H> {
    pub fn new(inner: H, event_types: Vec<&str>) -> Self {
        Self {
            inner,
            event_types: event_types.into_iter().map(String::from).collect(),
        }
    }
}

impl<H: EventHandler> EventHandler for FilteringEventHandler<H> {
    fn handle(&mut self, event: &OSPFEvent) -> EventResult {
        self.inner.handle(event)
    }
    
    fn id(&self) -> &str {
        self.inner.id()
    }
    
    fn should_handle(&self, event: &OSPFEvent) -> bool {
        let event_type = match event {
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
        };
        
        self.event_types.iter().any(|t| t == event_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ospf::NeighborState;
    
    struct TestHandler {
        id: String,
        handled_count: usize,
    }
    
    impl EventHandler for TestHandler {
        fn handle(&mut self, _event: &OSPFEvent) -> EventResult {
            self.handled_count += 1;
            Ok(vec![])
        }
        
        fn id(&self) -> &str {
            &self.id
        }
        
        fn should_handle(&self, _event: &OSPFEvent) -> bool {
            true
        }
    }
    
    #[test]
    fn test_composite_handler() {
        let mut composite = CompositeEventHandler::new("composite".to_string());
        
        let handler1 = Arc::new(Mutex::new(TestHandler {
            id: "handler1".to_string(),
            handled_count: 0,
        }));
        
        let handler2 = Arc::new(Mutex::new(TestHandler {
            id: "handler2".to_string(),
            handled_count: 0,
        }));
        
        composite.add_handler(handler1.clone());
        composite.add_handler(handler2.clone());
        
        let event = OSPFEvent::NeighborStateChanged {
            router_id: 1,
            neighbor_id: 2,
            from_state: NeighborState::Down,
            to_state: NeighborState::Init,
            interface_id: 1,
        };
        
        composite.handle(&event).unwrap();
        
        assert_eq!(handler1.lock().unwrap().handled_count, 1);
        assert_eq!(handler2.lock().unwrap().handled_count, 1);
    }
    
    #[test]
    fn test_filtering_handler() {
        let inner = TestHandler {
            id: "test".to_string(),
            handled_count: 0,
        };
        
        let mut filter = FilteringEventHandler::new(inner, vec!["NeighborStateChanged"]);
        
        let event1 = OSPFEvent::NeighborStateChanged {
            router_id: 1,
            neighbor_id: 2,
            from_state: NeighborState::Down,
            to_state: NeighborState::Init,
            interface_id: 1,
        };
        
        let event2 = OSPFEvent::DRElectionRequired {
            interface_id: 1,
            priority_changed: false,
        };
        
        assert!(filter.should_handle(&event1));
        assert!(!filter.should_handle(&event2));
    }
}
// OSPF Event System
//
// This module implements an event-driven architecture for OSPF protocol handling.
// It decouples components and makes the system more modular and testable.

use serde::{Serialize, Deserialize};

pub mod event_bus;
pub mod handlers;

pub use event_bus::EventBus;
pub use handlers::EventHandler;

use crate::ospf_refactored::state::NeighborState;
// TODO: Import TimerType when ospf_engine module is refactored
// use crate::ospf_engine::TimerType;

// Type alias for router ID (IPv4 address as u32)
type RouterId = u32;

/// Timer types used in OSPF
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimerType {
    Hello,
    Dead,
    Wait,
    RetransmitDD,
    RetransmitRequest,
    RetransmitUpdate,
    Acknowledgment,
}

/// Core OSPF events that can occur in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OSPFEvent {
    /// Neighbor state has changed
    NeighborStateChanged {
        router_id: RouterId,
        neighbor_id: RouterId,
        from_state: NeighborState,
        to_state: NeighborState,
        interface_id: u32,
    },
    
    /// DR/BDR election is required on an interface
    DRElectionRequired {
        interface_id: u32,
        priority_changed: bool,
    },
    
    /// LSA has been received and needs processing
    LSAReceived {
        lsa_type: u8,
        lsa_id: u32,
        advertising_router: RouterId,
        from_neighbor: RouterId,
        sequence_number: u32,
    },
    
    /// Timer has expired and needs handling
    TimerExpired {
        timer_type: TimerType,
        context: TimerContext,
    },
    
    /// SPF calculation is required
    SPFCalculationRequired {
        reason: SPFTriggerReason,
        delay_ms: u32,
    },
    
    /// Packet needs to be sent
    PacketSendRequired {
        packet_type: PacketType,
        destination: RouterId,
        interface_id: u32,
        additional_data: Option<String>,
    },
    
    /// Interface state has changed
    InterfaceStateChanged {
        interface_id: u32,
        new_state: InterfaceState,
        old_state: InterfaceState,
    },
    
    /// LSA needs to be flooded
    LSAFloodRequired {
        lsa_key: String,
        exclude_interface: Option<u32>,
        exclude_neighbor: Option<RouterId>,
    },
    
    /// SPF calculation is required
    SPFRequired {
        area_id: u32,
        reason: String,
    },
    
    /// LSA has been acknowledged
    LSAAcknowledged {
        lsa_type: u8,
        lsa_id: u32,
        advertising_router: RouterId,
        from_neighbor: RouterId,
    },
    
    /// All LSAs have been acknowledged by a neighbor
    AllLSAsAcknowledged {
        neighbor_id: RouterId,
    },
}

/// Context information for timer events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerContext {
    pub interface_id: Option<u32>,
    pub neighbor_id: Option<RouterId>,
    pub lsa_key: Option<String>,
    pub additional_data: Option<String>,
}

/// Reasons that can trigger SPF calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SPFTriggerReason {
    LSAChange,
    TopologyChange,
    InitialCalculation,
    ManualTrigger,
}

/// Types of packets that can be sent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PacketType {
    Hello,
    DatabaseDescription,
    LinkStateRequest,
    LinkStateUpdate,
    LinkStateAck,
}

/// Interface states in OSPF
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterfaceState {
    Down,
    Loopback,
    Waiting,
    PointToPoint,
    DROther,
    Backup,
    DR,
}

/// Result type for event handling
pub type EventResult = Result<Vec<OSPFEvent>, EventError>;

/// Errors that can occur during event processing
#[derive(Debug, Clone)]
pub enum EventError {
    HandlerNotFound,
    ProcessingError(String),
    InvalidEventData(String),
    EventLoopDetected,
}

impl std::fmt::Display for EventError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventError::HandlerNotFound => write!(f, "Handler not found for event type"),
            EventError::ProcessingError(msg) => write!(f, "Event processing failed: {}", msg),
            EventError::InvalidEventData(msg) => write!(f, "Invalid event data: {}", msg),
            EventError::EventLoopDetected => write!(f, "Event loop detected"),
        }
    }
}

impl std::error::Error for EventError {}

/// Trait for components that can process events
pub trait EventProcessor: Send + Sync {
    /// Process a single event and return any new events generated
    fn process_event(&mut self, event: &OSPFEvent) -> EventResult;
    
    /// Get the types of events this processor handles
    fn handled_event_types(&self) -> Vec<&'static str>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let event = OSPFEvent::NeighborStateChanged {
            router_id: 1,
            neighbor_id: 2,
            from_state: NeighborState::Down,
            to_state: NeighborState::Init,
            interface_id: 1,
        };
        
        match event {
            OSPFEvent::NeighborStateChanged { neighbor_id, .. } => {
                assert_eq!(neighbor_id, 2);
            }
            _ => panic!("Wrong event type"),
        }
    }
    
    #[test]
    fn test_timer_context() {
        let ctx = TimerContext {
            interface_id: Some(1),
            neighbor_id: Some(2),
            lsa_key: None,
            additional_data: None,
        };
        
        assert_eq!(ctx.interface_id, Some(1));
        assert_eq!(ctx.neighbor_id, Some(2));
        assert!(ctx.lsa_key.is_none());
    }
}
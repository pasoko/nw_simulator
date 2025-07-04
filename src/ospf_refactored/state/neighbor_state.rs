// OSPF Neighbor State Machine
//
// Implements the neighbor state machine as defined in RFC 2328.
// Uses the State pattern for clean, maintainable state transitions.

use super::StateContext;
use crate::ospf_refactored::events::OSPFEvent;
use std::fmt;
use serde::{Serialize, Deserialize};

/// OSPF Neighbor States (RFC 2328 Section 10.1)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NeighborState {
    /// Initial state - no recent hello packets received
    Down,
    
    /// Hello packet received, but bidirectional communication not established
    Init,
    
    /// Bidirectional communication established
    TwoWay,
    
    /// Beginning database synchronization
    ExStart,
    
    /// Database description packets are being exchanged
    Exchange,
    
    /// Link state requests are being sent
    Loading,
    
    /// Neighbors are fully synchronized
    Full,
}

impl fmt::Display for NeighborState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NeighborState::Down => write!(f, "Down"),
            NeighborState::Init => write!(f, "Init"),
            NeighborState::TwoWay => write!(f, "TwoWay"),
            NeighborState::ExStart => write!(f, "ExStart"),
            NeighborState::Exchange => write!(f, "Exchange"),
            NeighborState::Loading => write!(f, "Loading"),
            NeighborState::Full => write!(f, "Full"),
        }
    }
}

/// State transition result
#[derive(Debug, Clone)]
pub enum StateTransition {
    /// No state change
    None,
    
    /// Transition to a new state
    To(NeighborState),
    
    /// Transition with generated events
    ToWithEvents(NeighborState, Vec<OSPFEvent>),
}

/// Trait for neighbor state handlers
pub trait NeighborStateHandler {
    /// Get the state this handler represents
    fn state(&self) -> NeighborState;
    
    /// Handle a hello packet reception
    fn on_hello_received(&self, ctx: &mut StateContext, bidirectional: bool) -> StateTransition;
    
    /// Handle database description packet
    fn on_dd_received(&self, ctx: &mut StateContext) -> StateTransition;
    
    /// Handle neighbor inactivity timer expiration
    fn on_inactivity_timer(&self, ctx: &mut StateContext) -> StateTransition;
    
    /// Handle adjacency requirement check
    fn on_adjacency_required(&self, ctx: &mut StateContext) -> StateTransition;
    
    /// Called when entering this state
    fn on_enter(&self, ctx: &mut StateContext) -> Vec<OSPFEvent>;
    
    /// Called when exiting this state
    fn on_exit(&self, ctx: &mut StateContext) -> Vec<OSPFEvent>;
}

/// Down state handler
pub struct DownStateHandler;

impl NeighborStateHandler for DownStateHandler {
    fn state(&self) -> NeighborState {
        NeighborState::Down
    }
    
    fn on_hello_received(&self, _ctx: &mut StateContext, _bidirectional: bool) -> StateTransition {
        // Any hello packet moves us to Init
        StateTransition::To(NeighborState::Init)
    }
    
    fn on_dd_received(&self, _ctx: &mut StateContext) -> StateTransition {
        // DD packets are ignored in Down state
        StateTransition::None
    }
    
    fn on_inactivity_timer(&self, _ctx: &mut StateContext) -> StateTransition {
        // Already in Down state
        StateTransition::None
    }
    
    fn on_adjacency_required(&self, _ctx: &mut StateContext) -> StateTransition {
        // Cannot form adjacency from Down state
        StateTransition::None
    }
    
    fn on_enter(&self, _ctx: &mut StateContext) -> Vec<OSPFEvent> {
        vec![]
    }
    
    fn on_exit(&self, _ctx: &mut StateContext) -> Vec<OSPFEvent> {
        vec![]
    }
}

/// Init state handler
pub struct InitStateHandler;

impl NeighborStateHandler for InitStateHandler {
    fn state(&self) -> NeighborState {
        NeighborState::Init
    }
    
    fn on_hello_received(&self, _ctx: &mut StateContext, bidirectional: bool) -> StateTransition {
        if bidirectional {
            // Bidirectional communication established
            StateTransition::To(NeighborState::TwoWay)
        } else {
            // Stay in Init
            StateTransition::None
        }
    }
    
    fn on_dd_received(&self, _ctx: &mut StateContext) -> StateTransition {
        // DD packets are ignored until TwoWay
        StateTransition::None
    }
    
    fn on_inactivity_timer(&self, _ctx: &mut StateContext) -> StateTransition {
        // Neighbor is considered dead
        StateTransition::To(NeighborState::Down)
    }
    
    fn on_adjacency_required(&self, _ctx: &mut StateContext) -> StateTransition {
        // Must reach TwoWay first
        StateTransition::None
    }
    
    fn on_enter(&self, _ctx: &mut StateContext) -> Vec<OSPFEvent> {
        vec![]
    }
    
    fn on_exit(&self, _ctx: &mut StateContext) -> Vec<OSPFEvent> {
        vec![]
    }
}

/// TwoWay state handler
pub struct TwoWayStateHandler;

impl NeighborStateHandler for TwoWayStateHandler {
    fn state(&self) -> NeighborState {
        NeighborState::TwoWay
    }
    
    fn on_hello_received(&self, _ctx: &mut StateContext, bidirectional: bool) -> StateTransition {
        if !bidirectional {
            // Lost bidirectional communication
            StateTransition::To(NeighborState::Init)
        } else {
            StateTransition::None
        }
    }
    
    fn on_dd_received(&self, _ctx: &mut StateContext) -> StateTransition {
        // Unexpected DD packet, might be from previous session
        StateTransition::None
    }
    
    fn on_inactivity_timer(&self, _ctx: &mut StateContext) -> StateTransition {
        StateTransition::To(NeighborState::Down)
    }
    
    fn on_adjacency_required(&self, ctx: &mut StateContext) -> StateTransition {
        // Start forming adjacency
        let events = vec![
            OSPFEvent::PacketSendRequired {
                packet_type: crate::ospf_refactored::events::PacketType::DatabaseDescription,
                destination: ctx.router_id,
                interface_id: ctx.interface_id,
                additional_data: None,
            }
        ];
        
        StateTransition::ToWithEvents(NeighborState::ExStart, events)
    }
    
    fn on_enter(&self, _ctx: &mut StateContext) -> Vec<OSPFEvent> {
        vec![]
    }
    
    fn on_exit(&self, _ctx: &mut StateContext) -> Vec<OSPFEvent> {
        vec![]
    }
}

/// ExStart state handler
pub struct ExStartStateHandler;

impl NeighborStateHandler for ExStartStateHandler {
    fn state(&self) -> NeighborState {
        NeighborState::ExStart
    }
    
    fn on_hello_received(&self, _ctx: &mut StateContext, bidirectional: bool) -> StateTransition {
        if !bidirectional {
            StateTransition::To(NeighborState::Init)
        } else {
            StateTransition::None
        }
    }
    
    fn on_dd_received(&self, _ctx: &mut StateContext) -> StateTransition {
        // Master/Slave negotiation successful
        StateTransition::To(NeighborState::Exchange)
    }
    
    fn on_inactivity_timer(&self, _ctx: &mut StateContext) -> StateTransition {
        StateTransition::To(NeighborState::Down)
    }
    
    fn on_adjacency_required(&self, _ctx: &mut StateContext) -> StateTransition {
        // Already forming adjacency
        StateTransition::None
    }
    
    fn on_enter(&self, _ctx: &mut StateContext) -> Vec<OSPFEvent> {
        vec![]
    }
    
    fn on_exit(&self, _ctx: &mut StateContext) -> Vec<OSPFEvent> {
        vec![]
    }
}

/// Exchange state handler
pub struct ExchangeStateHandler;

impl NeighborStateHandler for ExchangeStateHandler {
    fn state(&self) -> NeighborState {
        NeighborState::Exchange
    }
    
    fn on_hello_received(&self, _ctx: &mut StateContext, bidirectional: bool) -> StateTransition {
        if !bidirectional {
            // Lost bidirectional communication
            StateTransition::To(NeighborState::Init)
        } else {
            StateTransition::None
        }
    }
    
    fn on_dd_received(&self, ctx: &mut StateContext) -> StateTransition {
        // Check if DD exchange is complete
        // In real implementation, would check if all LSAs have been described
        // For now, we'll use a simple time-based check
        if ctx.current_time > 0.0 {
            StateTransition::To(NeighborState::Loading)
        } else {
            StateTransition::None
        }
    }
    
    fn on_inactivity_timer(&self, _ctx: &mut StateContext) -> StateTransition {
        StateTransition::To(NeighborState::Down)
    }
    
    fn on_adjacency_required(&self, _ctx: &mut StateContext) -> StateTransition {
        // Already in adjacency formation
        StateTransition::None
    }
    
    fn on_enter(&self, ctx: &mut StateContext) -> Vec<OSPFEvent> {
        // Start DD exchange
        vec![
            OSPFEvent::PacketSendRequired {
                packet_type: crate::ospf_refactored::events::PacketType::DatabaseDescription,
                destination: ctx.router_id,
                interface_id: ctx.interface_id,
                additional_data: None,
            }
        ]
    }
    
    fn on_exit(&self, _ctx: &mut StateContext) -> Vec<OSPFEvent> {
        vec![]
    }
}

/// Loading state handler
pub struct LoadingStateHandler;

impl NeighborStateHandler for LoadingStateHandler {
    fn state(&self) -> NeighborState {
        NeighborState::Loading
    }
    
    fn on_hello_received(&self, _ctx: &mut StateContext, bidirectional: bool) -> StateTransition {
        if !bidirectional {
            StateTransition::To(NeighborState::Init)
        } else {
            StateTransition::None
        }
    }
    
    fn on_dd_received(&self, _ctx: &mut StateContext) -> StateTransition {
        // Unexpected DD in Loading state - might be retransmission
        StateTransition::None
    }
    
    fn on_inactivity_timer(&self, _ctx: &mut StateContext) -> StateTransition {
        StateTransition::To(NeighborState::Down)
    }
    
    fn on_adjacency_required(&self, _ctx: &mut StateContext) -> StateTransition {
        // Already in adjacency formation
        StateTransition::None
    }
    
    fn on_enter(&self, ctx: &mut StateContext) -> Vec<OSPFEvent> {
        // Start requesting LSAs
        vec![
            OSPFEvent::PacketSendRequired {
                packet_type: crate::ospf_refactored::events::PacketType::LinkStateRequest,
                destination: ctx.router_id,
                interface_id: ctx.interface_id,
                additional_data: None,
            }
        ]
    }
    
    fn on_exit(&self, _ctx: &mut StateContext) -> Vec<OSPFEvent> {
        vec![]
    }
}

/// Full state handler - adjacency fully established
pub struct FullStateHandler;

impl NeighborStateHandler for FullStateHandler {
    fn state(&self) -> NeighborState {
        NeighborState::Full
    }
    
    fn on_hello_received(&self, _ctx: &mut StateContext, bidirectional: bool) -> StateTransition {
        if !bidirectional {
            // Lost bidirectional communication - restart
            StateTransition::To(NeighborState::Init)
        } else {
            StateTransition::None
        }
    }
    
    fn on_dd_received(&self, _ctx: &mut StateContext) -> StateTransition {
        // Unexpected DD in Full state - might indicate neighbor restart
        // In real implementation, would check sequence numbers
        StateTransition::None
    }
    
    fn on_inactivity_timer(&self, _ctx: &mut StateContext) -> StateTransition {
        StateTransition::To(NeighborState::Down)
    }
    
    fn on_adjacency_required(&self, _ctx: &mut StateContext) -> StateTransition {
        // Already fully adjacent
        StateTransition::None
    }
    
    fn on_enter(&self, _ctx: &mut StateContext) -> Vec<OSPFEvent> {
        // Adjacency fully established - trigger SPF calculation
        vec![
            OSPFEvent::SPFCalculationRequired {
                reason: crate::ospf_refactored::events::SPFTriggerReason::TopologyChange,
                delay_ms: 5000, // 5 second delay
            }
        ]
    }
    
    fn on_exit(&self, _ctx: &mut StateContext) -> Vec<OSPFEvent> {
        // Adjacency lost - trigger SPF recalculation
        vec![
            OSPFEvent::SPFCalculationRequired {
                reason: crate::ospf_refactored::events::SPFTriggerReason::TopologyChange,
                delay_ms: 5000,
            }
        ]
    }
}

/// Factory function to get state handler
pub fn get_state_handler(state: NeighborState) -> Box<dyn NeighborStateHandler> {
    match state {
        NeighborState::Down => Box::new(DownStateHandler),
        NeighborState::Init => Box::new(InitStateHandler),
        NeighborState::TwoWay => Box::new(TwoWayStateHandler),
        NeighborState::ExStart => Box::new(ExStartStateHandler),
        NeighborState::Exchange => Box::new(ExchangeStateHandler),
        NeighborState::Loading => Box::new(LoadingStateHandler),
        NeighborState::Full => Box::new(FullStateHandler),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_down_to_init_transition() {
        let handler = DownStateHandler;
        let mut ctx = StateContext {
            router_id: 1,
            interface_id: 1,
            current_time: 0.0,
            area_id: 0,
        };
        
        let transition = handler.on_hello_received(&mut ctx, false);
        match transition {
            StateTransition::To(new_state) => {
                assert_eq!(new_state, NeighborState::Init);
            }
            _ => panic!("Expected state transition to Init"),
        }
    }
    
    #[test]
    fn test_init_to_twoway_transition() {
        let handler = InitStateHandler;
        let mut ctx = StateContext {
            router_id: 1,
            interface_id: 1,
            current_time: 0.0,
            area_id: 0,
        };
        
        // Without bidirectional communication
        let transition = handler.on_hello_received(&mut ctx, false);
        assert!(matches!(transition, StateTransition::None));
        
        // With bidirectional communication
        let transition = handler.on_hello_received(&mut ctx, true);
        match transition {
            StateTransition::To(new_state) => {
                assert_eq!(new_state, NeighborState::TwoWay);
            }
            _ => panic!("Expected state transition to TwoWay"),
        }
    }
    
    #[test]
    fn test_state_display() {
        assert_eq!(format!("{}", NeighborState::Down), "Down");
        assert_eq!(format!("{}", NeighborState::Init), "Init");
        assert_eq!(format!("{}", NeighborState::Full), "Full");
    }
}
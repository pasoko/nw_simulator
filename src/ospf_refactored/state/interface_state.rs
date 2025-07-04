// OSPF Interface State Machine
//
// Implements the interface state machine as defined in RFC 2328 Section 9.
// This manages the state of OSPF interfaces and DR/BDR election.

use super::{StateContext, StateResult, StateError};
use crate::ospf_refactored::events::OSPFEvent;
use crate::network_type::OSPFNetworkType as NetworkType;
use std::fmt;

/// OSPF Interface States (RFC 2328 Section 9.1)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InterfaceState {
    /// Interface is down
    Down,
    
    /// Interface is looped back
    Loopback,
    
    /// Waiting for DR/BDR election
    Waiting,
    
    /// Point-to-point interface (no DR election)
    PointToPoint,
    
    /// Interface is neither DR nor BDR
    DROther,
    
    /// Interface is Backup DR
    Backup,
    
    /// Interface is Designated Router
    DR,
}

impl fmt::Display for InterfaceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InterfaceState::Down => write!(f, "Down"),
            InterfaceState::Loopback => write!(f, "Loopback"),
            InterfaceState::Waiting => write!(f, "Waiting"),
            InterfaceState::PointToPoint => write!(f, "PointToPoint"),
            InterfaceState::DROther => write!(f, "DROther"),
            InterfaceState::Backup => write!(f, "Backup"),
            InterfaceState::DR => write!(f, "DR"),
        }
    }
}

/// Interface state transition result
#[derive(Debug, Clone)]
pub enum InterfaceTransition {
    /// No state change
    None,
    
    /// Transition to a new state
    To(InterfaceState),
    
    /// Transition with generated events
    ToWithEvents(InterfaceState, Vec<OSPFEvent>),
}

/// Interface state handler trait
pub trait InterfaceStateHandler {
    /// Get the state this handler represents
    fn state(&self) -> InterfaceState;
    
    /// Handle interface up event
    fn on_interface_up(&self, ctx: &mut InterfaceContext) -> InterfaceTransition;
    
    /// Handle interface down event
    fn on_interface_down(&self, ctx: &mut InterfaceContext) -> InterfaceTransition;
    
    /// Handle waiting timer expiration
    fn on_wait_timer(&self, ctx: &mut InterfaceContext) -> InterfaceTransition;
    
    /// Handle neighbor change event
    fn on_neighbor_change(&self, ctx: &mut InterfaceContext) -> InterfaceTransition;
    
    /// Handle loop indication
    fn on_loop_ind(&self, ctx: &mut InterfaceContext) -> InterfaceTransition;
    
    /// Handle unloop indication
    fn on_unloop_ind(&self, ctx: &mut InterfaceContext) -> InterfaceTransition;
    
    /// Called when entering this state
    fn on_enter(&self, ctx: &mut InterfaceContext) -> Vec<OSPFEvent>;
    
    /// Called when exiting this state
    fn on_exit(&self, ctx: &mut InterfaceContext) -> Vec<OSPFEvent>;
}

/// Context for interface state machine
#[derive(Debug, Clone)]
pub struct InterfaceContext {
    pub interface_id: u32,
    pub network_type: NetworkType,
    pub dr_priority: u8,
    pub current_dr: Option<u32>,
    pub current_bdr: Option<u32>,
    pub is_dr_eligible: bool,
    pub wait_timer_expired: bool,
}

/// Down state handler
pub struct DownStateHandler;

impl InterfaceStateHandler for DownStateHandler {
    fn state(&self) -> InterfaceState {
        InterfaceState::Down
    }
    
    fn on_interface_up(&self, ctx: &mut InterfaceContext) -> InterfaceTransition {
        match ctx.network_type {
            NetworkType::PointToPoint => InterfaceTransition::To(InterfaceState::PointToPoint),
            NetworkType::Broadcast | NetworkType::NBMA => {
                if ctx.dr_priority > 0 {
                    InterfaceTransition::To(InterfaceState::Waiting)
                } else {
                    InterfaceTransition::To(InterfaceState::DROther)
                }
            }
            _ => InterfaceTransition::None,
        }
    }
    
    fn on_interface_down(&self, _ctx: &mut InterfaceContext) -> InterfaceTransition {
        InterfaceTransition::None // Already down
    }
    
    fn on_wait_timer(&self, _ctx: &mut InterfaceContext) -> InterfaceTransition {
        InterfaceTransition::None // Timer not relevant in Down state
    }
    
    fn on_neighbor_change(&self, _ctx: &mut InterfaceContext) -> InterfaceTransition {
        InterfaceTransition::None // Neighbors not relevant in Down state
    }
    
    fn on_loop_ind(&self, _ctx: &mut InterfaceContext) -> InterfaceTransition {
        InterfaceTransition::To(InterfaceState::Loopback)
    }
    
    fn on_unloop_ind(&self, _ctx: &mut InterfaceContext) -> InterfaceTransition {
        InterfaceTransition::None // Not looped
    }
    
    fn on_enter(&self, _ctx: &mut InterfaceContext) -> Vec<OSPFEvent> {
        vec![]
    }
    
    fn on_exit(&self, _ctx: &mut InterfaceContext) -> Vec<OSPFEvent> {
        vec![]
    }
}

/// Waiting state handler
pub struct WaitingStateHandler;

impl InterfaceStateHandler for WaitingStateHandler {
    fn state(&self) -> InterfaceState {
        InterfaceState::Waiting
    }
    
    fn on_interface_up(&self, _ctx: &mut InterfaceContext) -> InterfaceTransition {
        InterfaceTransition::None // Already up
    }
    
    fn on_interface_down(&self, _ctx: &mut InterfaceContext) -> InterfaceTransition {
        InterfaceTransition::To(InterfaceState::Down)
    }
    
    fn on_wait_timer(&self, ctx: &mut InterfaceContext) -> InterfaceTransition {
        ctx.wait_timer_expired = true;
        // Run DR election
        self.run_dr_election(ctx)
    }
    
    fn on_neighbor_change(&self, ctx: &mut InterfaceContext) -> InterfaceTransition {
        // Neighbor change might trigger immediate DR election
        if ctx.wait_timer_expired {
            self.run_dr_election(ctx)
        } else {
            InterfaceTransition::None
        }
    }
    
    fn on_loop_ind(&self, _ctx: &mut InterfaceContext) -> InterfaceTransition {
        InterfaceTransition::To(InterfaceState::Loopback)
    }
    
    fn on_unloop_ind(&self, _ctx: &mut InterfaceContext) -> InterfaceTransition {
        InterfaceTransition::None
    }
    
    fn on_enter(&self, ctx: &mut InterfaceContext) -> Vec<OSPFEvent> {
        // Start wait timer
        vec![
            OSPFEvent::TimerExpired {
                timer_type: crate::ospf_refactored::events::TimerType::Acknowledgment, // Would be WaitTimer
                context: crate::ospf_refactored::events::TimerContext {
                    interface_id: Some(ctx.interface_id),
                    neighbor_id: None,
                    lsa_key: None,
                    additional_data: Some("wait_timer".to_string()),
                },
            }
        ]
    }
    
    fn on_exit(&self, _ctx: &mut InterfaceContext) -> Vec<OSPFEvent> {
        vec![]
    }
}

impl WaitingStateHandler {
    /// Run DR/BDR election and determine new state
    fn run_dr_election(&self, ctx: &InterfaceContext) -> InterfaceTransition {
        // Simplified DR election logic
        // In real implementation, would consider all neighbors
        
        if ctx.current_dr == Some(ctx.interface_id) {
            InterfaceTransition::To(InterfaceState::DR)
        } else if ctx.current_bdr == Some(ctx.interface_id) {
            InterfaceTransition::To(InterfaceState::Backup)
        } else {
            InterfaceTransition::To(InterfaceState::DROther)
        }
    }
}

/// DR state handler
pub struct DRStateHandler;

impl InterfaceStateHandler for DRStateHandler {
    fn state(&self) -> InterfaceState {
        InterfaceState::DR
    }
    
    fn on_interface_up(&self, _ctx: &mut InterfaceContext) -> InterfaceTransition {
        InterfaceTransition::None
    }
    
    fn on_interface_down(&self, _ctx: &mut InterfaceContext) -> InterfaceTransition {
        InterfaceTransition::To(InterfaceState::Down)
    }
    
    fn on_wait_timer(&self, _ctx: &mut InterfaceContext) -> InterfaceTransition {
        InterfaceTransition::None
    }
    
    fn on_neighbor_change(&self, ctx: &mut InterfaceContext) -> InterfaceTransition {
        // Check if we should still be DR
        if ctx.current_dr != Some(ctx.interface_id) {
            if ctx.current_bdr == Some(ctx.interface_id) {
                InterfaceTransition::To(InterfaceState::Backup)
            } else {
                InterfaceTransition::To(InterfaceState::DROther)
            }
        } else {
            InterfaceTransition::None
        }
    }
    
    fn on_loop_ind(&self, _ctx: &mut InterfaceContext) -> InterfaceTransition {
        InterfaceTransition::To(InterfaceState::Loopback)
    }
    
    fn on_unloop_ind(&self, _ctx: &mut InterfaceContext) -> InterfaceTransition {
        InterfaceTransition::None
    }
    
    fn on_enter(&self, ctx: &mut InterfaceContext) -> Vec<OSPFEvent> {
        // Generate Network LSA
        vec![
            OSPFEvent::LSAFloodRequired {
                lsa_key: format!("network_lsa_{}", ctx.interface_id),
                exclude_interface: None,
                exclude_neighbor: None,
            }
        ]
    }
    
    fn on_exit(&self, _ctx: &mut InterfaceContext) -> Vec<OSPFEvent> {
        vec![]
    }
}

/// Factory function to get interface state handler
pub fn get_interface_state_handler(state: InterfaceState) -> Box<dyn InterfaceStateHandler> {
    match state {
        InterfaceState::Down => Box::new(DownStateHandler),
        InterfaceState::Waiting => Box::new(WaitingStateHandler),
        InterfaceState::DR => Box::new(DRStateHandler),
        // TODO: Implement remaining states
        _ => Box::new(DownStateHandler), // Fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_down_to_waiting_transition() {
        let handler = DownStateHandler;
        let mut ctx = InterfaceContext {
            interface_id: 1,
            network_type: NetworkType::Broadcast,
            dr_priority: 1,
            current_dr: None,
            current_bdr: None,
            is_dr_eligible: true,
            wait_timer_expired: false,
        };
        
        match handler.on_interface_up(&mut ctx) {
            InterfaceTransition::To(new_state) => {
                assert_eq!(new_state, InterfaceState::Waiting);
            }
            _ => panic!("Expected transition to Waiting"),
        }
    }
    
    #[test]
    fn test_down_to_pointtopoint_transition() {
        let handler = DownStateHandler;
        let mut ctx = InterfaceContext {
            interface_id: 1,
            network_type: NetworkType::PointToPoint,
            dr_priority: 1,
            current_dr: None,
            current_bdr: None,
            is_dr_eligible: false,
            wait_timer_expired: false,
        };
        
        match handler.on_interface_up(&mut ctx) {
            InterfaceTransition::To(new_state) => {
                assert_eq!(new_state, InterfaceState::PointToPoint);
            }
            _ => panic!("Expected transition to PointToPoint"),
        }
    }
}
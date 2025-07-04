// OSPF State Management
//
// This module implements state machines for OSPF protocol entities,
// including neighbor states and interface states.

pub mod neighbor_state;
pub mod interface_state;
pub mod transition_validator;

pub use neighbor_state::{NeighborState, NeighborStateHandler, StateTransition};
pub use interface_state::{InterfaceState, InterfaceStateHandler, InterfaceContext, InterfaceTransition};
pub use transition_validator::{
    NeighborTransitionValidator, InterfaceTransitionValidator, TransitionCondition
};

/// Common state context used by state handlers
#[derive(Debug, Clone)]
pub struct StateContext {
    pub router_id: u32,
    pub interface_id: u32,
    pub current_time: f64,
    pub area_id: u32,
}

/// Result type for state operations
pub type StateResult<T> = Result<T, StateError>;

/// Errors that can occur during state transitions
#[derive(Debug, Clone)]
pub enum StateError {
    InvalidTransition {
        from: String,
        to: String,
    },
    PreconditionFailed(String),
    ProcessingError(String),
}

impl std::fmt::Display for StateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StateError::InvalidTransition { from, to } => {
                write!(f, "Invalid state transition from {} to {}", from, to)
            }
            StateError::PreconditionFailed(msg) => write!(f, "State precondition not met: {}", msg),
            StateError::ProcessingError(msg) => write!(f, "State processing error: {}", msg),
        }
    }
}

impl std::error::Error for StateError {}
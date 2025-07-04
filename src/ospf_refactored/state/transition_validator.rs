// State Transition Validator
//
// Validates OSPF state transitions according to RFC 2328 rules.
// Ensures that state machines follow the correct transition paths.

use super::{NeighborState, InterfaceState, StateError};
use std::collections::HashMap;

/// Validates neighbor state transitions
pub struct NeighborTransitionValidator {
    /// Valid transitions: from_state -> list of allowed to_states
    valid_transitions: HashMap<NeighborState, Vec<NeighborState>>,
}

impl NeighborTransitionValidator {
    pub fn new() -> Self {
        let mut valid_transitions = HashMap::new();
        
        // Define valid transitions according to RFC 2328 Section 10.3
        
        // From Down state
        valid_transitions.insert(
            NeighborState::Down,
            vec![NeighborState::Init, NeighborState::Down],
        );
        
        // From Init state
        valid_transitions.insert(
            NeighborState::Init,
            vec![
                NeighborState::Down,
                NeighborState::TwoWay,
                NeighborState::Init,
            ],
        );
        
        // From TwoWay state
        valid_transitions.insert(
            NeighborState::TwoWay,
            vec![
                NeighborState::Down,
                NeighborState::Init,
                NeighborState::ExStart,
                NeighborState::TwoWay,
            ],
        );
        
        // From ExStart state
        valid_transitions.insert(
            NeighborState::ExStart,
            vec![
                NeighborState::Down,
                NeighborState::Init,
                NeighborState::TwoWay,
                NeighborState::Exchange,
                NeighborState::ExStart,
            ],
        );
        
        // From Exchange state
        valid_transitions.insert(
            NeighborState::Exchange,
            vec![
                NeighborState::Down,
                NeighborState::Init,
                NeighborState::TwoWay,
                NeighborState::Loading,
                NeighborState::Full,
                NeighborState::Exchange,
            ],
        );
        
        // From Loading state
        valid_transitions.insert(
            NeighborState::Loading,
            vec![
                NeighborState::Down,
                NeighborState::Init,
                NeighborState::TwoWay,
                NeighborState::Full,
                NeighborState::Loading,
            ],
        );
        
        // From Full state
        valid_transitions.insert(
            NeighborState::Full,
            vec![
                NeighborState::Down,
                NeighborState::Init,
                NeighborState::TwoWay,
                NeighborState::ExStart,
                NeighborState::Full,
            ],
        );
        
        Self { valid_transitions }
    }
    
    /// Validate a neighbor state transition
    pub fn validate_transition(
        &self,
        from: NeighborState,
        to: NeighborState,
    ) -> Result<(), StateError> {
        if let Some(allowed_states) = self.valid_transitions.get(&from) {
            if allowed_states.contains(&to) {
                Ok(())
            } else {
                Err(StateError::InvalidTransition {
                    from: format!("{:?}", from),
                    to: format!("{:?}", to),
                })
            }
        } else {
            Err(StateError::InvalidTransition {
                from: format!("{:?}", from),
                to: format!("{:?}", to),
            })
        }
    }
    
    /// Check if a transition requires special conditions
    pub fn transition_requires_condition(
        &self,
        from: NeighborState,
        to: NeighborState,
    ) -> Option<TransitionCondition> {
        match (from, to) {
            (NeighborState::Init, NeighborState::TwoWay) => {
                Some(TransitionCondition::BidirectionalCommunication)
            }
            (NeighborState::TwoWay, NeighborState::ExStart) => {
                Some(TransitionCondition::AdjacencyRequired)
            }
            (NeighborState::ExStart, NeighborState::Exchange) => {
                Some(TransitionCondition::MasterSlaveNegotiated)
            }
            (NeighborState::Exchange, NeighborState::Loading) => {
                Some(TransitionCondition::DatabaseDescriptionComplete)
            }
            (NeighborState::Exchange, NeighborState::Full) => {
                Some(TransitionCondition::NoLSAsToRequest)
            }
            (NeighborState::Loading, NeighborState::Full) => {
                Some(TransitionCondition::AllLSAsReceived)
            }
            _ => None,
        }
    }
}

/// Validates interface state transitions
pub struct InterfaceTransitionValidator {
    /// Valid transitions: from_state -> list of allowed to_states
    valid_transitions: HashMap<InterfaceState, Vec<InterfaceState>>,
}

impl InterfaceTransitionValidator {
    pub fn new() -> Self {
        let mut valid_transitions = HashMap::new();
        
        // Define valid transitions according to RFC 2328 Section 9.3
        
        // From Down state
        valid_transitions.insert(
            InterfaceState::Down,
            vec![
                InterfaceState::Loopback,
                InterfaceState::PointToPoint,
                InterfaceState::Waiting,
                InterfaceState::DROther,
                InterfaceState::Down,
            ],
        );
        
        // From Loopback state
        valid_transitions.insert(
            InterfaceState::Loopback,
            vec![
                InterfaceState::Down,
                InterfaceState::Loopback,
            ],
        );
        
        // From Waiting state
        valid_transitions.insert(
            InterfaceState::Waiting,
            vec![
                InterfaceState::Down,
                InterfaceState::Loopback,
                InterfaceState::DR,
                InterfaceState::Backup,
                InterfaceState::DROther,
                InterfaceState::Waiting,
            ],
        );
        
        // From PointToPoint state
        valid_transitions.insert(
            InterfaceState::PointToPoint,
            vec![
                InterfaceState::Down,
                InterfaceState::Loopback,
                InterfaceState::PointToPoint,
            ],
        );
        
        // From DROther state
        valid_transitions.insert(
            InterfaceState::DROther,
            vec![
                InterfaceState::Down,
                InterfaceState::Loopback,
                InterfaceState::DR,
                InterfaceState::Backup,
                InterfaceState::DROther,
            ],
        );
        
        // From Backup state
        valid_transitions.insert(
            InterfaceState::Backup,
            vec![
                InterfaceState::Down,
                InterfaceState::Loopback,
                InterfaceState::DR,
                InterfaceState::DROther,
                InterfaceState::Backup,
            ],
        );
        
        // From DR state
        valid_transitions.insert(
            InterfaceState::DR,
            vec![
                InterfaceState::Down,
                InterfaceState::Loopback,
                InterfaceState::Backup,
                InterfaceState::DROther,
                InterfaceState::DR,
            ],
        );
        
        Self { valid_transitions }
    }
    
    /// Validate an interface state transition
    pub fn validate_transition(
        &self,
        from: InterfaceState,
        to: InterfaceState,
    ) -> Result<(), StateError> {
        if let Some(allowed_states) = self.valid_transitions.get(&from) {
            if allowed_states.contains(&to) {
                Ok(())
            } else {
                Err(StateError::InvalidTransition {
                    from: format!("{:?}", from),
                    to: format!("{:?}", to),
                })
            }
        } else {
            Err(StateError::InvalidTransition {
                from: format!("{:?}", from),
                to: format!("{:?}", to),
            })
        }
    }
}

/// Conditions required for certain transitions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransitionCondition {
    /// Bidirectional communication established (seen self in neighbor's hello)
    BidirectionalCommunication,
    
    /// Adjacency formation is required
    AdjacencyRequired,
    
    /// Master/Slave relationship negotiated
    MasterSlaveNegotiated,
    
    /// Database description exchange complete
    DatabaseDescriptionComplete,
    
    /// No LSAs need to be requested
    NoLSAsToRequest,
    
    /// All requested LSAs have been received
    AllLSAsReceived,
    
    /// DR/BDR election complete
    ElectionComplete,
    
    /// Interface became DR
    BecameDR,
    
    /// Interface became BDR
    BecameBDR,
}

impl Default for NeighborTransitionValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for InterfaceTransitionValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_valid_neighbor_transitions() {
        let validator = NeighborTransitionValidator::new();
        
        // Valid transitions
        assert!(validator.validate_transition(NeighborState::Down, NeighborState::Init).is_ok());
        assert!(validator.validate_transition(NeighborState::Init, NeighborState::TwoWay).is_ok());
        assert!(validator.validate_transition(NeighborState::TwoWay, NeighborState::ExStart).is_ok());
        assert!(validator.validate_transition(NeighborState::Full, NeighborState::Init).is_ok());
    }
    
    #[test]
    fn test_invalid_neighbor_transitions() {
        let validator = NeighborTransitionValidator::new();
        
        // Invalid transitions
        assert!(validator.validate_transition(NeighborState::Down, NeighborState::Full).is_err());
        assert!(validator.validate_transition(NeighborState::Init, NeighborState::Loading).is_err());
        assert!(validator.validate_transition(NeighborState::Loading, NeighborState::Exchange).is_err());
    }
    
    #[test]
    fn test_transition_conditions() {
        let validator = NeighborTransitionValidator::new();
        
        assert_eq!(
            validator.transition_requires_condition(NeighborState::Init, NeighborState::TwoWay),
            Some(TransitionCondition::BidirectionalCommunication)
        );
        
        assert_eq!(
            validator.transition_requires_condition(NeighborState::TwoWay, NeighborState::ExStart),
            Some(TransitionCondition::AdjacencyRequired)
        );
        
        assert_eq!(
            validator.transition_requires_condition(NeighborState::Down, NeighborState::Init),
            None
        );
    }
    
    #[test]
    fn test_valid_interface_transitions() {
        let validator = InterfaceTransitionValidator::new();
        
        // Valid transitions
        assert!(validator.validate_transition(InterfaceState::Down, InterfaceState::Waiting).is_ok());
        assert!(validator.validate_transition(InterfaceState::Waiting, InterfaceState::DR).is_ok());
        assert!(validator.validate_transition(InterfaceState::DR, InterfaceState::Backup).is_ok());
    }
    
    #[test]
    fn test_invalid_interface_transitions() {
        let validator = InterfaceTransitionValidator::new();
        
        // Invalid transitions
        assert!(validator.validate_transition(InterfaceState::Loopback, InterfaceState::DR).is_err());
        assert!(validator.validate_transition(InterfaceState::PointToPoint, InterfaceState::DR).is_err());
    }
}
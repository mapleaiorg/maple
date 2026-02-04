use crate::error::IBankError;

/// Strict execution stages for consequential iBank actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsequenceStage {
    Initialized,
    Presence,
    Coupling,
    Meaning,
    Intent,
    Commitment,
    Consequence,
}

impl ConsequenceStage {
    pub fn name(self) -> &'static str {
        match self {
            Self::Initialized => "initialized",
            Self::Presence => "presence",
            Self::Coupling => "coupling",
            Self::Meaning => "meaning",
            Self::Intent => "intent",
            Self::Commitment => "commitment",
            Self::Consequence => "consequence",
        }
    }
}

/// Enforces presence->coupling->meaning->intent->commitment->consequence ordering.
///
/// The stage machine is intentionally explicit so accidental skips cannot happen silently.
#[derive(Debug, Clone)]
pub struct ConsequenceStageMachine {
    trace_id: String,
    stage: ConsequenceStage,
}

impl ConsequenceStageMachine {
    pub fn new(trace_id: impl Into<String>) -> Self {
        Self {
            trace_id: trace_id.into(),
            stage: ConsequenceStage::Initialized,
        }
    }

    pub fn trace_id(&self) -> &str {
        &self.trace_id
    }

    pub fn stage(&self) -> ConsequenceStage {
        self.stage
    }

    pub fn mark_presence(&mut self) -> Result<(), IBankError> {
        self.advance(ConsequenceStage::Initialized, ConsequenceStage::Presence)
    }

    pub fn mark_coupling(&mut self) -> Result<(), IBankError> {
        self.advance(ConsequenceStage::Presence, ConsequenceStage::Coupling)
    }

    pub fn mark_meaning(&mut self) -> Result<(), IBankError> {
        self.advance(ConsequenceStage::Coupling, ConsequenceStage::Meaning)
    }

    pub fn mark_intent(&mut self) -> Result<(), IBankError> {
        self.advance(ConsequenceStage::Meaning, ConsequenceStage::Intent)
    }

    pub fn mark_commitment(&mut self) -> Result<(), IBankError> {
        self.advance(ConsequenceStage::Intent, ConsequenceStage::Commitment)
    }

    pub fn mark_consequence(&mut self) -> Result<(), IBankError> {
        self.advance(ConsequenceStage::Commitment, ConsequenceStage::Consequence)
    }

    fn advance(
        &mut self,
        expected_current: ConsequenceStage,
        next: ConsequenceStage,
    ) -> Result<(), IBankError> {
        if self.stage != expected_current {
            return Err(IBankError::stage_violation(
                expected_current.name(),
                self.stage.name(),
            ));
        }
        self.stage = next;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforces_stage_order() {
        let mut machine = ConsequenceStageMachine::new("trace-a");
        assert!(machine.mark_presence().is_ok());
        assert!(machine.mark_coupling().is_ok());
        assert!(machine.mark_meaning().is_ok());
        assert!(machine.mark_intent().is_ok());
        assert!(machine.mark_commitment().is_ok());
        assert!(machine.mark_consequence().is_ok());
    }

    #[test]
    fn rejects_skipping_commitment() {
        let mut machine = ConsequenceStageMachine::new("trace-b");
        machine.mark_presence().unwrap();
        machine.mark_coupling().unwrap();
        machine.mark_meaning().unwrap();
        machine.mark_intent().unwrap();

        let err = machine.mark_consequence().unwrap_err();
        assert!(err
            .to_string()
            .contains("expected 'commitment', got 'intent'"));
    }
}

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TopicItemKind {
    Issue,
    Proposal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueState {
    Open,
    Closed,
}

impl IssueState {
    pub fn close(&mut self) -> Result<(), StateTransitionError> {
        self.transition(Self::Closed)
    }

    pub fn reopen(&mut self) -> Result<(), StateTransitionError> {
        self.transition(Self::Open)
    }

    fn transition(&mut self, target: Self) -> Result<(), StateTransitionError> {
        if *self == target {
            return Err(StateTransitionError::AlreadyInState);
        }
        *self = target;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalState {
    Draft,
    Open,
    Merged,
    Closed,
}

impl ProposalState {
    pub fn open(&mut self) -> Result<(), StateTransitionError> {
        match self {
            Self::Draft | Self::Closed => {
                *self = Self::Open;
                Ok(())
            }
            Self::Open => Err(StateTransitionError::AlreadyInState),
            Self::Merged => Err(StateTransitionError::TerminalState),
        }
    }

    pub fn close(&mut self) -> Result<(), StateTransitionError> {
        match self {
            Self::Draft | Self::Open => {
                *self = Self::Closed;
                Ok(())
            }
            Self::Closed => Err(StateTransitionError::AlreadyInState),
            Self::Merged => Err(StateTransitionError::TerminalState),
        }
    }

    pub fn merge(&mut self) -> Result<(), StateTransitionError> {
        match self {
            Self::Open => {
                *self = Self::Merged;
                Ok(())
            }
            Self::Merged => Err(StateTransitionError::AlreadyInState),
            Self::Draft | Self::Closed => Err(StateTransitionError::MustBeOpen),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewVerdict {
    Comment,
    Approve,
    RequestChanges,
}

impl ReviewVerdict {
    #[must_use]
    pub const fn blocks_merge(self, reviewer: PrincipalKind) -> bool {
        matches!(reviewer, PrincipalKind::Human) && matches!(self, Self::RequestChanges)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Queued,
    InProgress,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunConclusion {
    Success,
    Failure,
    Cancelled,
    TimedOut,
    Skipped,
    Neutral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum StateTransitionError {
    #[error("resource is already in the requested state")]
    AlreadyInState,
    #[error("resource is in a terminal state")]
    TerminalState,
    #[error("proposal must be open")]
    MustBeOpen,
}

use super::PrincipalKind;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proposal_can_only_merge_while_open() {
        let mut draft = ProposalState::Draft;
        assert_eq!(draft.merge(), Err(StateTransitionError::MustBeOpen));
        draft.open().unwrap();
        draft.merge().unwrap();
        assert_eq!(draft, ProposalState::Merged);
        assert_eq!(draft.open(), Err(StateTransitionError::TerminalState));
    }

    #[test]
    fn only_human_change_requests_block_merge() {
        assert!(ReviewVerdict::RequestChanges.blocks_merge(PrincipalKind::Human));
        assert!(!ReviewVerdict::RequestChanges.blocks_merge(PrincipalKind::Ai));
        assert!(!ReviewVerdict::Approve.blocks_merge(PrincipalKind::Human));
    }
}

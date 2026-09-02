use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrincipalKind {
    Human,
    Ai,
    Caller,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MembershipRole {
    Read,
    Contribute,
    Write,
}

#[must_use]
pub const fn can_merge_proposal(kind: PrincipalKind, role: MembershipRole) -> bool {
    matches!(kind, PrincipalKind::Human) && matches!(role, MembershipRole::Write)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_human_writers_can_merge() {
        assert!(can_merge_proposal(
            PrincipalKind::Human,
            MembershipRole::Write
        ));

        for kind in [
            PrincipalKind::Ai,
            PrincipalKind::Caller,
            PrincipalKind::System,
        ] {
            assert!(!can_merge_proposal(kind, MembershipRole::Write));
        }

        assert!(!can_merge_proposal(
            PrincipalKind::Human,
            MembershipRole::Contribute
        ));
    }
}

use super::error::DomainError;

/// The manual owner-approval state of an account. A new account starts `Pending`;
/// the app owner promotes it to `Approved` out-of-band (an operator updates the
/// row) after the user has verified their email. `Rejected` is a terminal state
/// the cleanup sweep will eventually delete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
}

impl ApprovalStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ApprovalStatus::Pending => "pending",
            ApprovalStatus::Approved => "approved",
            ApprovalStatus::Rejected => "rejected",
        }
    }

    /// Parse a stored approval-status string. Unknown values are a data error.
    pub fn parse(s: &str) -> Result<ApprovalStatus, DomainError> {
        match s {
            "pending" => Ok(ApprovalStatus::Pending),
            "approved" => Ok(ApprovalStatus::Approved),
            "rejected" => Ok(ApprovalStatus::Rejected),
            other => Err(DomainError::Repository(format!(
                "unknown approval status '{other}'"
            ))),
        }
    }
}

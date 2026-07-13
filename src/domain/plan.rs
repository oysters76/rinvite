use super::error::DomainError;

/// A subscription plan. Every account has exactly one; new accounts start on
/// `Free`. Plan changes are performed out-of-band (an operator updates the row)
/// — there is no self-serve billing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plan {
    Free,
    Pro,
    Max,
}

/// The usage ceilings for a plan. `None` means "no limit" (the `Max` plan).
#[derive(Debug, Clone, Copy)]
pub struct PlanLimits {
    /// Maximum number of events an owner may create.
    pub max_events: Option<u32>,
    /// Maximum number of guests allowed on a single event.
    pub max_guests_per_event: Option<u32>,
}

impl Plan {
    /// The limits enforced for this plan.
    pub fn limits(&self) -> PlanLimits {
        match self {
            Plan::Free => PlanLimits {
                max_events: Some(1),
                max_guests_per_event: Some(10),
            },
            Plan::Pro => PlanLimits {
                max_events: Some(5),
                max_guests_per_event: Some(100),
            },
            Plan::Max => PlanLimits {
                max_events: None,
                max_guests_per_event: None,
            },
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Plan::Free => "free",
            Plan::Pro => "pro",
            Plan::Max => "max",
        }
    }

    /// Parse a stored plan string. Unknown values are a data/config error.
    pub fn parse(s: &str) -> Result<Plan, DomainError> {
        match s {
            "free" => Ok(Plan::Free),
            "pro" => Ok(Plan::Pro),
            "max" => Ok(Plan::Max),
            other => Err(DomainError::Repository(format!("unknown plan '{other}'"))),
        }
    }
}

use super::error::DomainError;

// Domain policy for what counts as an acceptable email/password. These are
// pure functions (no framework, no I/O) so they hold no matter which inbound
// adapter — HTTP, CLI, a message consumer — calls the use case.

const EMAIL_MIN_LEN: usize = 3; // "a@b" is the shortest conceivable address
const EMAIL_MAX_LEN: usize = 254; // RFC 5321 upper bound on a forward-path
const PASSWORD_MIN_LEN: usize = 8;
// Upper bound so a giant password can't turn argon2 into a CPU DoS vector.
const PASSWORD_MAX_LEN: usize = 1024;

/// Cheap structural email check — deliberately not a full RFC 5322 parser.
/// Requires exactly one `@`, a non-empty local part, and a dotted domain.
pub fn validate_email(email: &str) -> Result<(), DomainError> {
    let email = email.trim();

    if !(EMAIL_MIN_LEN..=EMAIL_MAX_LEN).contains(&email.len()) {
        return Err(DomainError::InvalidInput(
            "email length is out of range".to_owned(),
        ));
    }

    let mut parts = email.split('@');
    let (local, domain) = match (parts.next(), parts.next(), parts.next()) {
        (Some(local), Some(domain), None) => (local, domain),
        // zero `@`, or more than one
        _ => {
            return Err(DomainError::InvalidInput(
                "email must contain exactly one '@'".to_owned(),
            ));
        }
    };

    let domain_has_dot = domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.');
    if local.is_empty() || domain.is_empty() || !domain_has_dot {
        return Err(DomainError::InvalidInput(
            "email is not a valid address".to_owned(),
        ));
    }

    Ok(())
}

/// Password length policy. We bound the byte length on both ends: a floor for
/// strength, a ceiling to cap argon2's work per request.
pub fn validate_password(password: &str) -> Result<(), DomainError> {
    if !(PASSWORD_MIN_LEN..=PASSWORD_MAX_LEN).contains(&password.len()) {
        return Err(DomainError::InvalidInput(format!(
            "password must be between {PASSWORD_MIN_LEN} and {PASSWORD_MAX_LEN} characters"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_reasonable_email() {
        assert!(validate_email("a@b.com").is_ok());
        assert!(validate_email("  user.name@example.co.uk  ").is_ok());
    }

    #[test]
    fn rejects_malformed_emails() {
        for bad in [
            "",
            "no-at-sign",
            "a@b",
            "@b.com",
            "a@",
            "a@@b.com",
            "a@.com",
        ] {
            assert!(validate_email(bad).is_err(), "should reject {bad:?}");
        }
    }

    #[test]
    fn enforces_password_bounds() {
        assert!(validate_password("hunter2!").is_ok());
        assert!(validate_password("short").is_err());
        assert!(validate_password(&"x".repeat(PASSWORD_MAX_LEN + 1)).is_err());
    }
}

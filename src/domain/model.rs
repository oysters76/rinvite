use uuid::Uuid;

/// Core domain entity. No framework, database, or transport types leak in here.
#[derive(Debug, Clone)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
}

impl User {
    /// Create a brand new user with a freshly generated id.
    pub fn new(email: String, password_hash: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            email,
            password_hash,
        }
    }
}

use derive_more::Display;
use rand::{Rng, rng};
use thiserror::Error;

/// Hash password using argon2
pub fn hash_password(password: String, config: &argon2::Config<'_>) -> Result<String, HashError> {
    // Generate radom salt
    let mut salt = [b'0'; 32];
    rng().fill(&mut salt);

    // Create hashed password
    let hashed_password = argon2::hash_encoded(password.as_bytes(), &salt, config)?;

    Ok(hashed_password)
}

#[derive(Debug, Display, Error)]
pub enum HashError {
    Argon(#[from] argon2::Error),
}

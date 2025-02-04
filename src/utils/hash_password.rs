use quick_error::quick_error;
use rand::{rng, Rng};

/// Hash password using argon2
pub fn hash_password(password: String, config: &argon2::Config<'_>) -> Result<String, HashError> {
    // Generate radom salt
    let mut salt = [b'0'; 32];
    rng().fill(&mut salt);

    // Create hashed password
    let hashed_password = argon2::hash_encoded(password.as_bytes(), &salt, config)?;

    Ok(hashed_password)
}

quick_error! {
    #[derive(Debug)]
    pub enum HashError {
        Argon(err: argon2::Error) { from() }
    }
}

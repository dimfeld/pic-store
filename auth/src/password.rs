use crate::error::Error;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

pub fn new_hash(password: &str) -> Result<String, Error> {
    hash_password(password)
}

fn hash_password(password: &str) -> Result<String, Error> {
    let saltstring = SaltString::generate(&mut OsRng);

    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &saltstring)
        .map_err(|e| Error::PasswordHasherError(e.to_string()))?;

    Ok(hash.to_string())
}

pub fn verify_password(password: &str, hash_str: &str) -> Result<(), Error> {
    let hash =
        PasswordHash::new(hash_str).map_err(|e| Error::PasswordHasherError(e.to_string()))?;

    Argon2::default()
        .verify_password(password.as_bytes(), &hash)
        .map_err(|_| Error::InvalidPassword)
}

#[cfg(all(test, any(feature = "test-slow", feature = "test-password")))]
mod tests {
    use super::*;
    use crate::error::Result;

    #[test]
    fn good_password() -> Result<()> {
        let hash = new_hash("abcdef")?;
        verify_password("abcdef", &hash)
    }

    #[test]
    fn bad_password() -> Result<()> {
        let hash = new_hash("abcdef")?;
        verify_password("abcdefg", &hash).expect_err("non-matching password");
        Ok(())
    }

    #[test]
    fn unique_password_salt() {
        let p1 = new_hash("abc").unwrap();
        let p2 = new_hash("abc").unwrap();
        assert_ne!(p1, p2);
    }
}

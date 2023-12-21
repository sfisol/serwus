//! Allows to validate access token against 3rd party authority

use awc::Client;
use alcoholic_jwt::{token_kid, validate, Validation, JWKS, ValidJWT};

#[derive(Debug)]
pub enum ValidateError {
    Super(alcoholic_jwt::ValidationError),
    NoKidInToken,
    NoKidInJwks,
}

#[derive(Debug)]
pub enum CredentialsError {
    JwksFetch(String),
    AccessToken(ValidateError),
    IdToken(ValidateError),
}

pub async fn validate_credentials(authority: &str, access_token: String, id_token: String) -> Result<ValidJWT, CredentialsError> {
    let uri = format!("{}/{}", authority, ".well-known/jwks.json");
    let jwks = fetch_jwks(&uri)
        .await
        .map_err(CredentialsError::JwksFetch)?;

    validate_token(authority, &access_token, &jwks)
        .await
        .map_err(CredentialsError::AccessToken)?;

    validate_token(authority, &id_token, &jwks)
        .await
        .map_err(CredentialsError::IdToken)
}

async fn fetch_jwks(uri: &str) -> Result<JWKS, String> {
    let mut res = Client::default().get(uri).send()
        .await
        .map_err(|e| e.to_string())?;

    let val = res.json::<JWKS>()
        .await
        .map_err(|e| e.to_string())?;
    Ok(val)
}

async fn validate_token(authority: &str, token: &str, jwks: &JWKS) -> Result<ValidJWT, ValidateError> {
    let validations = vec![Validation::Issuer(authority.to_string()), Validation::SubjectPresent];
    let kid = match token_kid(token) {
        Ok(Some(kid)) => kid,
        Ok(None) => return Err(ValidateError::NoKidInToken),
        Err(e) => return Err(ValidateError::Super(e)),
    };

    let jwk = match jwks.find(&kid) {
        Some(jwk) => jwk,
        None => return Err(ValidateError::NoKidInJwks),
    };

    let res = validate(token, jwk, validations)
        .map_err(ValidateError::Super)?;

    // TODO: CHeck expiry time

    Ok(res)
}

use actix_web::{
    HttpRequest, Error,
    error::{ErrorUnauthorized},
};
use jsonwebtoken::{
    encode, decode, Header, Validation, DecodingKey, EncodingKey,
    errors::{Error as JwtError, ErrorKind::ExpiredSignature},
};
use log::warn;
use serde::{Serialize, de::DeserializeOwned};

pub trait KnowSecret {
    fn get_secret() -> Vec<u8>;
}

// TODO: Unify logic for AccessToken and UserToken
pub fn encode_jwt<T>(object: &T) -> Result<String, JwtError>
where T: Serialize + KnowSecret
{
    let header: &Header = &Header::default();
    encode(header, object, &EncodingKey::from_secret(&T::get_secret()))
}

pub trait FromEncoded: Sized + KnowSecret {
    fn from_encoded(encoded_token: &str) -> Result<Self, JwtError>;
}

impl<T: DeserializeOwned + KnowSecret> FromEncoded for T {
    fn from_encoded(encoded_token: &str) -> Result<T, JwtError> {
        let header: &Header = &Header::default();
        let validation: &Validation = &Validation::new(header.alg);

        decode::<T>(encoded_token, &DecodingKey::from_secret(&T::get_secret()), validation)
            .map(|data| data.claims)
    }
}

// TODO: If FromRequest returns Unauthorized, additional headers should be added like WWW-Authorization.
//       See: https://developer.mozilla.org/pl/docs/Web/HTTP/Headers/WWW-Authenticate
pub fn from_request<T: Sized + FromEncoded>(req: &HttpRequest) -> Result<T, Error> {
    let encoded_tokens: Vec<_> = req.headers().get_all("Authorization")
        .filter_map(|header_value| header_value.to_str().ok())
        .filter_map(|string_value| {
            let mut split = string_value.split_whitespace();
            if let Some(auth_type) = split.next() {
                if auth_type == "Bearer" {
                    return split.next();
                }
            }
            None
        })
        .collect();

    let encoded_token = match encoded_tokens.len() {
        0 => return Err(ErrorUnauthorized("Missing auth token")),
        1 => encoded_tokens[0],
        _ => return Err(ErrorUnauthorized("Multiple auth headers")),
    };

    match T::from_encoded(encoded_token) {
        Err(error) => {
            if let ExpiredSignature = error.kind() {
                Err(ErrorUnauthorized("Token Expired"))
            } else {
                warn!("Error decoding auth token: {}", error);
                Err(ErrorUnauthorized("Invalid Token"))
            }
        },
        Ok(access_token) => Ok(access_token)
    }
}

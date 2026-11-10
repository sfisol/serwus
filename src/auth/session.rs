//! Sessions kept in a cookie: an opaque random token the browser sends back with every
//! request, so server-side rendering sees the session too. The service stores only the
//! token's hash, so a leaked database doesn't leak working sessions.
//!
//! Register a [`SessionCookie`] with `app_data`, then take [`SessionToken`] (or
//! `Option<SessionToken>`) and the [`SessionCookie`] itself as extractors:
//!
//! ```no_run
//! use actix_web::{App, HttpResponse, web};
//! use serwus::auth::session::{SessionCookie, SessionToken};
//!
//! async fn whoami(token: SessionToken) -> String {
//!     // Look the session up by its hash, never by the token itself
//!     token.hash()
//! }
//!
//! async fn start(cookie: SessionCookie) -> HttpResponse {
//!     let token = SessionToken::generate();
//!     // Store token.hash() with the new session, then hand the token to the browser
//!     HttpResponse::Ok().cookie(cookie.cookie(&token)).finish()
//! }
//!
//! let app = App::new()
//!     .app_data(SessionCookie::new("game_session"))
//!     .route("/whoami", web::get().to(whoami))
//!     .route("/start", web::post().to(start));
//! ```

use std::{
    fmt::{self, Write},
    future::{Ready, ready},
};

use actix_web::{
    Error, FromRequest, HttpRequest,
    cookie::{Cookie, SameSite, time::Duration},
    dev::Payload,
};
use rand::{RngExt, distr::Alphanumeric, rng};
use sha2::{Digest, Sha256};

use crate::server::json_error::ErrorBuilder;

/// Characters of a token: 43 alphanumerics carry about 256 bits.
pub const TOKEN_LENGTH: usize = 43;

/// The secret the browser holds. Store [`SessionToken::hash`], never the token; its `Debug`
/// doesn't show it, so it can't end up in a log by accident.
#[derive(Clone, PartialEq, Eq)]
pub struct SessionToken(String);

impl SessionToken {
    /// A new random token.
    pub fn generate() -> Self {
        let mut rng = rng();
        Self(
            (0..TOKEN_LENGTH)
                .map(|_| char::from(rng.sample(Alphanumeric)))
                .collect(),
        )
    }

    /// The token a cookie carries, or `None` if the value can't be one.
    pub fn parse(value: &str) -> Option<Self> {
        let well_formed =
            value.len() == TOKEN_LENGTH && value.bytes().all(|byte| byte.is_ascii_alphanumeric());
        well_formed.then(|| Self(value.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// SHA-256 of the token in lowercase hex: what the service stores and looks sessions up
    /// by. A plain hash is enough, since the token is random and long, not a password.
    pub fn hash(&self) -> String {
        Sha256::digest(self.0.as_bytes())
            .iter()
            .fold(String::with_capacity(64), |mut hex, byte| {
                // Writing to a String can't fail
                let _ = write!(hex, "{byte:02x}");
                hex
            })
    }
}

impl fmt::Debug for SessionToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SessionToken(..)")
    }
}

/// How the session cookie is set. The [`SessionToken`] extractor reads it from `app_data`.
#[derive(Clone, Debug)]
pub struct SessionCookie {
    name: String,
    secure: bool,
    max_age: Duration,
}

impl SessionCookie {
    /// A cookie `name` that is `Secure`, `HttpOnly`, `SameSite=Lax`, for the whole site and
    /// for a year.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            secure: true,
            max_age: Duration::days(365),
        }
    }

    /// Browsers keep a `Secure` cookie on `http://localhost` too, so turn it off only for
    /// plain HTTP to another host.
    pub fn secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }

    pub fn max_age(mut self, max_age: Duration) -> Self {
        self.max_age = max_age;
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// The cookie that gives the browser `token`.
    pub fn cookie(&self, token: &SessionToken) -> Cookie<'static> {
        Cookie::build(self.name.clone(), token.as_str().to_string())
            .path("/")
            .http_only(true)
            .secure(self.secure)
            .same_site(SameSite::Lax)
            .max_age(self.max_age)
            .finish()
    }

    /// The cookie that removes the session from the browser.
    pub fn removal(&self) -> Cookie<'static> {
        let mut cookie = Cookie::build(self.name.clone(), "")
            .path("/")
            .http_only(true)
            .secure(self.secure)
            .same_site(SameSite::Lax)
            .finish();
        cookie.make_removal();
        cookie
    }

    /// The well-formed token `req` carries, if any.
    pub fn token(&self, req: &HttpRequest) -> Option<SessionToken> {
        req.cookie(&self.name)
            .and_then(|cookie| SessionToken::parse(cookie.value()))
    }
}

/// The cookie registered with `app_data`, or 500 when there is none.
fn registered(req: &HttpRequest) -> Result<&SessionCookie, Error> {
    req.app_data::<SessionCookie>().ok_or_else(|| {
        log::error!("sessions need a SessionCookie registered with app_data");
        ErrorBuilder::internal("session cookie not configured")
            .finish()
            .into()
    })
}

/// The session of the request: 401 without a well-formed session cookie, 500 when no
/// [`SessionCookie`] is registered. `Option<SessionToken>` gives `None` instead of 401.
impl FromRequest for SessionToken {
    type Error = Error;
    type Future = Ready<Result<Self, Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let token = registered(req).and_then(|config| {
            config.token(req).ok_or_else(|| {
                ErrorBuilder::unauthorized("missing session")
                    .finish()
                    .into()
            })
        });
        ready(token)
    }
}

/// The registered cookie, for handlers that start or end sessions.
impl FromRequest for SessionCookie {
    type Error = Error;
    type Future = Ready<Result<Self, Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        ready(registered(req).cloned())
    }
}

#[cfg(test)]
mod tests {
    use actix_web::{App, HttpResponse, http::StatusCode, test as actix_test, web};

    use super::*;

    const TOKEN: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[test]
    fn generated_tokens_are_well_formed_and_differ() {
        let a = SessionToken::generate();
        let b = SessionToken::generate();
        assert_eq!(SessionToken::parse(a.as_str()), Some(a.clone()));
        assert_ne!(a, b);
    }

    #[test]
    fn values_that_are_no_token_are_refused() {
        assert!(SessionToken::parse(TOKEN).is_some());
        for value in [
            "",
            &TOKEN[1..],
            &format!("{TOKEN}a"),
            &format!("{}-", &TOKEN[1..]),
        ] {
            assert_eq!(SessionToken::parse(value), None, "{value:?}");
        }
    }

    /// The reference value comes from Python's hashlib.
    #[test]
    fn the_hash_is_sha256_in_hex() {
        let token = SessionToken::parse(TOKEN).unwrap();
        assert_eq!(
            token.hash(),
            "66d34fba71f8f450f7e45598853e53bfc23bbd129027cbb131a2f4ffd7878cd0"
        );
    }

    #[test]
    fn debug_hides_the_secret() {
        let token = SessionToken::parse(TOKEN).unwrap();
        assert!(!format!("{token:?}").contains(TOKEN));
    }

    #[test]
    fn the_cookie_is_http_only_secure_and_lax() {
        let token = SessionToken::parse(TOKEN).unwrap();
        let cookie = SessionCookie::new("game_session").cookie(&token);
        assert_eq!(cookie.name(), "game_session");
        assert_eq!(cookie.value(), TOKEN);
        assert_eq!(cookie.http_only(), Some(true));
        assert_eq!(cookie.secure(), Some(true));
        assert_eq!(cookie.same_site(), Some(SameSite::Lax));
        assert_eq!(cookie.path(), Some("/"));
        assert_eq!(cookie.max_age(), Some(Duration::days(365)));

        let plain = SessionCookie::new("game_session")
            .secure(false)
            .cookie(&token);
        assert_eq!(plain.secure(), Some(false));
    }

    #[test]
    fn the_removal_cookie_expires_at_once() {
        let cookie = SessionCookie::new("game_session").removal();
        assert_eq!(cookie.value(), "");
        assert_eq!(cookie.max_age(), Some(Duration::ZERO));
    }

    async fn whoami(token: SessionToken) -> String {
        token.hash()
    }

    async fn start(cookie: SessionCookie) -> HttpResponse {
        HttpResponse::Ok()
            .cookie(cookie.cookie(&SessionToken::generate()))
            .finish()
    }

    async fn maybe(token: Option<SessionToken>) -> HttpResponse {
        match token {
            Some(_) => HttpResponse::Ok().body("session"),
            None => HttpResponse::Ok().body("none"),
        }
    }

    #[actix_web::test]
    async fn the_extractor_reads_the_registered_cookie() {
        let app = actix_test::init_service(
            App::new()
                .app_data(SessionCookie::new("game_session"))
                .route("/whoami", web::get().to(whoami))
                .route("/maybe", web::get().to(maybe))
                .route("/start", web::post().to(start)),
        )
        .await;
        let token = SessionToken::parse(TOKEN).unwrap();

        let request = actix_test::TestRequest::get()
            .uri("/whoami")
            .cookie(Cookie::new("game_session", TOKEN))
            .to_request();
        let body = actix_test::call_and_read_body(&app, request).await;
        assert_eq!(body, token.hash());

        for cookie in [
            None,
            Some(Cookie::new("game_session", "forged")),
            Some(Cookie::new("other", TOKEN)),
        ] {
            let mut request = actix_test::TestRequest::get().uri("/whoami");
            if let Some(cookie) = cookie {
                request = request.cookie(cookie);
            }
            let response = actix_test::call_service(&app, request.to_request()).await;
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }

        let request = actix_test::TestRequest::get().uri("/maybe").to_request();
        assert_eq!(actix_test::call_and_read_body(&app, request).await, "none");

        let request = actix_test::TestRequest::post().uri("/start").to_request();
        let response = actix_test::call_service(&app, request).await;
        let given = response.response().cookies().next().unwrap();
        assert_eq!(given.name(), "game_session");
        assert!(SessionToken::parse(given.value()).is_some());
    }

    #[actix_web::test]
    async fn without_a_registered_cookie_the_extractor_fails_loudly() {
        let app =
            actix_test::init_service(App::new().route("/whoami", web::get().to(whoami))).await;
        let request = actix_test::TestRequest::get()
            .uri("/whoami")
            .cookie(Cookie::new("game_session", TOKEN))
            .to_request();
        let response = actix_test::call_service(&app, request).await;
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}

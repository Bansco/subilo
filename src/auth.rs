use actix_web::dev::Payload;
use actix_web::dev::ServiceRequest;
use actix_web::HttpRequest;
use actix_web_httpauth::extractors::bearer::{BearerAuth, Config};
use actix_web_httpauth::extractors::AuthenticationError;
use futures::future;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use super::Context;
use super::SubiloError;

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    permissions: Vec<String>,
}

impl User {
    pub fn has_permission(&self, permission: String) -> bool {
        self.permissions.contains(&permission)
    }
}

impl actix_web::FromRequest for User {
    type Config = ();
    type Error = SubiloError;
    type Future = future::Ready<Result<User, SubiloError>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let context = req.app_data::<actix_web::web::Data<Context>>();
        if context.is_none() {
            return future::err(SubiloError::ReadContext {});
        }

        let token = req
            .headers()
            .get("authorization")
            .unwrap()
            .to_str()
            .ok()
            .unwrap()
            .replace("Bearer ", "");

        let token_result = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(&context.unwrap().secret.as_bytes()),
            &Validation::new(Algorithm::HS512),
        );

        match token_result {
            Ok(token) => future::ok(token.claims.user),
            Err(err) => future::err(SubiloError::Authenticate { source: err }),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    exp: usize,  // Expiration time (as UTC timestamp)
    iat: usize,  // Issued at (as UTC timestamp)
    iss: String, // Issuer
    user: User,
}

pub fn create_token(
    secret: &str,
    permissions: Vec<String>,
    duration: i64,
) -> Result<String, SubiloError> {
    let header = Header::new(Algorithm::HS512);
    let user = User { permissions };
    let claims = Claims {
        exp: (chrono::Local::now() + chrono::Duration::minutes(duration)).timestamp() as usize,
        iat: chrono::Local::now().timestamp() as usize,
        iss: "subilo:agent".to_owned(),
        user,
    };

    encode(
        &header,
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|err| SubiloError::Authenticate { source: err })
}

// TODO: Migrate result to SubiloError and handle app data context unwrap.
pub async fn validator(
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, actix_web::Error> {
    let context = req.app_data::<Context>().unwrap();

    let config = req
        .app_data::<Config>()
        .map(|data| data.get_ref().clone())
        .unwrap_or_else(Default::default);

    let token = credentials.token();
    let token_result = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(&context.secret.as_bytes()),
        &Validation::new(Algorithm::HS512),
    );

    token_result
        .map(|_| req)
        .map_err(|_| AuthenticationError::from(config).into())
}

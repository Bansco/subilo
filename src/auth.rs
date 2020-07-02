use actix_web::dev::ServiceRequest;
use actix_web_httpauth::extractors::bearer::{BearerAuth, Config};
use actix_web_httpauth::extractors::AuthenticationError;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use super::Context;
use super::SubiloError;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    company: String,
    exp: usize,
}

pub fn create_token(secret: &str) -> Result<String, SubiloError> {
    let claims = Claims {
        sub: "subilo:agent".to_owned(),
        company: "subilo".to_owned(),
        // TODO: Move exp to configuration
        exp: 10_000_000_000,
    };

    let mut header = Header::default();
    header.alg = Algorithm::HS512;

    encode(
        &header,
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    ).map_err(|err| SubiloError::Authenticate { source: err })
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

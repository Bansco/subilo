use actix_web::dev::ServiceRequest;
use actix_web::Error;
use actix_web_httpauth::extractors::bearer::{BearerAuth, Config};
use actix_web_httpauth::extractors::AuthenticationError;
use serde::{Deserialize, Serialize};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};

use super::Context;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    company: String,
    exp: usize,
}

pub fn create_token() -> Result<String, jsonwebtoken::errors::Error> {
    let my_claims = Claims {
        sub: "thresh:agent".to_owned(),
        company: "thresh".to_owned(),
        exp: 10000000000,
    };
    let key = b"secret";

    let mut header = Header::default();
    header.alg = Algorithm::HS512;

    encode(&header, &my_claims, &EncodingKey::from_secret(key))
}

pub async fn validator(req: ServiceRequest, credentials: BearerAuth) -> Result<ServiceRequest, Error> {
    let context = req
        .app_data::<Context>()
        .unwrap();

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

    match token_result {
        Ok(_) => Ok(req),
        Err(_) => Err(AuthenticationError::from(config).into())
    }
}
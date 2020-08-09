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

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub enum Permissions {
    #[serde(rename = "job:write")]
    JobWrite,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    permissions: Vec<Permissions>,
}

impl User {
    pub fn has_permission(&self, permission: Permissions) -> bool {
        self.permissions.contains(&permission)
    }
}

impl actix_web::FromRequest for User {
    type Config = ();
    type Error = SubiloError;
    type Future = future::Ready<Result<User, SubiloError>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let token_result = req
            .app_data::<actix_web::web::Data<Context>>()
            .ok_or(SubiloError::ReadContext {})
            .and_then(|context| {
                let token = req
                    .headers()
                    .get("authorization")
                    .and_then(|header| header.to_str().ok())
                    .map(|s| s.replace("Bearer ", ""))
                    .ok_or_else(|| SubiloError::MissingToken {})?;

                decode::<Claims>(
                    &token,
                    &DecodingKey::from_secret(&context.secret.as_bytes()),
                    &Validation::new(Algorithm::HS512),
                )
                .map_err(|err| SubiloError::Authenticate { source: err })
            });

        match token_result {
            Ok(token) => future::ok(token.claims.user),
            Err(err) => future::err(err),
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
    permissions: Vec<Permissions>,
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

pub async fn validator(
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, actix_web::Error> {
    let context = req.app_data::<Context>().expect("Failed to read context on validator");

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

use sha1::Sha1;
use hmac::{Hmac, Mac, NewMac};

// https://developer.github.com/webhooks/securing/
// https://github.com/rust-lang/triagebot/blob/master/src/payload.rs
// https://github.com/qubyte/github_webhook_message_validator/blob/master/src/lib.rs
// https://docs.rs/hmac/0.8.0/hmac/
pub fn validate(secret: &[u8], signature: &[u8], body: &[u8]) -> bool {
    let signature = signature.get("sha1=".len()..).unwrap();
    let mut hmac = Hmac::<Sha1>::new_varkey(secret).unwrap();
    hmac.update(body);
    
    let result = hmac.finalize();
    // To get underlying array use `code` method, but be careful, since
    // incorrect use of the code value may permit timing attacks which defeat
    // the security provided by the `Output`
    let code_bytes = result.into_bytes();

    println!("code_bytes {:?}", code_bytes);

    // hmac.verify(signature).is_ok()
    false
}
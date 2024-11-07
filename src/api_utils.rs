use serde::{Deserialize, Serialize};

pub const HTTPS_API_FF_URL: &str = "https://tradernet.com/api/check-login-password";
pub const WS_API_FF_URL: &str = "wss://wss.tradernet.com/";
pub const FF_GET_SID: &str = "/api/check-login-password";
pub const FF_SID: &str = "?SID=";

#[derive(Serialize, Deserialize)]
pub struct AuthMessage {
    login: String,
    password: String,
    rememberMe: i32,
}
impl AuthMessage {
    pub(crate) fn new(login: &str, password: &str) -> Self {
        AuthMessage {
            login: login.to_owned(),
            password: password.to_owned(),
            rememberMe: 1,
        }
    }
}

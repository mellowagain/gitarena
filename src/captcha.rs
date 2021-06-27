use crate::CONFIG;

use std::borrow::Borrow;

use anyhow::{Context, Result};
use log::{error, warn};
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub(crate) async fn verify_captcha(token: &String) -> Result<bool> {
    if cfg!(debug_assertions) {
        return Ok(true);
    }

    let api_key: &str = CONFIG.hcaptcha.secret.borrow();

    let response: HCaptchaResponse = Client::new()
        .post("https://hcaptcha.com/siteverify")
        .form(&[("response", token), ("secret", &api_key.to_owned())])
        .send()
        .await
        .context("Unable to verify hCaptcha captcha token.")?
        .json()
        .await
        .context("Unable to convert hCaptcha response into Json structure.")?;

    if let Some(errors) = response.errors {
        let errors_str = errors.join(", ");
        error!("hCaptcha failed to verify challenge token: {}", errors_str);
    }

    if let Some(credit) = response.credit {
        if !credit {
            warn!("Credit was not earned for captcha response.");
        }
    }

    Ok(response.success)
}

#[derive(Serialize, Deserialize)]
struct HCaptchaResponse {
    success: bool,
    challenge_ts: Option<String>,
    hostname: Option<String>,
    credit: Option<bool>,
    #[serde(rename(deserialize = "error-codes"))]
    errors: Option<Vec<String>>
}

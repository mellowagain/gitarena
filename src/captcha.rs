use crate::config::get_optional_setting;

use anyhow::{Context, Result};
use log::{error, warn};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::{Executor, Postgres};
use tokio_compat_02::FutureExt;

pub(crate) async fn verify_captcha<'e, E: Executor<'e, Database = Postgres>>(token: &String, executor: E) -> Result<bool> {
    let api_key = match get_optional_setting::<String, _>("hcaptcha.site_key", executor).await? {
        Some(api_key) => api_key,
        None => return Ok(true)
    };

    let response: HCaptchaResponse = Client::new()
        .post("https://hcaptcha.com/siteverify")
        .form(&[("response", token), ("secret", &api_key)])
        .send()
        .compat()
        .await
        .context("Unable to verify hCaptcha captcha token.")?
        .json()
        .compat()
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

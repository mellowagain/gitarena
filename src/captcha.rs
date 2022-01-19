use crate::config::get_optional_setting;
use crate::err;
use crate::prelude::AwcExtensions;

use anyhow::Result;
use awc::Client;
use log::{error, warn};
use serde::{Deserialize, Serialize};
use sqlx::{Executor, Postgres};

pub(crate) async fn verify_captcha<'e, E: Executor<'e, Database = Postgres>>(token: &String, executor: E) -> Result<bool> {
    let api_key = match get_optional_setting::<String, _>("hcaptcha.site_key", executor).await? {
        Some(api_key) => api_key,
        None => return Ok(true)
    };

    let response: HCaptchaResponse = Client::gitarena()
        .post("https://hcaptcha.com/siteverify")
        .send_form(&[("response", token), ("secret", &api_key)])
        .await
        .map_err(|err| err!(BAD_GATEWAY, "Unable to verify hCaptcha captcha token: {}", err))?
        .json()
        .await
        .map_err(|err| err!(BAD_GATEWAY, "Unable to convert hCaptcha response into Json structure: {}", err))?;

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

use crate::config::get_optional_setting;
use crate::err;
use crate::prelude::AwcExtensions;

use crate::database::Database;
use anyhow::Result;
use awc::Client;
use serde::{Deserialize, Serialize};
use sqlx::Transaction;
use tracing::{error, instrument, warn};

#[instrument(err, skip(tx))]
pub(crate) async fn verify_captcha(token: &String, tx: &mut Transaction<'_, Database>) -> Result<bool> {
    let Some(api_key) = get_optional_setting::<String>("hcaptcha.site_key", tx).await? else {
        return Ok(true);
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
        error!(err = errors_str, "hCaptcha failed to verify challenge token");
    }

    if let Some(credit) = response.credit
        && !credit
    {
        warn!("Credit was not earned for captcha response.");
    }

    Ok(response.success)
}

#[derive(Serialize, Deserialize)]
struct HCaptchaResponse {
    success: bool,
    challenge_ts: Option<String>,
    hostname: Option<String>,
    credit: Option<bool>,
    #[serde(rename = "error-codes")]
    errors: Option<Vec<String>>,
}

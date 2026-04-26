use std::collections::HashMap;
use std::sync::OnceLock;

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use gitarena_common::database::Database;
use serde_cbor_2::Value as CborValue;
use serde_json::Value;
use sqlx::{FromRow, Transaction};
use url::Url;
use uuid::Uuid;
use webauthn_rs::prelude::{DiscoverableAuthentication, Passkey, PasskeyRegistration, Webauthn, WebauthnBuilder};

static AAGUID_NAMES: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();

fn aaguid_map() -> &'static HashMap<&'static str, &'static str> {
    AAGUID_NAMES.get_or_init(|| serde_json::from_str(include_str!("aaguids.json")).expect("aaguids.json is valid JSON"))
}

pub(crate) fn aaguid_from_raw_credential(raw_credential: &Value) -> Option<String> {
    let attest_b64 = raw_credential.pointer("/response/attestationObject").and_then(|v| v.as_str())?;

    let attest_bytes = base64::decode_config(attest_b64, base64::URL_SAFE_NO_PAD).ok()?;

    // Parse CBOR: { "fmt": ..., "attStmt": ..., "authData": bytes }
    let cbor_val: CborValue = serde_cbor_2::from_slice(&attest_bytes).ok()?;

    let auth_data: Vec<u8> = match cbor_val {
        CborValue::Map(map) => map
            .into_iter()
            .find(|(k, _)| matches!(k, CborValue::Text(s) if s == "authData"))
            .and_then(|(_, v)| match v {
                CborValue::Bytes(b) => Some(b),
                _ => None,
            }),
        _ => None,
    }?;

    // authenticatorData layout:
    //   [0..32]  rpIdHash
    //   [32]     flags  (bit 6 = AT: attested credential data present)
    //   [33..37] signCount
    //   [37..53] AAGUID (16 bytes) – present only when AT flag is set
    if auth_data.len() < 53 {
        return None;
    }

    let flags = auth_data[32];
    if flags & 0x40 == 0 {
        return None;
    }

    let aaguid_bytes = &auth_data[37..53];
    if aaguid_bytes.iter().all(|&b| b == 0) {
        return None;
    }

    let uuid = Uuid::from_slice(aaguid_bytes).ok()?;
    let aaguid_str = uuid.to_string();

    aaguid_map().get(aaguid_str.as_str()).map(ToString::to_string)
}

pub(crate) fn name_from_user_agent(ua: &str) -> String {
    let os = if ua.contains("iPhone") || ua.contains("iPad") {
        "iOS"
    } else if ua.contains("Android") {
        "Android"
    } else if ua.contains("Macintosh") || ua.contains("Mac OS X") {
        "macOS"
    } else if ua.contains("Windows") {
        "Windows"
    } else if ua.contains("Linux") {
        "Linux"
    } else {
        "Unknown OS"
    };

    let browser = if ua.contains("Edg/") || ua.contains("EdgA/") {
        "Edge"
    } else if ua.contains("Chrome") {
        "Chrome"
    } else if ua.contains("Firefox") {
        "Firefox"
    } else if ua.contains("Safari") {
        "Safari"
    } else {
        "Browser"
    };

    format!("{browser} on {os}")
}

pub(crate) fn build_webauthn(domain: &str, origin: Option<&str>) -> Result<Webauthn> {
    let rp_origin_url = Url::parse(origin.unwrap_or(domain))?;
    let domain_url = Url::parse(domain)?;

    let rp_id = domain_url
        .domain()
        .ok_or_else(|| anyhow!("Domain setting does not contain a valid hostname: {domain}"))?;

    WebauthnBuilder::new(rp_id, &rp_origin_url)
        .map_err(|e| anyhow!("Failed to build Webauthn instance: {e:?}"))?
        .rp_name("GitArena")
        .build()
        .map_err(|e| anyhow!("Failed to build Webauthn instance: {e:?}"))
}

#[derive(FromRow, Debug)]
pub(crate) struct StoredPasskey {
    pub(crate) id: Uuid,
    pub(crate) user_id: i32,
    pub(crate) name: String,
    pub(crate) credential: Value,
}

impl StoredPasskey {
    pub(crate) fn into_passkey(self) -> Result<Passkey> {
        serde_json::from_value(self.credential).map_err(|e| anyhow!("Failed to deserialise passkey credential: {e}"))
    }

    pub(crate) async fn find_for_user(user_id: i32, tx: &mut Transaction<'_, Database>) -> Result<Vec<StoredPasskey>> {
        let rows = sqlx::query_as::<_, StoredPasskey>("select * from passkeys where user_id = $1")
            .bind(user_id)
            .fetch_all(&mut **tx)
            .await?;

        Ok(rows)
    }

    pub(crate) async fn find_user_id_by_cred_id(cred_id_b64: &str, tx: &mut Transaction<'_, Database>) -> Result<Option<i32>> {
        let row: Option<(i32,)> = sqlx::query_as("select user_id from passkeys where credential->'cred'->>'cred_id' = $1 limit 1")
            .bind(cred_id_b64)
            .fetch_optional(&mut **tx)
            .await?;

        Ok(row.map(|(id,)| id))
    }
}

#[derive(FromRow, Debug)]
pub(crate) struct WebAuthnChallenge {
    pub(crate) id: Uuid,
    pub(crate) user_id: Option<i32>,
    pub(crate) state: serde_json::Value,
    pub(crate) expires_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::Type, serde::Serialize, serde::Deserialize)]
#[sqlx(type_name = "webauthn_challenge_type", rename_all = "lowercase")]
pub(crate) enum ChallengeType {
    Registration,
    Authentication,
}

impl WebAuthnChallenge {
    pub(crate) async fn insert_registration(id: Uuid, user_id: i32, state: &PasskeyRegistration, tx: &mut Transaction<'_, Database>) -> Result<()> {
        let state_json = serde_json::to_value(state)?;
        sqlx::query(
            "insert into webauthn_challenges (id, user_id, state, type, expires_at) \
             values ($1, $2, $3, 'registration', now() + interval '5 minutes')",
        )
        .bind(id)
        .bind(user_id)
        .bind(state_json)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    pub(crate) async fn insert_authentication(id: Uuid, state: &DiscoverableAuthentication, tx: &mut Transaction<'_, Database>) -> Result<()> {
        let state_json = serde_json::to_value(state)?;
        sqlx::query(
            "insert into webauthn_challenges (id, user_id, state, type, expires_at) \
             values ($1, null, $2, 'authentication', now() + interval '5 minutes')",
        )
        .bind(id)
        .bind(state_json)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    pub(crate) async fn take(id: Uuid, expected_type: ChallengeType, tx: &mut Transaction<'_, Database>) -> Result<Option<WebAuthnChallenge>> {
        let row: Option<WebAuthnChallenge> = sqlx::query_as::<_, WebAuthnChallenge>(
            "delete from webauthn_challenges \
             where id = $1 and type = $2 and expires_at > now() \
             returning id, user_id, state, expires_at",
        )
        .bind(id)
        .bind(expected_type)
        .fetch_optional(&mut **tx)
        .await?;

        Ok(row)
    }
}

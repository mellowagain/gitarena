use crate::config::{get_setting, get_settings_like};
use crate::database::Pool;
use crate::sso::oidc::Oidc;
use anyhow::{Result, anyhow};
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::sync::OnceCell;
use tracing::{debug, error, info, instrument};

pub(crate) static SSO_PROVIDERS: OnceCell<Vec<SsoProviderType>> = OnceCell::const_new();

pub(crate) trait SsoProvider {
    type State: DeserializeOwned + Serialize + Send + Unpin;

    async fn auth_url(&self) -> Result<ProviderAuthUrlReturn<Self::State>>;
    async fn exchange(&self, auth_code: String, state: Self::State) -> Result<ProviderExchangeReturn>;

    fn name(&self) -> String;
}

pub(crate) struct ProviderAuthUrlReturn<T: DeserializeOwned + Serialize> {
    pub(crate) csrf_token: String,
    pub(crate) auth_url: String,
    pub(crate) state: T,
}

pub(crate) struct ProviderExchangeReturn {
    pub(crate) subject_id: String,
    pub(crate) username: String,
    pub(crate) emails: Vec<String>,
    pub(crate) emails_verified: bool,
}

#[derive(Clone)]
pub(crate) enum SsoProviderType {
    OAuth2,
    Oidc(Box<Oidc>),
}

impl SsoProviderType {
    pub(crate) fn name(&self) -> String {
        match self {
            SsoProviderType::OAuth2 => todo!(),
            SsoProviderType::Oidc(oidc) => oidc.name(),
        }
    }
}

#[instrument(skip(db_pool))]
pub(crate) async fn init(db_pool: &Pool) -> Result<()> {
    debug!("loading sso providers");

    let mut tx = db_pool.begin().await?;

    let settings = get_settings_like("sso.%.%.client_id", &mut tx).await?;
    let mut providers = Vec::with_capacity(settings.len());

    for setting in settings {
        if let Some(client_id) = setting.value {
            let mut split = setting.key.split('.');

            let _ = split.next();
            let provider_type = split.next().expect("postgres to follow our pattern");
            let name = split.next().expect("postgres to follow our pattern");

            let client_secret: String = get_setting(&format!("sso.{provider_type}.{name}.client_secret"), &mut tx).await?;

            match provider_type {
                "oauth2" => todo!(),
                "oidc" => {
                    let issuer_url: String = get_setting(&format!("sso.{provider_type}.{name}.issuer_url"), &mut tx).await?;
                    let scopes: String = get_setting(&format!("sso.{provider_type}.{name}.scopes"), &mut tx).await?;

                    let oidc = Oidc::new(
                        name.to_string(),
                        issuer_url,
                        client_id,
                        client_secret,
                        scopes.split(',').map(ToString::to_string).collect(),
                        &mut tx,
                    )
                    .await?;

                    providers.push(SsoProviderType::Oidc(Box::new(oidc)));

                    info!(type = "oidc", %name, "initialized sso provider");
                }
                ty => error!(type = ty, "unknown provider type in sso settings"),
            }
        }
    }

    SSO_PROVIDERS
        .set(providers)
        .map_err(|_| anyhow!("sso providers should only be initialized once"))?;

    tx.commit().await?;
    Ok(())
}

pub(crate) fn get(name: &str) -> Option<impl SsoProvider + use<>> {
    SSO_PROVIDERS.get().and_then(|providers| {
        let name = name.to_lowercase();

        providers.iter().find(|&p| p.name().to_lowercase() == name).cloned().map(|ty| match ty {
            SsoProviderType::OAuth2 => todo!(),
            SsoProviderType::Oidc(oidc) => *oidc,
        })
    })
}

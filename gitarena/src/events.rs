use crate::database::{Database, Pool};
use crate::organization::Organization;
use crate::repository::Repository;
use crate::session;
use crate::user::User;
use actix_web::HttpRequest;
use anyhow::{Result, anyhow};
use ipnetwork::IpNetwork;
use opentelemetry::trace::{TraceContextExt, TraceId};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sqlx::{FromRow, Transaction};
use tokio::sync::OnceCell;
use tracing::{Span, instrument};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use utoipa::ToSchema;
use uuid::Uuid;

pub(crate) static SYSTEM_USER: OnceCell<Uuid> = OnceCell::const_new();

pub(crate) async fn init(db_pool: &Pool) -> Result<()> {
    let mut tx = db_pool.begin().await?;

    let user = User::find_using_name("gitarena", &mut tx)
        .await
        .ok_or_else(|| anyhow!("system user named `gitarena` not found"))?;

    let _ = SYSTEM_USER.set(user.id);

    tx.commit().await?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, FromRow, ToSchema)]
pub(crate) struct Event {
    /// ID
    pub(crate) id: Uuid,

    #[serde(skip)]
    pub(crate) trace_id: Option<Uuid>,

    /// Actor user ID
    /// May be nil UUID if no user id exists for the triggering users (only on `auth.login_failed` and maybe on `email.verified` and `git.*`)
    pub(crate) actor_id: Uuid,
    /// IP Address
    pub(crate) ip_address: Option<IpNetwork>,
    /// User agent
    pub(crate) user_agent: Option<String>,

    /// Subject user id
    pub(crate) subject_id_user: Option<Uuid>,
    /// Subject organization id
    pub(crate) subject_id_org: Option<Uuid>,
    /// Subject repository id
    pub(crate) subject_id_repo: Option<Uuid>,

    /// Class
    pub(crate) class: EventClass,
    /// Type
    #[sqlx(rename = "type")]
    #[serde(rename = "type")]
    pub(crate) type_: String,
    /// Payload
    pub(crate) payload: Value,
}

impl Event {
    #[must_use]
    pub(crate) fn new(event: &'static str, actor: Uuid, request: &HttpRequest, subject: Subject, payload: Option<Value>) -> Self {
        let (ip_network, user_agent) = session::extract_ip_and_ua_owned(request);

        let mut event = Self::new_without_request(event, actor, subject, payload);
        event.ip_address = Some(ip_network);
        event.user_agent = Some(user_agent);

        event
    }

    #[must_use]
    pub(crate) fn new_without_actor(event: &'static str, request: &HttpRequest, subject: Subject, payload: Option<Value>) -> Self {
        let uuid = SYSTEM_USER.get().expect("system user to be filled at startup");
        Self::new(event, *uuid, request, subject, payload)
    }

    #[must_use]
    pub(crate) fn new_without_request(event: &'static str, actor: Uuid, subject: Subject, payload: Option<Value>) -> Self {
        let span = Span::current();
        let trace_id = if span.is_disabled() || span.is_none() {
            None
        } else {
            let otel_trace_id = span.context().span().span_context().trace_id();
            if otel_trace_id != TraceId::INVALID {
                Some(Uuid::from_u128(u128::from_be_bytes(otel_trace_id.to_bytes())))
            } else {
                None
            }
        };

        Self {
            id: Uuid::now_v7(),
            trace_id,
            actor_id: actor,
            ip_address: None,
            user_agent: None,
            subject_id_user: subject.user(),
            subject_id_org: subject.org(),
            subject_id_repo: subject.repo(),
            class: EventClass::from_event(event),
            type_: event.to_string(),
            payload: payload.unwrap_or_else(|| Value::Object(Map::new())),
        }
    }

    #[must_use]
    pub(crate) async fn new_without_actor_and_request(event: &'static str, subject: Subject, payload: Option<Value>) -> Self {
        let uuid = SYSTEM_USER.get().expect("system user to be filled at startup");
        Self::new_without_request(event, *uuid, subject, payload)
    }

    #[instrument(err, skip(tx))]
    pub(crate) async fn save(self, tx: &mut Transaction<'_, Database>) -> Result<()> {
        sqlx::query(
            "insert into events (id, trace_id, actor_id, ip_address, user_agent, subject_id_user, subject_id_org, subject_id_repo, class, type, payload) \
             values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(self.id)
        .bind(self.trace_id)
        .bind(self.actor_id)
        .bind(self.ip_address)
        .bind(self.user_agent)
        .bind(self.subject_id_user)
        .bind(self.subject_id_org)
        .bind(self.subject_id_repo)
        .bind(self.class)
        .bind(self.type_)
        .bind(self.payload)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    pub(crate) async fn save_pool(self, db_pool: &Pool) -> Result<()> {
        let mut tx = db_pool.begin().await?;

        self.save(&mut tx).await?;

        tx.commit().await?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "event_class", rename_all = "lowercase")]
pub(crate) enum EventClass {
    /// Events a user should review upon account compromise
    Security,
    /// Events used to build timelines for the profile or dashboard etc.
    Activity,
    /// Events triggered without any human behind it, e.g. cron jobs
    System,
}

impl EventClass {
    fn from_event(event: &'static str) -> Self {
        match event {
            "user.disabled" => EventClass::Security,
            "repo.visibility_changed" => EventClass::Security,
            "repo.transferred" => EventClass::Security,
            _ if event.starts_with("auth.") => EventClass::Security,
            _ if event.starts_with("session.") => EventClass::Security,
            _ if event.starts_with("ssh_key.") => EventClass::Security,
            _ if event.starts_with("passkey.") => EventClass::Security,
            _ if event.starts_with("email.") => EventClass::Security,
            _ if event.starts_with("privilege.") => EventClass::Security,

            "user.created" => EventClass::Activity,
            "user.updated" => EventClass::Activity,
            "user.deleted" => EventClass::Activity,
            "repo.created" => EventClass::Activity,
            "repo.updated" => EventClass::Activity,
            "repo.deleted" => EventClass::Activity,
            "repo.archived" => EventClass::Activity,
            "repo.unarchived" => EventClass::Activity,
            "repo.forked" => EventClass::Activity,
            "repo.mirrored" => EventClass::Activity,
            _ if event.starts_with("org.") => EventClass::Activity,
            _ if event.starts_with("star.") => EventClass::Activity,
            _ if event.starts_with("issue.") => EventClass::Activity,
            _ if event.starts_with("git.") => EventClass::Activity,

            _ => unimplemented!("unknown event {event}. please implement in `EventClass::from_event`"),
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub(crate) enum Subject {
    User(Uuid),
    Org(Uuid),
    Repo(Uuid),
}

impl Subject {
    fn user(&self) -> Option<Uuid> {
        match self {
            Subject::User(uuid) => Some(*uuid),
            _ => None,
        }
    }

    fn org(&self) -> Option<Uuid> {
        match self {
            Subject::Org(uuid) => Some(*uuid),
            _ => None,
        }
    }

    fn repo(&self) -> Option<Uuid> {
        match self {
            Subject::Repo(uuid) => Some(*uuid),
            _ => None,
        }
    }
}

impl From<&User> for Subject {
    fn from(value: &User) -> Self {
        Subject::User(value.id)
    }
}

impl From<&Organization> for Subject {
    fn from(value: &Organization) -> Self {
        Subject::Org(value.id)
    }
}

impl From<&Repository> for Subject {
    fn from(value: &Repository) -> Self {
        Subject::Repo(value.id)
    }
}

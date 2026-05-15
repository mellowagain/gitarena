use crate::config::get_setting;
use crate::database::Pool;
use crate::die;
use crate::mail::TRANSPORTER;
use crate::ssh::SSH_TASK_HANDLE;
use crate::user::WebUser;
use crate::utils::time_function;
use actix_web::{HttpResponse, Responder, web};
use anyhow::{Context, Result};
use fang::{AsyncQueue, Serialize};
use futures::future;
use gitarena_macros::route;
use std::env;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::task::JoinSet;
use tokio::time::timeout;
use tracing::error;
use utoipa::ToSchema;

#[utoipa::path(
    get,
    path = "/api/admin/health",
    responses(
        (status = 201, description = "Instance health", body = InstanceHealth),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Admin required"),
    ),
    security(("cookieAuth" = [])),
    tag = "admin"
)]
#[route("/api/admin/health", method = "GET", err = "json")]
pub(crate) async fn get_instance_health(web_user: WebUser, queue: web::Data<AsyncQueue>, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    if !user.admin {
        die!(FORBIDDEN, "Admin endpoints can only be called by admins");
    }

    // https://stackoverflow.com/a/69424585/11494565
    // todo: add object storage and redis once thats implemented
    let mut set = JoinSet::new();

    set.spawn({
        let db_pool = db_pool.clone();
        async move { check_database(&db_pool).await }
    });

    set.spawn({
        let db_pool = db_pool.clone();
        async move { check_ssh(&db_pool).await }
    });

    set.spawn(future::ready(Ok(check_workers(&queue))));
    set.spawn(check_email());

    let mut components = Vec::with_capacity(set.len());

    for task in set.join_all().await {
        components.push(task?);
    }

    components.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(HttpResponse::Ok().json(InstanceHealth { components }))
}

async fn check_database(db_pool: &Pool) -> Result<InstanceComponent> {
    let mut tx = db_pool.begin().await?;

    let (latency, result) = time_function(|| async { sqlx::query("select 1").execute(&mut *tx).await }).await;

    let status = match result {
        Ok(result) if result.rows_affected() == 1 => ComponentStatus::Healthy,
        Ok(result) => {
            // pretty sure this can never happen but better be safe ig
            ComponentStatus::Degraded(format!("database returned {} rows but expected 1", result.rows_affected()))
        }
        Err(err) => {
            error!(?err, "database health check failed");
            ComponentStatus::Unhealthy
        }
    };

    tx.commit().await?;

    Ok(InstanceComponent {
        name: "Database".to_string(),
        status,
        latency: Some(latency),
    })
}

async fn check_ssh(db_pool: &Pool) -> Result<InstanceComponent> {
    let (latency, status) = if let Some(handle) = SSH_TASK_HANDLE.get() {
        if handle.is_finished() {
            (None, ComponentStatus::Unhealthy)
        } else {
            let port = {
                let mut tx = db_pool.begin().await?;

                let port: i32 = get_setting("ssh.port", &mut tx).await?;

                tx.commit().await?;
                u16::try_from(port).context("port needs to be within 0-65535")?
            };

            let bind_address = env::var("BIND_ADDRESS").expect("BIND_ADDRESS env var to be set at this point");
            let address = bind_address
                .rsplit_once(':')
                .map_or_else(|| bind_address.clone(), |(address, _)| address.to_string());

            match timeout(
                Duration::from_secs(5),
                time_function(|| async { TcpStream::connect((address.as_str(), port)).await }),
            )
            .await
            {
                Ok((latency, Ok(_))) => (Some(latency), ComponentStatus::Healthy),
                Ok((latency, Err(err))) => (Some(latency), ComponentStatus::Degraded(err.to_string())),
                Err(_) => (None, ComponentStatus::Degraded("5sec time out reached trying to connect to ssh".to_string())),
            }
        }
    } else {
        (None, ComponentStatus::Disabled)
    };

    Ok(InstanceComponent {
        name: "SSH".to_string(),
        status,
        latency,
    })
}

fn check_workers(queue: &AsyncQueue) -> InstanceComponent {
    let status = if queue.check_if_connection().is_ok() {
        ComponentStatus::Healthy
    } else {
        ComponentStatus::Unhealthy
    };

    InstanceComponent {
        name: "Workers".to_string(),
        status,
        latency: None,
    }
}

async fn check_email() -> Result<InstanceComponent> {
    let (latency, status) = if let Some(transporter) = TRANSPORTER.get() {
        let (latency, result) = time_function(async || transporter.test_connection().await).await;

        match result {
            Ok(true) => (Some(latency), ComponentStatus::Healthy),
            Ok(false) => (
                None,
                ComponentStatus::Degraded("email pool is connected but SMTP didnt return the NOOP".to_string()),
            ),
            Err(_) => (None, ComponentStatus::Unhealthy),
        }
    } else {
        (None, ComponentStatus::Disabled)
    };

    Ok(InstanceComponent {
        name: "Email".to_string(),
        status,
        latency,
    })
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InstanceHealth {
    components: Vec<InstanceComponent>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InstanceComponent {
    /// Name
    name: String,
    /// Status
    status: ComponentStatus,
    /// Latency in ms
    latency: Option<u64>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ComponentStatus {
    Healthy,
    Degraded(String),
    Unhealthy,
    Disabled,
}

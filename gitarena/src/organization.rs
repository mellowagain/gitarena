use crate::database::Database;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Transaction, Type};
use tracing::instrument;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(FromRow, Clone, Debug, Serialize, ToSchema)]
#[serde(rename_all(serialize = "camelCase"))]
pub(crate) struct Organization {
    /// ID
    pub(crate) id: Uuid,
    /// Name
    pub(crate) name: String,
    /// Description
    pub(crate) description: String,
}

impl Organization {
    #[instrument(skip(tx))]
    pub(crate) async fn find_by_name(name: &str, tx: &mut Transaction<'_, Database>) -> Option<Organization> {
        sqlx::query_as::<_, Organization>("select * from organizations where lower(name) = lower($1) limit 1")
            .bind(name)
            .fetch_optional(&mut **tx)
            .await
            .ok()
            .flatten()
    }

    #[instrument(skip(tx))]
    pub(crate) async fn find_by_id(id: Uuid, tx: &mut Transaction<'_, Database>) -> Option<Organization> {
        sqlx::query_as::<_, Organization>("select * from organizations where id = $1 limit 1")
            .bind(id)
            .fetch_optional(&mut **tx)
            .await
            .ok()
            .flatten()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Type, ToSchema)]
#[sqlx(type_name = "org_role", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub(crate) enum OrgRole {
    Owner,
    Admin,
    Member,
}

#[derive(FromRow, Clone, Debug, Serialize, ToSchema)]
#[serde(rename_all(serialize = "camelCase"))]
pub(crate) struct OrgMember {
    /// Organization ID
    pub(crate) org_id: Uuid,
    /// User ID
    pub(crate) user_id: Uuid,
    /// User role
    pub(crate) role: OrgRole,
}

impl OrgMember {
    pub(crate) async fn get_role(org_id: Uuid, user_id: Uuid, tx: &mut Transaction<'_, Database>) -> Result<Option<OrgRole>> {
        let row: Option<(OrgRole,)> = sqlx::query_as("select role from organization_members where org_id = $1 and user_id = $2 limit 1")
            .bind(org_id)
            .bind(user_id)
            .fetch_optional(&mut **tx)
            .await?;

        Ok(row.map(|(role,)| role))
    }

    pub(crate) fn has_permission(role: OrgRole, required: OrgRole) -> bool {
        match required {
            OrgRole::Member => true,
            OrgRole::Admin => matches!(role, OrgRole::Admin | OrgRole::Owner),
            OrgRole::Owner => role == OrgRole::Owner,
        }
    }
}

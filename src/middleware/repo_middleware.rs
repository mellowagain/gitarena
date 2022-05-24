use crate::err;
use crate::error::{ErrorDisplayType, GitArenaError};
use crate::repository::Repository;
use crate::user::User;

use std::sync::Arc;

use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::HttpMessage;
use actix_web::web::Data;
use actix_web_lab::middleware::Next;
use anyhow::anyhow;
use git_repository::refs::file::find::existing::Error as GitoxideFindError;
use git_repository::refs::file::loose::Reference;
use log::{debug, error};
use sqlx::PgPool;

pub(crate) async fn repo_middleware(req: ServiceRequest, next: Next<impl MessageBody>) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    let match_info = req.match_info();

    debug!("{:?}", match_info);
    debug!("unproceddsed pasrts: {:?}", match_info.unprocessed());

    for (indx, i) in match_info.iter().enumerate() {
        debug!("match info iter: #{} -> {:?}", indx, i);
    }

    debug!("{:?}", req.match_pattern());
    debug!("{:?}", req.match_name());

    let username = match_info.get("username");
    let repository = match_info.get("repository");
    let tree = match_info.get("tree");

    debug!("{:?} {:?} {:?}", username, repository, tree);

    if let (Some(username), Some(repository)) = (username, repository) {
        if let Some(db_pool) = req.app_data::<Data<PgPool>>() {
            let mut transaction = db_pool.begin().await.map_err(|err| transform_ga_error(anyhow!(err)))?;

            let user = User::find_using_name(username, &mut transaction).await.ok_or_else(|| transform_ga_error(err!(NOT_FOUND, "Repository not found")))?;
            let repo = Repository::open(user, repository, &mut transaction).await.ok_or_else(|| transform_ga_error(err!(NOT_FOUND, "Repository not found")))?;

            if let Some(tree) = tree {
                let gitoxide_repo = repo.gitoxide(&mut transaction).await.map_err(|err| transform_ga_error(anyhow!(err)))?;

                match gitoxide_repo.refs.find_loose(tree) {
                    Ok(tree) => {
                        req.extensions_mut().insert(Some(tree));
                    }
                    Err(GitoxideFindError::Find(err)) => {
                        error!("Failed to lookup branch: {}", err);
                        return Err(transform_ga_error(err!(NOT_FOUND, "Branch not found")));
                    }
                    Err(GitoxideFindError::NotFound(_)) => {
                        if tree == repo.default_branch {
                            req.extensions_mut().insert::<Option<Reference>>(None);
                        } else {
                            return Err(transform_ga_error(err!(NOT_FOUND, "Branch not found")));
                        }
                    }
                }
            }

            debug!("middleare inserted repo: {:?}", &repo);
            req.extensions_mut().insert(repo);

            transaction.commit().await.map_err(|err| transform_ga_error(anyhow!(err)))?;
        } else {
            return Err(transform_ga_error(anyhow!("No PgPool in application data")));
        }
    }

    debug!("calling enxt");
    next.call(req).await
}

/// Transforms [`anyhow::Error`] into [`actix_web::Error`]
#[inline]
fn transform_ga_error<E: Into<anyhow::Error>>(err: E) -> actix_web::Error {
    // TODO: Generalize this and put it into error.rs
    GitArenaError {
        source: Arc::new(err.into()),
        display_type: ErrorDisplayType::Html // TODO: Check whenever route is err = "html|json|git" etc...
    }.into()
}

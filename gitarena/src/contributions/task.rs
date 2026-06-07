use crate::TASK_DB_POOL;
use crate::contributions::insert_contributions;
use crate::prelude::MapToFangError;
use crate::repository::Repository;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{NaiveDate, TimeZone, Utc};
use fang::{AsyncQueueable, AsyncRunnable, FangError, typetag};
use git2::Sort;
use serde::{Deserialize, Serialize};
use tokio::task::spawn_blocking;
use tracing::{debug, info, instrument};
use uuid::Uuid;

pub(crate) static CONTRIBUTIONS_TASK_TYPE: &str = "contributions";

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "fang::serde")]
pub(crate) struct BackfillRepoContributionsTask {
    pub(crate) repo_id: Uuid,
}

#[async_trait]
#[typetag::serde]
impl AsyncRunnable for BackfillRepoContributionsTask {
    #[instrument(skip(_client))]
    async fn run(&self, _client: &dyn AsyncQueueable) -> Result<(), FangError> {
        let db_pool = TASK_DB_POOL.get().ok_or_else(|| FangError {
            description: "task db pool OnceCell is empty".to_string(),
        })?;

        let mut tx = db_pool.begin().await.fang()?;

        let repo: Option<Repository> = sqlx::query_as("select * from repositories where id = $1 limit 1")
            .bind(self.repo_id)
            .fetch_optional(&mut *tx)
            .await
            .fang()?;

        let Some(repo) = repo else {
            debug!(repo_id = %self.repo_id, "repo not found, skipping contribution backfill");
            return Ok(());
        };

        let path = repo.get_fs_path(&mut tx).await.fang()?;

        let commit_data = spawn_blocking(move || -> Result<Vec<(String, String, NaiveDate)>> {
            let git_repo = git2::Repository::open(&path)?;

            if git_repo.is_empty()? {
                return Ok(vec![]);
            }

            let mut revwalk = git_repo.revwalk()?;
            revwalk.set_sorting(Sort::TOPOLOGICAL)?;
            revwalk.push_glob("refs/heads/*")?;

            let mut commit_data = Vec::new();

            for result in &mut revwalk {
                let Ok(oid) = result else { continue };
                let Ok(commit) = git_repo.find_commit(oid) else { continue };

                let author = commit.author();
                let Some(email) = author.email() else { continue };

                let Some(dt) = Utc.timestamp_opt(author.when().seconds(), 0).single() else {
                    continue;
                };

                commit_data.push((oid.to_string(), email.to_lowercase(), dt.date_naive()));
            }

            Ok(commit_data)
        })
        .await
        .fang()?
        .fang()?;

        info!("backfilling contributions: {}", commit_data.len());

        insert_contributions(self.repo_id, &commit_data, &mut tx).await.fang()?;

        tx.commit().await.fang()?;

        debug!(repo.id = %self.repo_id, count = commit_data.len(), "contribution backfill complete for repo");
        Ok(())
    }

    fn task_type(&self) -> String {
        CONTRIBUTIONS_TASK_TYPE.to_string()
    }

    fn max_retries(&self) -> i32 {
        3
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "fang::serde")]
pub(crate) struct BackfillEmailContributionsTask {
    pub(crate) email: String,
    pub(crate) user_id: Uuid,
}

#[async_trait]
#[typetag::serde]
impl AsyncRunnable for BackfillEmailContributionsTask {
    #[instrument(skip(_client))]
    async fn run(&self, _client: &dyn AsyncQueueable) -> Result<(), FangError> {
        let db_pool = TASK_DB_POOL.get().ok_or_else(|| FangError {
            description: "task db pool OnceCell is empty".to_string(),
        })?;

        let mut tx = db_pool.begin().await.fang()?;

        let repos: Vec<Repository> = sqlx::query_as("select * from repositories").fetch_all(&mut *tx).await.fang()?;

        for repo in repos {
            let path = repo.get_fs_path(&mut tx).await.fang()?;
            let email = self.email.clone();

            let commit_data = spawn_blocking(move || -> Result<Vec<(String, String, NaiveDate)>> {
                let git_repo = git2::Repository::open(&path)?;

                if git_repo.is_empty()? {
                    return Ok(vec![]);
                }

                let mut revwalk = git_repo.revwalk()?;
                revwalk.set_sorting(Sort::TOPOLOGICAL)?;
                revwalk.push_glob("refs/heads/*")?;

                let mut commit_data = Vec::new();

                for result in &mut revwalk {
                    let Ok(oid) = result else { continue };
                    let Ok(commit) = git_repo.find_commit(oid) else { continue };

                    let author = commit.author();
                    let Some(author_email) = author.email() else { continue };

                    if author_email.to_lowercase() != email {
                        continue;
                    }

                    let Some(dt) = Utc.timestamp_opt(author.when().seconds(), 0).single() else {
                        continue;
                    };

                    commit_data.push((oid.to_string(), author_email.to_lowercase(), dt.date_naive()));
                }

                Ok(commit_data)
            })
            .await
            .fang()?
            .fang()?;

            insert_contributions(repo.id, &commit_data, &mut tx).await.fang()?;
        }

        tx.commit().await.fang()?;

        debug!(email = %self.email, "email contribution backfill complete");
        Ok(())
    }

    fn task_type(&self) -> String {
        CONTRIBUTIONS_TASK_TYPE.to_string()
    }

    fn max_retries(&self) -> i32 {
        3
    }
}

use crate::mail::Email;
use crate::user::User;
use crate::{err, mail};

use crate::database::Pool;
use anyhow::{Context, Result};
use git2::{Repository as LibGit2Repo, Signature};
use tracing::instrument;

/// Writes and commits a file into the repository
#[instrument(err, skip(repo, db_pool))]
pub(crate) async fn write_file(repo: &LibGit2Repo, user: &User, branch: Option<&str>, file_name: &str, content: &[u8], db_pool: &Pool) -> Result<()> {
    let mut transaction = db_pool.begin().await?;

    let author_email = Email::find_commit_email(user.id, &mut transaction)
        .await?
        .ok_or_else(|| err!(BAD_REQUEST, "User has no commit email"))?;
    let author_signature = Signature::now(user.username.as_str(), author_email.email.as_str())?;

    // todo what if not configured we should use the user as committer
    let root_email = mail::get_root_email(db_pool).await?;
    let root_signature = Signature::now("GitArena", root_email.as_str())?;

    let blob = repo.blob(content).context("Failed to create blob")?;

    let mut tree_builder = repo.treebuilder(None).context("Failed to acquire tree builder")?;
    tree_builder.insert(file_name, blob, 0o100_644).context("Failed to create blob")?;

    let tree_oid = tree_builder.write().context("Failed to write tree")?;
    let tree = repo.find_tree(tree_oid)?;

    repo.commit(branch, &author_signature, &root_signature, "Initial commit", &tree, &[])
        .context("Failed to commit")?;

    transaction.commit().await?;

    Ok(())
}

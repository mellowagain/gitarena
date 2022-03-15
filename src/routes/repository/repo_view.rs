use crate::git::GIT_HASH_KIND;
use crate::git::history::{all_branches, all_commits, all_tags, last_commit_for_blob, last_commit_for_ref};
use crate::git::utils::{read_blob_content, repo_files_at_ref};
use crate::prelude::{ContextExtensions, LibGit2SignatureExtensions};
use crate::privileges::privilege;
use crate::repository::Repository;
use crate::routes::repository::{GitRequest, GitTreeRequest};
use crate::templates::web::{GitCommit, RepoFile};
use crate::user::{User, WebUser};
use crate::{die, err, render_template};

use std::cmp::Ordering;

use actix_web::{Responder, web};
use anyhow::Result;
use bstr::ByteSlice;
use git_repository::hash::ObjectId;
use git_repository::objs::tree::EntryMode;
use git_repository::objs::Tree;
use git_repository::refs::file::find::existing::Error as GitoxideFindError;
use gitarena_macros::route;
use sqlx::{PgPool, Postgres, Transaction};
use tera::Context;
use tracing_unwrap::OptionExt;

async fn render(tree_option: Option<&str>, repo: Repository, username: &str, web_user: WebUser, mut transaction: Transaction<'_, Postgres>) -> Result<impl Responder> {
    let tree_name = tree_option.unwrap_or_else(|| repo.default_branch.as_str());

    if !privilege::check_access(&repo, web_user.as_ref(), &mut transaction).await? {
        die!(NOT_FOUND, "Not found");
    }

    let mut context = Context::new();

    let libgit2_repo = repo.libgit2(&mut transaction).await?;
    let gitoxide_repo = repo.gitoxide(&mut transaction).await?;

    let (issues_count,): (i64,) = sqlx::query_as("select count(*) from issues where repo = $1 and closed = false and confidential = false")
        .bind(&repo.id)
        .fetch_one(&mut transaction)
        .await?;

    context.try_insert("repo", &repo)?;
    context.try_insert("repo_owner_name", &username)?;
    context.try_insert("issues_count", &issues_count)?;
    context.try_insert("merge_requests_count", &0_i32)?;
    context.try_insert("releases_count", &0_i32)?;
    context.try_insert("tree", tree_name)?;
    context.try_insert("branches", &all_branches(&libgit2_repo).await?)?;
    context.try_insert("tags", &all_tags(&libgit2_repo, None).await?)?;
    context.try_insert("repo_size", &repo.repo_size(&mut transaction).await?)?;
    context.insert_web_user(&web_user)?;

    let loose_ref = match gitoxide_repo.refs.find_loose(tree_name) {
        Ok(loose_ref) => Ok(loose_ref),
        Err(GitoxideFindError::Find(err)) => Err(err),
        Err(GitoxideFindError::NotFound(_)) => {
            if tree_name == repo.default_branch {
                context.try_insert("files", &Vec::<()>::new())?;

                return render_template!("repo/index.html", context, transaction);
            } else {
                die!(NOT_FOUND, "Not found")
            }
        }
    }?; // Handle 404

    let full_tree_name = loose_ref.name.as_bstr().to_str()?;

    context.try_insert("full_tree", full_tree_name)?;

    let mut buffer = Vec::<u8>::new();
    let store = gitoxide_repo.objects.clone();

    let tree = repo_files_at_ref(&loose_ref, store.clone(), &gitoxide_repo, &mut buffer).await?;
    let tree = Tree::from(tree);

    let mut files = Vec::<RepoFile>::new();
    files.reserve(tree.entries.len().min(1000));

    for entry in tree.entries.iter().take(1000) {
        let name = entry.filename.to_str().unwrap_or("Invalid file name");

        let oid = last_commit_for_blob(&libgit2_repo, full_tree_name, name).await?.unwrap_or_log();
        let commit = libgit2_repo.find_commit(oid)?;

        let submodule_target_oid = if matches!(entry.mode, EntryMode::Commit) {
            Some(read_blob_content(entry.oid.as_ref(), store.clone()).await.unwrap_or_else(|_| ObjectId::null(GIT_HASH_KIND).to_string()))
        } else {
            None
        };

        files.push(RepoFile {
            file_type: entry.mode as u16,
            file_name: name,
            submodule_target_oid,
            commit: GitCommit {
                oid: format!("{}", oid),
                message: commit.message().unwrap_or_default().to_owned(),
                time: commit.time().seconds(),
                date: None,
                author_name: String::new(), // Unused for file listing
                author_uid: None, // Unused for file listing
                author_email: String::new() // Unused for file listing
            }
        });
    }

    files.sort_by(|lhs, rhs| {
        // 1. Directory
        // 2. Submodules
        // 3. Rest

        if lhs.file_type == EntryMode::Tree as u16 && rhs.file_type != EntryMode::Tree as u16 {
            Ordering::Less
        } else if lhs.file_type != EntryMode::Tree as u16 && rhs.file_type == EntryMode::Tree as u16 {
            Ordering::Greater
        } else if lhs.file_type == EntryMode::Tree as u16 && rhs.file_type == EntryMode::Tree as u16 {
            lhs.file_name.cmp(rhs.file_name)
        } else if lhs.file_type == EntryMode::Commit as u16 && rhs.file_type != EntryMode::Commit as u16 {
            Ordering::Less
        } else if lhs.file_type != EntryMode::Commit as u16 && rhs.file_type == EntryMode::Commit as u16 {
            Ordering::Greater
        } else {
            lhs.file_name.cmp(rhs.file_name)
        }
    });

    if let Some(fork_repo_id) = repo.forked_from {
        const QUERY: &str = "select users.username, repositories.name from repositories \
         inner join users on users.id = repositories.owner \
         where repositories.id = $1 limit 1";

        let option: Option<(String, String)> = sqlx::query_as(QUERY)
            .bind(fork_repo_id)
            .fetch_optional(&mut transaction)
            .await?;

        if let Some((username, repo_name)) = option {
            context.try_insert("repo_fork_owner", &username)?;
            context.try_insert("repo_fork_name", &repo_name)?;
        }
    }

    context.try_insert("files", &files)?;
    context.try_insert("commits_count", &all_commits(&libgit2_repo, full_tree_name, 0).await?.len())?;

    let last_commit_oid = last_commit_for_ref(&libgit2_repo, full_tree_name).await?.ok_or_else(|| err!(OK, "Repository is empty"))?;
    let last_commit = libgit2_repo.find_commit(last_commit_oid)?;

    // TODO: Additionally show last_commit.committer and if doesn't match with author
    let (author_name, author_uid, author_email) = last_commit.author().try_disassemble(&mut transaction).await;

    context.try_insert("last_commit", &GitCommit {
        oid: format!("{}", last_commit_oid),
        message: last_commit.message().unwrap_or_default().to_owned(),
        time: last_commit.time().seconds(),
        date: None,
        author_name,
        author_uid,
        author_email
    })?;

    render_template!("repo/index.html", context, transaction)
}

#[route("/{username}/{repository}/tree/{tree:.*}", method = "GET", err = "html")]
pub(crate) async fn view_repo_tree(uri: web::Path<GitTreeRequest>, web_user: WebUser, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let mut transaction = db_pool.begin().await?;

    let repo_owner = User::find_using_name(&uri.username, &mut transaction).await.ok_or_else(|| err!(NOT_FOUND, "Repository not found"))?;
    let repo = Repository::open(repo_owner, &uri.repository, &mut transaction).await.ok_or_else(|| err!(NOT_FOUND, "Repository not found"))?;

    render(Some(uri.tree.as_str()), repo, &uri.username, web_user, transaction).await
}

#[route("/{username}/{repository}", method = "GET", err = "html")]
pub(crate) async fn view_repo(uri: web::Path<GitRequest>, web_user: WebUser, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let mut transaction = db_pool.begin().await?;

    let repo_owner = User::find_using_name(&uri.username, &mut transaction).await.ok_or_else(|| err!(NOT_FOUND, "Repository not found"))?;
    let repo = Repository::open(repo_owner, &uri.repository, &mut transaction).await.ok_or_else(|| err!(NOT_FOUND, "Repository not found"))?;

    render(None, repo, &uri.username, web_user, transaction).await
}

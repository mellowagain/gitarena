use crate::error::GAErrors::HttpError;
use crate::extensions::{bstr_to_str, get_user_by_identity, repo_from_str};
use crate::git::history::{all_branches, all_commits, all_tags, last_commit_for_blob, last_commit_for_ref};
use crate::git::utils::{read_blob_content, repo_files_at_ref};
use crate::privileges::privilege;
use crate::render_template;
use crate::repository::Repository;
use crate::routes::repository::{GitRequest, GitTreeRequest};
use crate::templates::web::{GitCommit, RepoFile};

use std::cmp::Ordering;

use actix_identity::Identity;
use actix_web::{Responder, web};
use anyhow::Result;
use bstr::ByteSlice;
use git2::{Buf, Error, ErrorClass};
use git_hash::ObjectId;
use git_object::tree::EntryMode;
use git_object::Tree;
use git_pack::cache::lru::MemoryCappedHashmap;
use git_ref::file::find::existing::Error as GitoxideFindError;
use git_repository::actor::{Signature, SignatureRef};
use gitarena_macros::route;
use sqlx::{PgPool, Postgres, Transaction};
use tera::Context;
use tracing_unwrap::OptionExt;

async fn render(tree_option: Option<&str>, repo: Repository, username: &str, id: Identity, mut transaction: Transaction<'_, Postgres>) -> Result<impl Responder> {
    let tree_name = tree_option.unwrap_or(repo.default_branch.as_str());
    let user = get_user_by_identity(id.identity(), &mut transaction).await;

    if !privilege::check_access(&repo, user.as_ref(), &mut transaction).await? {
        return Err(HttpError(404, "Not found".to_owned()).into());
    }

    let mut context = Context::new();

    let libgit2_repo = repo.libgit2(username).await?;
    let gitoxide_repo = repo.gitoxide(username).await?;

    let loose_ref = match gitoxide_repo.refs.find_loose(tree_name) {
        Ok(loose_ref) => Ok(loose_ref),
        Err(GitoxideFindError::Find(err)) => Err(err),
        Err(GitoxideFindError::NotFound(_)) => return Err(HttpError(404, "Not found".to_owned()).into())
    }?; // Handle 404

    let full_tree_name = bstr_to_str(loose_ref.name.as_bstr())?;

    let mut buffer = Vec::<u8>::new();
    let mut cache = MemoryCappedHashmap::new(10000 * 1024); // 10 MB

    let tree = repo_files_at_ref(&gitoxide_repo, &loose_ref, &mut buffer, &mut cache).await?;
    let tree = Tree::from(tree);

    let mut files = Vec::<RepoFile>::new();
    files.reserve(tree.entries.len().min(1000));

    for entry in tree.entries.iter().take(1000) {
        let name = match entry.filename.to_str() {
            Ok(name) => name,
            Err(_) => "Invalid file name"
        };

        let oid = last_commit_for_blob(&libgit2_repo, full_tree_name, name).await?.unwrap_or_log();
        let commit = libgit2_repo.find_commit(oid)?;

        let submodule_target_oid = if matches!(entry.mode, EntryMode::Commit) {
            Some(read_blob_content(&gitoxide_repo, entry.oid.as_ref(), &mut cache).await.unwrap_or(ObjectId::null_sha1().to_sha1_hex_string()))
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
                author_name: "", // Unused for file listing
                author_uid: None // Unused for file listing
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
            lhs.file_name.cmp(&rhs.file_name)
        } else if lhs.file_type == EntryMode::Commit as u16 && rhs.file_type != EntryMode::Commit as u16 {
            Ordering::Less
        } else if lhs.file_type != EntryMode::Commit as u16 && rhs.file_type == EntryMode::Commit as u16 {
            Ordering::Greater
        } else {
            lhs.file_name.cmp(&rhs.file_name)
        }
    });

    context.try_insert("repo", &repo)?;
    context.try_insert("repo_owner_name", &username)?;
    context.try_insert("repo_size", &repo.repo_size(username).await?)?;
    context.try_insert("files", &files)?;
    context.try_insert("tree", tree_name)?;
    context.try_insert("full_tree", full_tree_name)?;
    context.try_insert("issues_count", &0)?;
    context.try_insert("merge_requests_count", &0)?;
    context.try_insert("releases_count", &0)?;
    context.try_insert("commits_count", &all_commits(&libgit2_repo, full_tree_name).await?.len())?;

    context.try_insert("branches", &all_branches(&libgit2_repo).await?)?;
    context.try_insert("tags", &all_tags(&libgit2_repo, None).await?)?;

    if let Some(user) = user.as_ref() {
        context.try_insert("user", user)?;
    }

    let last_commit_oid = last_commit_for_ref(&libgit2_repo, full_tree_name).await?.ok_or(HttpError(200, "Repository is empty".to_owned()))?;
    let last_commit = libgit2_repo.find_commit(last_commit_oid)?;

    let author_option: Option<(i32, String)> = sqlx::query_as("select id, username from users where lower(email) = lower($1)")
        .bind(last_commit.author().email().unwrap_or("Invalid e-mail address"))
        .fetch_optional(&mut transaction)
        .await?;

    let author_name;
    let author_uid;

    if let Some((user_id, username)) = author_option {
        author_uid = Some(user_id);
        author_name = username;
    } else {
        author_uid = None;
        author_name = last_commit.author().name().unwrap_or("Ghost").to_owned();
    }

    if let Some((signature_buffer, content_buffer)) = libgit2_repo.extract_signature(&last_commit_oid, None).ok() {
        use log::warn;

        let mut valid_signature = true;
        let signature = signature_buffer.as_str().unwrap_or_default();
        let content = content_buffer.as_str().unwrap_or_default();

        for content_line in content.lines() {
            if content_line.starts_with("tree ") {
                let signed_tree = &content_line[5..];
                valid_signature &= signed_tree == last_commit.tree_id().to_string().as_str();
            }

            if content_line.starts_with("parent ") {
                let signed_parent = &content_line[7..];
                let mut found_parent = false;

                for id in last_commit.parent_ids() {
                    if signed_parent == id.to_string().as_str() {
                        found_parent = true;
                        break;
                    }
                }

                valid_signature &= found_parent;
            }

            if content_line.starts_with("author ") {
                let signed_author = &content_line[7..];
                //let signature_ref = SignatureRef::from_bytes(signed_author.as_bytes())?;

                /*

current tree oid: 7a87f16ee302ffb42e13a561bf5abd70beed2826
sig: Some("-----BEGIN PGP SIGNATURE-----\n\niQIzBAABCAAdFiEER3ix/BExA06l2YsN9OUVN2kCW8IFAmE8+fwACgkQ9OUVN2kC\nW8KbHQ//c6sSWBStMw5A7SEnUDgx2C9lZ6BZUc7GM0MdzZZqn+hvPcUcTNRNnYKP\nW9oMslCUia872SpS0QSrP6kMX3PidtMgGKRqYhhEs6Jby3cZw9aQg8GDINt5p1/q\nxdkg7gofKYinzu9X6TJbBmbX7enQEp1Ofcir3LjNrIieSb5EPN7VaxYj7jnBABhk\nhOomTBcm2IeOeg9oviaJ62vfkflXK3Mr0tq2M2i+sNLT058PUgsxapKqze6ziD1U\ngr3f2lQM+dXxjvnVLI2lZQJc5ZPZ4KfjnAuj0Q6Qi3z/C2fBbQxkyumlwIwOo4kt\nrMy00c4jnGbnJad/Y+32ntEDzkn69o9mNJUeoUHk6KhPIia3Mzd/d5+MMxdpOt2t\ne2uT536swR5NqxRNV+OwK0bwW2S4hF5jsExzkpOUvBz57GwAMCawT/1FzaxGd+4G\nu/VJ7puYMrZKXLVuzE8tRXL0qPrRK340pUu05vAR09L7CdGaorK+55jgYDMAjpop\nQvL08W90BB+2y6a4XO+2jfbiFAFISBY5TkqCpLqqGWyRM5jCh3DpByg8RRsCMfG9\nLAIpUOHZE6LBmmPMHUDCsUWKY8zq/2XMFVMbrbEtKR9oEEu8W1Zs+SiqK6GVoAYj\n+9YgrvFrcxRTIzqEqpbfQwHHVJuGVyvCGqoB0CInAGZrhcix9T0=\n=nd7B\n-----END PGP SIGNATURE-----")
content: Some("tree 7a87f16ee302ffb42e13a561bf5abd70beed2826\nparent 5bab4bbd9867ee22c182d39d47cfab3b8bb5ce9e\nauthor Mari <git@cutegirl.tech> 1631385704 +0200\ncommitter Mari <git@cutegirl.tech> 1631386106 +0200\n\nadd image\n")


                 */

            }
        }


        warn!("current tree oid: {}", last_commit.tree_id());

        warn!("sig: {:?} content: {:?}", signature_buffer.as_str(), content_buffer.as_str());
    }

    context.try_insert("last_commit", &GitCommit {
        oid: format!("{}", last_commit_oid),
        message: last_commit.message().unwrap_or_default().to_owned(),
        time: last_commit.time().seconds(),
        author_name: author_name.as_str(),
        author_uid
    })?;

    render_template!("repo/index.html", context, transaction)
}

#[route("/{username}/{repository}/tree/{tree}", method="GET")]
pub(crate) async fn view_repo_tree(uri: web::Path<GitTreeRequest>, id: Identity, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let (repo, transaction) = repo_from_str(&uri.username, &uri.repository, db_pool.begin().await?).await?;

    render(Some(uri.tree.as_str()), repo, &uri.username, id, transaction).await
}

#[route("/{username}/{repository}", method="GET")]
pub(crate) async fn view_repo(uri: web::Path<GitRequest>, id: Identity, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let (repo, transaction) = repo_from_str(&uri.username, &uri.repository, db_pool.begin().await?).await?;

    render(None, repo, &uri.username, id, transaction).await
}

use crate::database::Pool;
use crate::git::history::batch_last_commits;
use crate::git::utils::{read_raw_blob_content, repo_files_at_ref};
use crate::repository::{Branch, Repository};

use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Write};
use std::path::Path;
use std::sync::Arc;

use actix_web::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use async_compression::tokio::write::GzipEncoder;
use async_recursion::async_recursion;
use bstr::ByteSlice;
use chrono::TimeZone;
use gitarena_macros::route;
use gix::objs::Tree;
use gix::objs::tree::EntryKind;
use gix::odb::Store;
use gix::odb::pack::FindExt;
use tokio_tar::{Builder as TarBuilder, Header as TarHeader};
use zip::ZipWriter;
use zip::write::FileOptions as ZipFileOptions;

#[utoipa::path(
    get,
    path = "/api/repo/{namespace}/{repository}/tree/{tree}/download/targz",
    params(
        ("namespace" = String, Path, description = "Repository namespace (user or organization)"),
        ("repository" = String, Path, description = "Repository name"),
        ("tree" = String, Path, description = "Branch or tag name"),
    ),
    responses(
        (status = 200, description = ".tar.gz archive of the repository source content", content_type = "application/gzip"),
    ),
    security((), ("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/{namespace}/{repository}/tree/{tree:.*}/download/targz", method = "GET", err = "text")]
pub(crate) async fn tar_gz_file(repo: Repository, branch: Branch, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let mut tx = db_pool.begin().await?;
    let libgit2_repo = repo.libgit2(&mut tx).await?;

    let gitoxide_repo = branch.gitoxide_repo;
    let mut buffer = Vec::<u8>::new();
    let store = gitoxide_repo.objects.store().clone();

    let tree_ref = repo_files_at_ref(&branch.reference, store.clone(), &gitoxide_repo, &mut buffer).await?;
    let tree = Tree::from(tree_ref);

    let mut paths = HashSet::new();
    collect_paths(store.clone(), tree, "", &mut paths, &mut buffer).await?;

    let commit_oids = batch_last_commits(&libgit2_repo, branch.reference.name.as_bstr().to_str()?, paths).await?;

    let timestamps: HashMap<String, i64> = commit_oids
        .into_iter()
        .filter_map(|(path, oid)| libgit2_repo.find_commit(oid).ok().map(|c| (path, c.time().seconds())))
        .collect();

    let tree_ref = repo_files_at_ref(&branch.reference, store.clone(), &gitoxide_repo, &mut buffer).await?;
    let tree = Tree::from(tree_ref);

    let mut builder = TarBuilder::new(Vec::new());
    write_directory_tar(store.clone(), tree, Path::new("."), &mut builder, &mut buffer, &timestamps).await?;

    let tar_data = builder.into_inner().await?;

    let encoder = GzipEncoder::new(tar_data);
    let gzip_data = encoder.into_inner();

    Ok(HttpResponse::Ok()
        .append_header((CONTENT_TYPE, "application/gzip"))
        .append_header((CONTENT_DISPOSITION, format!("attachment; filename=\"{}-{}.tar.gz\"", &repo.name, &branch.tree)))
        .body(gzip_data))
}

#[async_recursion(?Send)]
async fn collect_paths(store: Arc<Store>, tree: Tree, prefix: &str, paths: &mut HashSet<String>, buffer: &mut Vec<u8>) -> Result<()> {
    for entry in tree.entries {
        let filename = entry.filename.to_str()?;

        let path = if prefix.is_empty() {
            filename.to_owned()
        } else {
            format!("{prefix}/{filename}")
        };

        match entry.mode.kind() {
            EntryKind::Tree => {
                paths.insert(path.clone());

                let (tree_ref, _) = store.to_cache_arc().find_tree(entry.oid.as_ref(), buffer)?;
                collect_paths(store.clone(), Tree::from(tree_ref), &path, paths, buffer).await?;
            }
            EntryKind::Blob | EntryKind::BlobExecutable | EntryKind::Link => {
                paths.insert(path);
            }
            EntryKind::Commit => {}
        }
    }

    Ok(())
}

#[async_recursion(?Send)]
async fn write_directory_tar(
    store: Arc<Store>,
    tree: Tree,
    path: &Path,
    builder: &mut TarBuilder<Vec<u8>>,
    buffer: &mut Vec<u8>,
    timestamps: &HashMap<String, i64>,
) -> Result<()> {
    for entry in tree.entries {
        let filename = entry.filename.to_str()?;
        let path = path.join(filename);
        let kind = entry.mode.kind();

        match kind {
            EntryKind::Tree => {
                let (tree_ref, _) = store.to_cache_arc().find_tree(entry.oid.as_ref(), buffer)?;
                let tree = Tree::from(tree_ref);

                write_directory_tar(store.clone(), tree, path.as_path(), builder, buffer, timestamps).await?;
            }
            EntryKind::Blob | EntryKind::BlobExecutable | EntryKind::Link => {
                let content = read_raw_blob_content(entry.oid.as_ref(), store.clone()).await?;

                let path_str = path.to_string_lossy();
                let path_key = path_str.strip_prefix("./").unwrap_or(&path_str);
                let mtime = timestamps.get(path_key).copied().unwrap_or(0).max(0).cast_unsigned();

                let mut header = TarHeader::new_gnu();
                header.set_size(content.len() as u64);

                header.set_mode(if matches!(kind, EntryKind::BlobExecutable) { 0o775 } else { 0o664 });

                header.set_uid(0);
                header.set_gid(0);
                header.set_username("gitarena")?;
                header.set_groupname("gitarena")?;

                header.set_device_major(0)?;
                header.set_device_minor(0)?;

                header.set_mtime(mtime);

                if matches!(entry.mode.kind(), EntryKind::Link) {
                    let cow = String::from_utf8_lossy(&content[..]);
                    let borrow: &str = cow.borrow();

                    header.set_link_name(Path::new(borrow))?;
                }

                header.set_cksum();

                builder.append_data(&mut header, path.as_path(), &content[..]).await?;
            }
            EntryKind::Commit => {}
        }
    }

    Ok(())
}

#[utoipa::path(
    get,
    path = "/api/repo/{namespace}/{repository}/tree/{tree}/download/zip",
    params(
        ("namespace" = String, Path, description = "Repository namespace (user or organization)"),
        ("repository" = String, Path, description = "Repository name"),
        ("tree" = String, Path, description = "Branch or tag name"),
    ),
    responses(
        (status = 200, description = ".zip archive of the repository source content", content_type = "application/zip"),
    ),
    security((), ("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/{namespace}/{repository}/tree/{tree:.*}/download/zip", method = "GET", err = "text")]
pub(crate) async fn zip_file(repo: Repository, branch: Branch, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let mut transaction = db_pool.begin().await?;
    let libgit2_repo = repo.libgit2(&mut transaction).await?;
    transaction.commit().await?;

    let gitoxide_repo = branch.gitoxide_repo;
    let mut buffer = Vec::<u8>::new();
    let store = gitoxide_repo.objects.store().clone();

    let tree_ref = repo_files_at_ref(&branch.reference, store.clone(), &gitoxide_repo, &mut buffer).await?;
    let tree = Tree::from(tree_ref);

    let mut paths = HashSet::new();
    collect_paths(store.clone(), tree, "", &mut paths, &mut buffer).await?;

    let full_tree_name = branch.reference.name.as_bstr().to_str()?;
    let commit_oids = batch_last_commits(&libgit2_repo, full_tree_name, paths).await?;

    let timestamps: HashMap<String, i64> = commit_oids
        .into_iter()
        .filter_map(|(path, oid)| libgit2_repo.find_commit(oid).ok().map(|c| (path, c.time().seconds())))
        .collect();

    let tree_ref = repo_files_at_ref(&branch.reference, store.clone(), &gitoxide_repo, &mut buffer).await?;
    let tree = Tree::from(tree_ref);

    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    write_directory_zip(store.clone(), tree, Path::new(""), &mut writer, &mut buffer, &timestamps).await?;

    let cursor = writer.finish()?;
    let data = cursor.into_inner();

    Ok(HttpResponse::Ok()
        .append_header((CONTENT_TYPE, "application/zip"))
        .append_header((CONTENT_DISPOSITION, format!("attachment; filename=\"{}-{}.zip\"", &repo.name, &branch.tree)))
        .body(data))
}

#[async_recursion(?Send)]
async fn write_directory_zip(
    store: Arc<Store>,
    tree: Tree,
    path: &Path,
    writer: &mut ZipWriter<Cursor<Vec<u8>>>,
    buffer: &mut Vec<u8>,
    timestamps: &HashMap<String, i64>,
) -> Result<()> {
    for entry in tree.entries {
        let filename = entry.filename.to_str()?;
        let path_buffer = path.join(filename);
        let path = path_buffer.as_path();
        let kind = entry.mode.kind();

        let path_str = path.to_string_lossy();
        let path_key = path_str.strip_prefix("./").unwrap_or(&path_str);
        let mtime = timestamps.get(path_key).copied().unwrap_or(0);

        match kind {
            EntryKind::Tree => {
                let (tree_ref, _) = store.to_cache_arc().find_tree(entry.oid.as_ref(), buffer)?;
                let tree = Tree::from(tree_ref);

                let dir_options = ZipFileOptions::default().last_modified_time(unix_secs_to_zip_datetime(mtime));
                writer.add_directory(format!("{}", path.display()), dir_options)?;

                write_directory_zip(store.clone(), tree, path, writer, buffer, timestamps).await?;
            }
            EntryKind::Blob | EntryKind::BlobExecutable => {
                let content = read_raw_blob_content(entry.oid.as_ref(), store.clone()).await?;

                let options = ZipFileOptions::default()
                    .unix_permissions(if matches!(kind, EntryKind::BlobExecutable) { 0o775 } else { 0o664 })
                    .last_modified_time(unix_secs_to_zip_datetime(mtime))
                    .large_file(content.len() >= 4_294_967_000); // 4 GiB

                writer.start_file(format!("{}", path.display()), options)?;
                writer.write_all(&content[..])?;
            }
            EntryKind::Link => {
                let content = read_raw_blob_content(entry.oid.as_ref(), store.clone()).await?;

                let options = ZipFileOptions::default()
                    .unix_permissions(0o120_777)
                    .last_modified_time(unix_secs_to_zip_datetime(mtime));

                writer.start_file(format!("{}", path.display()), options)?;
                writer.write_all(&content[..])?;
            }
            EntryKind::Commit => {}
        }
    }

    Ok(())
}

fn unix_secs_to_zip_datetime(unix_secs: i64) -> zip::DateTime {
    let dt = chrono::Utc
        .timestamp_opt(unix_secs, 0)
        .single()
        .unwrap_or(chrono::DateTime::<chrono::Utc>::UNIX_EPOCH)
        .naive_utc();

    let dos_dt = dos_date_time::DateTime::try_from(dt).unwrap_or(dos_date_time::DateTime::MIN);

    zip::DateTime::from_msdos(dos_dt.date().to_raw(), dos_dt.time().to_raw())
}

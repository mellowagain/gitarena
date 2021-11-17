use crate::error::GAErrors::HttpError;
use crate::git::GitoxideCacheList;
use crate::git::utils::{read_raw_blob_content, repo_files_at_ref};
use crate::privileges::privilege;
use crate::repository::Repository;
use crate::routes::repository::GitTreeRequest;
use crate::user::{User, WebUser};

use std::borrow::Borrow;
use std::io::{Cursor, Write};
use std::path::Path;

use actix_web::http::header::CONTENT_DISPOSITION;
use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use async_compression::tokio_02::write::GzipEncoder;
use async_recursion::async_recursion;
use bstr::ByteSlice;
use git_repository::objs::tree::EntryMode;
use git_repository::objs::Tree;
use git_repository::odb::pack::cache::DecodeEntry;
use git_repository::odb::pack::FindExt;
use git_repository::refs::file::find::existing::Error as GitoxideFindError;
use git_repository::Repository as GitoxideRepository;
use gitarena_macros::route;
use sqlx::PgPool;
use tokio_tar::{Builder as TarBuilder, Header as TarHeader};
use zip::write::FileOptions as ZipFileOptions;
use zip::ZipWriter;

#[route("/{username}/{repository}/tree/{tree:.*}/archive/targz", method="GET")]
pub(crate) async fn tar_gz_file(uri: web::Path<GitTreeRequest>, web_user: WebUser, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let mut transaction = db_pool.begin().await?;

    let repo_owner = User::find_using_name(&uri.username, &mut transaction).await.ok_or_else(|| HttpError(404, "Repository not found".to_owned()))?;
    let repo = Repository::open(repo_owner, &uri.repository, &mut transaction).await.ok_or_else(|| HttpError(404, "Repository not found".to_owned()))?;

    if !privilege::check_access(&repo, web_user.as_ref(), &mut transaction).await? {
        return Err(HttpError(404, "Not found".to_owned()).into());
    }

    let gitoxide_repo = repo.gitoxide(&mut transaction).await?;

    let loose_ref = match gitoxide_repo.refs.find_loose(&uri.tree) {
        Ok(loose_ref) => Ok(loose_ref),
        Err(GitoxideFindError::Find(err)) => Err(err),
        Err(GitoxideFindError::NotFound(_)) => return Err(HttpError(404, "Not found".to_owned()).into())
    }?; // Handle 404

    let mut buffer = Vec::<u8>::new();

    let mut cache = GitoxideCacheList::default();

    let tree = repo_files_at_ref(&gitoxide_repo, &loose_ref, &mut buffer, &mut cache).await?;
    let tree = Tree::from(tree);

    let mut builder = TarBuilder::new(Vec::new());
    write_directory_tar(&gitoxide_repo, tree, Path::new("."), &mut builder, &mut buffer, &mut cache).await?;

    let tar_data = builder.into_inner().await?;

    let encoder = GzipEncoder::new(tar_data);
    let gzip_data = encoder.into_inner();

    Ok(HttpResponse::Ok()
        .header(CONTENT_DISPOSITION, format!("attachment; filename=\"{}.tar.gz\"", &repo.name))
        .body(gzip_data))
}

#[async_recursion(?Send)]
async fn write_directory_tar(repo: &GitoxideRepository, tree: Tree, path: &Path, builder: &mut TarBuilder<Vec<u8>>, buffer: &mut Vec<u8>, cache: &mut impl DecodeEntry) -> Result<()> {
    for entry in tree.entries {
        let filename = entry.filename.to_str()?;
        let path = path.join(filename);

        match entry.mode {
            EntryMode::Tree => {
                let tree = repo.odb.find_tree(&entry.oid, buffer, cache)?;
                let tree = Tree::from(tree);

                write_directory_tar(repo, tree, path.as_path(), builder, buffer, cache).await?;
            }
            EntryMode::Blob | EntryMode::BlobExecutable | EntryMode::Link => {
                let content = read_raw_blob_content(repo, entry.oid.as_ref(), cache).await?;

                let mut header = TarHeader::new_gnu();
                header.set_size(content.len() as u64);

                header.set_mode(if matches!(entry.mode, EntryMode::BlobExecutable) {
                    0o775
                } else {
                    0o664
                });

                header.set_uid(0);
                header.set_gid(0);
                header.set_username("gitarena")?;
                header.set_groupname("gitarena")?;

                header.set_device_major(0)?;
                header.set_device_minor(0)?;

                header.set_mtime(0); // TODO: Unix timestamp of last commit to this file

                if matches!(entry.mode, EntryMode::Link) {
                    let cow = String::from_utf8_lossy(&content[..]);
                    let borrow: &str = cow.borrow();

                    header.set_link_name(Path::new(borrow))?;
                }

                header.set_cksum();

                builder.append_data(&mut header, path.as_path(), &content[..]).await?;
            }
            EntryMode::Commit => { /* TODO: implement submodules */ }
        }
    }

    Ok(())
}

#[route("/{username}/{repository}/tree/{tree:.*}/archive/zip", method="GET")]
pub(crate) async fn zip_file(uri: web::Path<GitTreeRequest>, web_user: WebUser, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let mut transaction = db_pool.begin().await?;

    let repo_owner = User::find_using_name(&uri.username, &mut transaction).await.ok_or_else(|| HttpError(404, "Repository not found".to_owned()))?;
    let repo = Repository::open(repo_owner, &uri.repository, &mut transaction).await.ok_or_else(|| HttpError(404, "Repository not found".to_owned()))?;

    if !privilege::check_access(&repo, web_user.as_ref(), &mut transaction).await? {
        return Err(HttpError(404, "Not found".to_owned()).into());
    }

    let gitoxide_repo = repo.gitoxide(&mut transaction).await?;

    let loose_ref = match gitoxide_repo.refs.find_loose(&uri.tree) {
        Ok(loose_ref) => Ok(loose_ref),
        Err(GitoxideFindError::Find(err)) => Err(err),
        Err(GitoxideFindError::NotFound(_)) => return Err(HttpError(404, "Not found".to_owned()).into())
    }?; // Handle 404

    let mut buffer = Vec::<u8>::new();
    let mut cache = GitoxideCacheList::default();

    let tree = repo_files_at_ref(&gitoxide_repo, &loose_ref, &mut buffer, &mut cache).await?;
    let tree = Tree::from(tree);

    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    write_directory_zip(&gitoxide_repo, tree, Path::new(""), &mut writer, &mut buffer, &mut cache).await?;

    let cursor = writer.finish()?;
    let data = cursor.into_inner();

    Ok(HttpResponse::Ok()
        .header(CONTENT_DISPOSITION, format!("attachment; filename=\"{}.zip\"", &repo.name))
        .body(data))
}

#[async_recursion(?Send)]
async fn write_directory_zip(repo: &GitoxideRepository, tree: Tree, path: &Path, writer: &mut ZipWriter<Cursor<Vec<u8>>>, buffer: &mut Vec<u8>, cache: &mut impl DecodeEntry) -> Result<()> {
    for entry in tree.entries {
        let filename = entry.filename.to_str()?;
        let path_buffer = path.join(filename);
        let path = path_buffer.as_path();

        match entry.mode {
            EntryMode::Tree => {
                let tree = repo.odb.find_tree(&entry.oid, buffer, cache)?;
                let tree = Tree::from(tree);

                writer.add_directory(format!("{}", path.display()), ZipFileOptions::default())?;

                write_directory_zip(repo, tree, path, writer, buffer, cache).await?;
            }
            EntryMode::Blob | EntryMode::BlobExecutable => {
                let content = read_raw_blob_content(repo, entry.oid.as_ref(), cache).await?;

                let options = ZipFileOptions::default()
                    .unix_permissions(if matches!(entry.mode, EntryMode::BlobExecutable) {
                        0o775
                    } else {
                        0o664
                    })
                    .large_file(content.len() >= 4294967000); // 4 GiB
                    //.last_modified_time(...) TODO: DateTime of last commit to this file

                writer.start_file(format!("{}", path.display()), options)?;
                writer.write_all(&content[..])?;
            }
            EntryMode::Link | EntryMode::Commit => { /* TODO: implement symlinks and submodules */ }
        }
    }

    Ok(())
}

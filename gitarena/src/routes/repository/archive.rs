use crate::git::utils::{read_raw_blob_content, repo_files_at_ref};
use crate::repository::{Branch, Repository};

use std::borrow::Borrow;
use std::io::{Cursor, Write};
use std::path::Path;
use std::sync::Arc;

use actix_web::http::header::CONTENT_DISPOSITION;
use actix_web::{HttpResponse, Responder};
use anyhow::Result;
use async_compression::tokio::write::GzipEncoder;
use async_recursion::async_recursion;
use bstr::ByteSlice;
use gitarena_macros::route;
use gix::objs::Tree;
use gix::objs::tree::EntryKind;
use gix::odb::Store;
use gix::odb::pack::FindExt;
use tokio_tar::{Builder as TarBuilder, Header as TarHeader};
use zip::ZipWriter;
use zip::write::FileOptions as ZipFileOptions;

#[route("/{username}/{repository}/tree/{tree:.*}/archive/targz", method = "GET", err = "html")]
pub(crate) async fn tar_gz_file(repo: Repository, branch: Branch) -> Result<impl Responder> {
    let gitoxide_repo = branch.gitoxide_repo;

    let mut buffer = Vec::<u8>::new();

    let store = gitoxide_repo.objects.store().clone();

    let tree = repo_files_at_ref(&branch.reference, store.clone(), &gitoxide_repo, &mut buffer).await?;
    let tree = Tree::from(tree);

    let mut builder = TarBuilder::new(Vec::new());
    write_directory_tar(store.clone(), tree, Path::new("."), &mut builder, &mut buffer).await?;

    let tar_data = builder.into_inner().await?;

    let encoder = GzipEncoder::new(tar_data);
    let gzip_data = encoder.into_inner();

    Ok(HttpResponse::Ok()
        .append_header((CONTENT_DISPOSITION, format!("attachment; filename=\"{}.tar.gz\"", &repo.name)))
        .body(gzip_data))
}

#[async_recursion(?Send)]
async fn write_directory_tar(store: Arc<Store>, tree: Tree, path: &Path, builder: &mut TarBuilder<Vec<u8>>, buffer: &mut Vec<u8>) -> Result<()> {
    for entry in tree.entries {
        let filename = entry.filename.to_str()?;
        let path = path.join(filename);
        let kind = entry.mode.kind();

        match kind {
            EntryKind::Tree => {
                let (tree_ref, _) = store.to_cache_arc().find_tree(entry.oid.as_ref(), buffer)?;
                let tree = Tree::from(tree_ref);

                write_directory_tar(store.clone(), tree, path.as_path(), builder, buffer).await?;
            }
            EntryKind::Blob | EntryKind::BlobExecutable | EntryKind::Link => {
                let content = read_raw_blob_content(entry.oid.as_ref(), store.clone()).await?;

                let mut header = TarHeader::new_gnu();
                header.set_size(content.len() as u64);

                header.set_mode(if matches!(kind, EntryKind::BlobExecutable) { 0o775 } else { 0o664 });

                header.set_uid(0);
                header.set_gid(0);
                header.set_username("gitarena")?;
                header.set_groupname("gitarena")?;

                header.set_device_major(0)?;
                header.set_device_minor(0)?;

                header.set_mtime(0); // TODO: Unix timestamp of last commit to this file

                if matches!(entry.mode.kind(), EntryKind::Link) {
                    let cow = String::from_utf8_lossy(&content[..]);
                    let borrow: &str = cow.borrow();

                    header.set_link_name(Path::new(borrow))?;
                }

                header.set_cksum();

                builder.append_data(&mut header, path.as_path(), &content[..]).await?;
            }
            EntryKind::Commit => { /* TODO: implement submodules */ }
        }
    }

    Ok(())
}

#[route("/{username}/{repository}/tree/{tree:.*}/archive/zip", method = "GET", err = "html")]
pub(crate) async fn zip_file(repo: Repository, branch: Branch) -> Result<impl Responder> {
    let gitoxide_repo = branch.gitoxide_repo;

    let mut buffer = Vec::<u8>::new();
    let store = gitoxide_repo.objects.store().clone();

    let tree = repo_files_at_ref(&branch.reference, store.clone(), &gitoxide_repo, &mut buffer).await?;
    let tree = Tree::from(tree);

    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    write_directory_zip(store.clone(), tree, Path::new(""), &mut writer, &mut buffer).await?;

    let cursor = writer.finish()?;
    let data = cursor.into_inner();

    Ok(HttpResponse::Ok()
        .append_header((CONTENT_DISPOSITION, format!("attachment; filename=\"{}.zip\"", &repo.name)))
        .body(data))
}

#[async_recursion(?Send)]
async fn write_directory_zip(store: Arc<Store>, tree: Tree, path: &Path, writer: &mut ZipWriter<Cursor<Vec<u8>>>, buffer: &mut Vec<u8>) -> Result<()> {
    for entry in tree.entries {
        let filename = entry.filename.to_str()?;
        let path_buffer = path.join(filename);
        let path = path_buffer.as_path();
        let kind = entry.mode.kind();

        match kind {
            EntryKind::Tree => {
                let (tree_ref, _) = store.to_cache_arc().find_tree(entry.oid.as_ref(), buffer)?;
                let tree = Tree::from(tree_ref);

                writer.add_directory(format!("{}", path.display()), ZipFileOptions::default())?;

                write_directory_zip(store.clone(), tree, path, writer, buffer).await?;
            }
            EntryKind::Blob | EntryKind::BlobExecutable => {
                let content = read_raw_blob_content(entry.oid.as_ref(), store.clone()).await?;

                let options = ZipFileOptions::default()
                    .unix_permissions(if matches!(kind, EntryKind::BlobExecutable) { 0o775 } else { 0o664 })
                    .large_file(content.len() >= 4294967000); // 4 GiB

                //.last_modified_time(...) TODO: DateTime of last commit to this file

                writer.start_file(format!("{}", path.display()), options)?;
                writer.write_all(&content[..])?;
            }
            EntryKind::Link | EntryKind::Commit => { /* TODO: implement symlinks and submodules */ }
        }
    }

    Ok(())
}

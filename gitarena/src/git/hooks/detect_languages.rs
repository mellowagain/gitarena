use crate::git::utils::{read_raw_blob_content, repo_files_at_head};
use crate::repository::Repository;

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use async_recursion::async_recursion;
use bstr::ByteSlice;
use gix::objs::Tree;
use gix::objs::tree::EntryKind;
use gix::odb::Store;
use gix::odb::pack::FindExt;
use sqlx::types::Json;
use tokei::LanguageType;
use tracing::{debug, instrument};

// overrides for extensionless languages
fn language_from_filename(filename: &str) -> Option<LanguageType> {
    match filename {
        "Makefile" | "makefile" | "GNUmakefile" => Some(LanguageType::Makefile),
        "CMakeLists.txt" => Some(LanguageType::CMake),
        "Dockerfile" => Some(LanguageType::Dockerfile),
        "Gemfile" | "Rakefile" | "Vagrantfile" | "Podfile" => Some(LanguageType::Ruby),
        "Justfile" | "justfile" => Some(LanguageType::Just),
        "SConstruct" | "SConscript" => Some(LanguageType::Scons),
        "BUILD" | "WORKSPACE" => Some(LanguageType::Bazel),
        "meson.build" => Some(LanguageType::Meson),
        _ => None,
    }
}

fn is_lockfile(filename: &str) -> bool {
    matches!(
        filename,
        "package-lock.json"
            | "yarn.lock"
            | "pnpm-lock.yaml"
            | "Cargo.lock"
            | "Gemfile.lock"
            | "Podfile.lock"
            | "composer.lock"
            | "poetry.lock"
            | "mix.lock"
            | "packages.lock.json"
            | "pubspec.lock"
    )
}

fn is_asset(lang: LanguageType) -> bool {
    matches!(lang, LanguageType::Svg)
}

fn detect_language(filename: &str) -> Option<LanguageType> {
    if let Some(ext) = Path::new(filename).extension().and_then(|ext| ext.to_str())
        && let Some(lang) = LanguageType::from_file_extension(ext)
    {
        return Some(lang);
    }

    language_from_filename(filename)
}

#[instrument(err, skip(store))]
pub(crate) async fn detect_languages(store: Arc<Store>, gitoxide_repo: &gix::Repository, repo: &mut Repository) -> Result<()> {
    let mut buffer = Vec::<u8>::new();

    let tree_ref = repo_files_at_head(store.clone(), gitoxide_repo, &mut buffer).await?;
    let tree = Tree::from(tree_ref);

    let mut language_bytes: HashMap<String, u64> = HashMap::new();

    walk_tree(tree, store, &mut language_bytes, String::new()).await?;

    debug!(?repo, languages = ?language_bytes, "detected languages for repo");
    repo.languages = Json(language_bytes);

    Ok(())
}

#[async_recursion(?Send)]
async fn walk_tree(tree: Tree, store: Arc<Store>, language_bytes: &mut HashMap<String, u64>, path: String) -> Result<()> {
    for entry in tree.entries {
        let filename = entry.filename.to_str_lossy();
        let full_path = if path.is_empty() {
            filename.to_string()
        } else {
            format!("{path}/{filename}")
        };

        match entry.mode.kind() {
            EntryKind::Blob | EntryKind::BlobExecutable => {
                if is_lockfile(filename.as_ref()) || linguist::is_vendored(&full_path).unwrap_or(false) {
                    continue;
                }

                if let Some(lang) = detect_language(filename.as_ref()) {
                    if is_asset(lang) {
                        *language_bytes.entry(lang.name().to_owned()).or_default() += 1;
                    } else {
                        let blob = read_raw_blob_content(entry.oid.as_ref(), store.clone()).await?;
                        *language_bytes.entry(lang.name().to_owned()).or_default() += blob.len() as u64;
                    }
                }
            }
            EntryKind::Tree => {
                if linguist::is_vendored(format!("{full_path}/")).unwrap_or(false) {
                    continue;
                }

                let mut buffer = Vec::<u8>::new();
                let (tree_ref, _) = store.to_handle_arc().find_tree(entry.oid.as_ref(), &mut buffer)?;
                let sub_tree = Tree::from(tree_ref);

                walk_tree(sub_tree, store.clone(), language_bytes, full_path).await?;
            }
            EntryKind::Link | EntryKind::Commit => {}
        }
    }

    Ok(())
}

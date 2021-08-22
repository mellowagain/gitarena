use crate::git::hooks::detect_license::detect_license;
use crate::repository::Repository;

use anyhow::Result;
use git_odb::pack::cache::lru::MemoryCappedHashmap;

pub(crate) async fn run(repo: &mut Repository, owner_username: &str, cache: MemoryCappedHashmap) -> Result<MemoryCappedHashmap> {
    let gitoxide_repo = repo.gitoxide(owner_username).await?;
    let mut mut_cache = cache;

    // All hooks that require HEAD to point to something
    match gitoxide_repo.refs.loose_find("HEAD")? {
        Some(reference) => {
            mut_cache = detect_license(repo, &gitoxide_repo, reference, mut_cache).await?;
        }
        None => {}
    }

    Ok(mut_cache)
}

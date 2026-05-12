use crate::prelude::LibGit2SignatureExtensions;
use crate::repository::Repository;
use crate::{die, err};

use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use git2::{Delta, DiffFindOptions, DiffFormat, DiffOptions};
use gitarena_common::database::Pool;
use gitarena_macros::route;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

const MAX_LINES_PER_FILE: usize = 5_000;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SignatureInfo {
    /// Display name
    name: String,
    /// Email address
    email: String,
    /// Unix timestamp
    timestamp: i64,
    /// GitArena user ID, if the email matches a registered user
    uid: Option<Uuid>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommitMeta {
    /// Full SHA-1 hex
    oid: String,
    /// Abbreviated SHA-1 hex (7 chars)
    short_oid: String,
    /// First line of the commit message
    message: String,
    /// Remainder of the commit message after the first blank line, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    /// Author (who wrote the code)
    author: SignatureInfo,
    /// Committer (who committed the code)
    committer: SignatureInfo,
    /// Parent commit OIDs (full SHA-1 hex)
    parents: Vec<String>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DiffStats {
    /// Number of files changed
    files_changed: usize,
    /// Total lines added
    insertions: usize,
    /// Total lines deleted
    deletions: usize,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileStats {
    /// Lines added in this file
    insertions: usize,
    /// Lines deleted in this file
    deletions: usize,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum DiffLineKind {
    Context,
    Addition,
    Deletion,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DiffLineEntry {
    /// Whether this line is context, an addition, or a deletion
    kind: DiffLineKind,
    /// Line number in the old file (null for additions)
    #[serde(skip_serializing_if = "Option::is_none")]
    old_line_number: Option<u32>,
    /// Line number in the new file (null for deletions)
    #[serde(skip_serializing_if = "Option::is_none")]
    new_line_number: Option<u32>,
    /// Line content without the leading +/-/space character
    content: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DiffHunk {
    /// Hunk header line (e.g. "@@ -12,7 +12,9 @@ impl Cloner {")
    header: String,
    /// Starting line number in the old file
    old_start: u32,
    /// Number of lines in the old file
    old_lines: u32,
    /// Starting line number in the new file
    new_start: u32,
    /// Number of lines in the new file
    new_lines: u32,
    /// Lines in this hunk
    lines: Vec<DiffLineEntry>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum DiffStatus {
    Added,
    Deleted,
    Modified,
    Renamed,
    Copied,
    Untracked,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DiffFile {
    /// Change status of the file
    status: DiffStatus,
    /// New path of the file (or only path for non-renames)
    path: String,
    /// Old path, only present for renames and copies
    #[serde(skip_serializing_if = "Option::is_none")]
    old_path: Option<String>,
    /// Whether the file is binary (hunks will be empty)
    binary: bool,
    /// Whether the file was truncated due to exceeding the line cap (hunks will be empty)
    too_large: bool,
    /// Per-file insertion/deletion counts
    stats: FileStats,
    /// Diff hunks (empty for binary or too-large files)
    hunks: Vec<DiffHunk>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommitDetailResponse {
    commit: CommitMeta,
    /// Short branch name that contains this commit, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    branch: Option<String>,
    stats: DiffStats,
    files: Vec<DiffFile>,
}

#[utoipa::path(
    get,
    path = "/api/repos/{namespace}/{repository}/commits/{oid}",
    params(
        ("namespace" = String, Path, description = "Repository namespace (user or organization)"),
        ("repository" = String, Path, description = "Repository name"),
        ("oid" = String, Path, description = "Full or abbreviated commit SHA-1"),
    ),
    responses(
        (status = 200, description = "Commit detail with structured diff", body = CommitDetailResponse),
        (status = 400, description = "Invalid OID"),
        (status = 404, description = "Repository or commit not found"),
    ),
    security((), ("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repos/{namespace}/{repository}/commits/{oid}", method = "GET", err = "json")]
pub(crate) async fn commit_detail(repo: Repository, path: web::Path<(String, String, String)>, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let mut transaction = db_pool.begin().await?;

    let (_, _, oid_str) = path.into_inner();
    let libgit2_repo = repo.libgit2(&mut transaction).await?;

    let object = libgit2_repo.revparse_single(&oid_str).map_err(|_| err!(NOT_FOUND, "Commit not found"))?;

    let Ok(commit) = object.into_commit() else {
        die!(BAD_REQUEST, "OID does not refer to a commit");
    };

    let (author_name, author_uid, author_email) = commit.author().try_disassemble(&mut transaction).await;
    let (committer_name, committer_uid, committer_email) = commit.committer().try_disassemble(&mut transaction).await;

    let raw_message = commit.message().unwrap_or_default();
    let (message, description) = split_message(raw_message);

    let commit_meta = CommitMeta {
        oid: format!("{}", commit.id()),
        short_oid: format!("{:.7}", commit.id()),
        message,
        description,
        author: SignatureInfo {
            name: author_name,
            email: author_email,
            timestamp: commit.author().when().seconds(),
            uid: author_uid,
        },
        committer: SignatureInfo {
            name: committer_name,
            email: committer_email,
            timestamp: commit.committer().when().seconds(),
            uid: committer_uid,
        },
        parents: commit.parent_ids().map(|id| format!("{id}")).collect(),
    };

    let commit_tree = commit.tree()?;

    let parent_tree = if commit.parent_count() > 0 {
        let parent = commit.parent(0)?;
        Some(parent.tree()?)
    } else {
        None
    };

    let mut diff_opts = DiffOptions::new();
    diff_opts.context_lines(3);

    let mut diff = libgit2_repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&commit_tree), Some(&mut diff_opts))?;

    let mut find_opts = DiffFindOptions::new();
    diff.find_similar(Some(&mut find_opts))?; // rename / copy detection

    let num_deltas = diff.deltas().count();
    let mut files: Vec<DiffFile> = Vec::with_capacity(num_deltas);

    for delta in diff.deltas() {
        let binary = delta.flags().is_binary();

        let status = match delta.status() {
            Delta::Added => DiffStatus::Added,
            Delta::Deleted => DiffStatus::Deleted,
            Delta::Modified => DiffStatus::Modified,
            Delta::Renamed => DiffStatus::Renamed,
            Delta::Copied => DiffStatus::Copied,
            _ => DiffStatus::Untracked,
        };

        let is_rename_or_copy = matches!(status, DiffStatus::Renamed | DiffStatus::Copied);

        let path = delta.new_file().path().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
        let old_path = if is_rename_or_copy {
            delta.old_file().path().map(|p| p.to_string_lossy().into_owned())
        } else {
            None
        };

        files.push(DiffFile {
            status,
            path,
            old_path,
            binary,
            too_large: false,
            stats: FileStats { insertions: 0, deletions: 0 },
            hunks: Vec::new(),
        });
    }

    {
        let files_ref = &mut files;
        let mut current_file_idx: usize = 0;
        let mut current_line_count: usize = 0;
        let mut too_large_set = vec![false; num_deltas];
        let mut first_callback = true;
        let mut last_hunk_header: Option<String> = None;

        diff.print(DiffFormat::Patch, |_delta, hunk_opt, line| {
            // 'F' marks transition to the next file
            if line.origin() == 'F' {
                if first_callback {
                    first_callback = false;
                } else {
                    current_file_idx += 1;
                }
                current_line_count = 0;
                last_hunk_header = None;
                return true;
            }

            if current_file_idx >= files_ref.len() {
                return true;
            }

            let file = &mut files_ref[current_file_idx];

            if file.binary {
                return true;
            }

            if too_large_set[current_file_idx] {
                return true;
            }

            if let Some(hunk) = hunk_opt {
                let header = std::str::from_utf8(hunk.header()).unwrap_or("").trim_end_matches('\n').to_owned();
                if last_hunk_header.as_deref() != Some(&header) {
                    last_hunk_header = Some(header.clone());
                    file.hunks.push(DiffHunk {
                        header,
                        old_start: hunk.old_start(),
                        old_lines: hunk.old_lines(),
                        new_start: hunk.new_start(),
                        new_lines: hunk.new_lines(),
                        lines: Vec::new(),
                    });
                }
            }

            let kind = match line.origin() {
                '+' => Some(DiffLineKind::Addition),
                '-' => Some(DiffLineKind::Deletion),
                ' ' => Some(DiffLineKind::Context),
                _ => None,
            };

            if let Some(kind) = kind {
                current_line_count += 1;

                if current_line_count > MAX_LINES_PER_FILE {
                    too_large_set[current_file_idx] = true;
                    file.hunks.clear();
                    return true;
                }

                match kind {
                    DiffLineKind::Addition => file.stats.insertions += 1,
                    DiffLineKind::Deletion => file.stats.deletions += 1,
                    DiffLineKind::Context => {}
                }

                let content = String::from_utf8_lossy(line.content()).trim_end_matches('\n').to_string();

                if let Some(last_hunk) = file.hunks.last_mut() {
                    last_hunk.lines.push(DiffLineEntry {
                        kind,
                        old_line_number: line.old_lineno(),
                        new_line_number: line.new_lineno(),
                        content,
                    });
                }
            }

            true
        })?;

        for (idx, tl) in too_large_set.into_iter().enumerate() {
            files[idx].too_large = tl;
        }
    }

    let total_insertions: usize = files.iter().map(|f| f.stats.insertions).sum();
    let total_deletions: usize = files.iter().map(|f| f.stats.deletions).sum();

    let stats = DiffStats {
        files_changed: files.len(),
        insertions: total_insertions,
        deletions: total_deletions,
    };

    let commit_oid = commit.id();
    let default_ref = format!("refs/heads/{}", repo.default_branch);
    let branch = libgit2_repo
        .references_glob("refs/heads/*")?
        .filter_map(std::result::Result::ok)
        .filter(|r| {
            r.peel_to_commit()
                .ok()
                .and_then(|tip| {
                    if tip.id() == commit_oid {
                        Some(true)
                    } else {
                        libgit2_repo.graph_descendant_of(tip.id(), commit_oid).ok()
                    }
                })
                .unwrap_or(false)
        })
        .min_by_key(|r| i32::from(r.name().unwrap_or("") != default_ref))
        .and_then(|r| r.shorthand().map(ToOwned::to_owned));

    transaction.commit().await?;

    Ok(HttpResponse::Ok().json(CommitDetailResponse {
        commit: commit_meta,
        branch,
        stats,
        files,
    }))
}

/// Splits a raw git commit message at the first blank line.
/// Returns (first_line, optional_description)
fn split_message(raw: &str) -> (String, Option<String>) {
    let mut lines = raw.splitn(2, "\n\n");
    let first = lines.next().unwrap_or("").trim_end_matches('\n').to_owned();
    let rest = lines.next().map(|s| s.trim().to_owned()).filter(|s| !s.is_empty());
    (first, rest)
}

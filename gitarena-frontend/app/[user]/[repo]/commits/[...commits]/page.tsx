"use client";

import Link from "next/link";
import { useParams } from "next/navigation";
import { useState } from "react";
import useSWR from "swr";
import useSWRInfinite from "swr/infinite";
import { formatDistanceToNow, format } from "date-fns";
import { ArrowLeft, ArrowDown, ArrowUp, FolderOpen, GitCommit, ChevronDown, ChevronUp, Code, AlertCircle, GitMerge } from "lucide-react";
import { TopBar } from "@/components/top-bar";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { ErrorDisplay } from "@/components/error-display";
import { HashCopy } from "@/components/hash-copy";
import type { FileCommitInfo } from "@/components/repo-file-sidebar";
import type { BranchesResponse } from "@/components/branch-bar";
import type { RepoMetadata } from "@/app/[user]/[repo]/page";
import { UserAvatar } from "@/components/ui/user-avatar";

interface BranchCommitsResponse {
    commits: FileCommitInfo[];
}

const PAGE_SIZE = 20;

function CommitRow({ commit, user, repo }: { commit: FileCommitInfo; user: string; repo: string }) {
    const [expanded, setExpanded] = useState(false);

    const lines = commit.message.split("\n");
    const subject = lines[0];
    const body = lines.slice(1).join("\n").trim();
    const hasBody = body.length > 0;

    const shortHash = commit.sha1.slice(0, 7);
    const relativeDate = formatDistanceToNow(new Date(commit.time * 1000), { addSuffix: true, includeSeconds: true });

    return (
        <div className="group/commit pl-5 border-l-4 border-border transition-all hover:border-l-[5px]">
            <div className="flex items-start gap-3">
                <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 flex-wrap mb-0.5">
                        <Link href={`/${user}/${repo}/commit/${commit.sha1}`} className="text-sm font-medium hover:underline leading-snug">
                            {subject}
                        </Link>
                        {hasBody && (
                            <button
                                onClick={() => setExpanded((v) => !v)}
                                title={expanded ? "Collapse" : "Expand commit body"}
                                className="flex items-center gap-0.5 px-1.5 py-0.5 text-[10px] text-muted-foreground border border-border rounded hover:bg-accent/50 hover:text-foreground transition-colors shrink-0"
                            >
                                {expanded ? <ChevronUp className="h-3 w-3" /> : <ChevronDown className="h-3 w-3" />}
                            </button>
                        )}
                    </div>

                    {expanded && (
                        <pre className="mt-1.5 mb-2 text-xs text-muted-foreground whitespace-pre-wrap font-sans leading-relaxed">
                            {body}
                        </pre>
                    )}

                    <div className="flex items-center gap-2 text-xs text-muted-foreground mt-1.5">
                        <UserAvatar userId={commit.authorUid} username={commit.authorName} size="xs" />
                        <span className="font-medium text-foreground/80">{commit.authorName}</span>
                        <span className="text-muted-foreground/40">·</span>
                        <span>{relativeDate}</span>
                    </div>
                </div>

                <div className="flex items-center gap-2 shrink-0 mt-0.5">
                    <Link
                        href={`/${user}/${repo}/tree/${commit.sha1}`}
                        title="Browse repository at this commit"
                        className="flex items-center gap-1.5 px-2 py-1 text-xs text-muted-foreground border border-border rounded-md hover:bg-accent/50 hover:text-foreground transition-colors whitespace-nowrap"
                    >
                        <FolderOpen className="h-3.5 w-3.5" />
                        Browse
                    </Link>
                    <HashCopy shortHash={shortHash} fullHash={commit.sha1} />
                </div>
            </div>
        </div>
    );
}

function CommitsSkeleton() {
    return (
        <div className="space-y-8">
            {[1, 2].map((group) => (
                <div key={group}>
                    <Skeleton className="h-3 w-48 mb-4" />
                    <div className="space-y-3">
                        {Array.from({ length: group === 1 ? 4 : 3 }).map((_, i) => (
                            <div key={i} className="pl-5 border-l-4 border-border">
                                <div className="flex items-start gap-3">
                                    <div className="flex-1 min-w-0 space-y-2">
                                        <Skeleton className="h-4 w-3/4" />
                                        <div className="flex items-center gap-2">
                                            <Skeleton className="h-4 w-4 rounded-full" />
                                            <Skeleton className="h-3 w-20" />
                                            <Skeleton className="h-3 w-24" />
                                        </div>
                                    </div>
                                    <div className="flex items-center gap-2 shrink-0">
                                        <Skeleton className="h-6 w-16 rounded-md" />
                                        <Skeleton className="h-6 w-14 rounded" />
                                    </div>
                                </div>
                            </div>
                        ))}
                    </div>
                </div>
            ))}
        </div>
    );
}

function groupByDate(commits: FileCommitInfo[]): { label: string; commits: FileCommitInfo[] }[] {
    const groups: Record<string, FileCommitInfo[]> = {};
    for (const commit of commits) {
        const label = format(new Date(commit.time * 1000), "EEEE, MMMM d, yyyy");
        if (!groups[label]) {
            groups[label] = [];
        }
        groups[label].push(commit);
    }
    return Object.entries(groups).map(([label, commits]) => ({ label, commits }));
}

export default function CommitsPage() {
    const { user, repo, commits: undecodedSegments } = useParams<{ user: string; repo: string; commits: string[] }>();
    const branch = decodeURIComponent(undecodedSegments[0]);
    const undecodedBranch = undecodedSegments[0];
    const filePath = undecodedSegments.length > 1 ? undecodedSegments.slice(1).map(decodeURIComponent).join("/") : null;

    const {
        data: branchesData,
        isLoading: branchesLoading,
        error: branchesError,
    } = useSWR<BranchesResponse>(`/api/repos/${user}/${repo}/branches`);
    const { data: repoData, error: repoError } = useSWR<RepoMetadata>(`/api/repos/${user}/${repo}`);

    const { data, error, isLoading, isValidating, size, setSize } = useSWRInfinite<BranchCommitsResponse>((index) => {
        const base = `/api/repos/${user}/${repo}/branch/${undecodedBranch}/commits?limit=${PAGE_SIZE}&offset=${index * PAGE_SIZE}`;
        return filePath ? `${base}&path=${encodeURIComponent(filePath)}` : base;
    });

    if (branchesError) {
        return <ErrorDisplay failed="branches" error={branchesError} />;
    }

    if (repoError) {
        return <ErrorDisplay failed="repository" error={repoError} />;
    }

    if (error) {
        return <ErrorDisplay failed="commits" error={error} />;
    }

    const branchInfo = branchesData?.branches.find((b) => b.name === branch);
    const totalCommitCount = branchInfo?.commitCount;
    const isDefaultBranch = repoData != null && repoData.defaultBranch === branch;

    const allCommits = data?.flatMap((page) => page.commits) ?? [];
    const lastPage = data?.[data.length - 1];
    const hasMore = !isLoading && lastPage != null && lastPage.commits.length === PAGE_SIZE;

    const groups = groupByDate(allCommits);

    return (
        <div className="flex flex-col h-screen overflow-hidden bg-background text-foreground font-sans">
            <TopBar
                breadcrumb={[{ label: user, href: `/${user}` }, { label: repo, href: `/${user}/${repo}` }, { label: branch }]}
                navLinks={[
                    { label: "Code", href: `/${user}/${repo}`, icon: <Code className="h-[18px] w-[18px]" /> },
                    { label: "Issues", href: `/${user}/${repo}/issues`, icon: <AlertCircle className="h-[18px] w-[18px]" /> },
                    { label: "Merge Requests", href: `/${user}/${repo}/merge-requests`, icon: <GitMerge className="h-[18px] w-[18px]" /> },
                ]}
                hasNotifications
            />

            <div className="flex-1 overflow-y-auto">
                <div className="max-w-4xl mx-auto px-6 py-8">
                    <Link
                        href={`/${user}/${repo}`}
                        className="inline-flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors mb-6"
                    >
                        <ArrowLeft className="h-4 w-4" />
                        Back to repository
                    </Link>

                    <div className="mb-8">
                        <h1 className="text-2xl font-semibold mb-3 leading-snug">Commits</h1>
                        {filePath && (
                            <div className="flex items-center gap-2 text-sm mb-2">
                                <FolderOpen className="h-4 w-4 shrink-0 text-muted-foreground" />
                                <span className="text-muted-foreground">
                                    History for <code className="font-mono text-foreground">{filePath}</code>
                                </span>
                            </div>
                        )}

                        <div className="flex flex-col gap-1.5">
                            <div className="flex items-center gap-2 text-sm text-muted-foreground">
                                <GitCommit className="h-4 w-4 shrink-0" />
                                {branchesLoading ? (
                                    <Skeleton className="h-4 w-40" />
                                ) : (
                                    <span>
                                        <span className="text-foreground font-medium">{totalCommitCount}</span> commit
                                        {totalCommitCount !== 1 ? "s" : ""} on <code className="font-mono">{branch}</code>
                                    </span>
                                )}
                            </div>

                            {!isDefaultBranch &&
                                repoData != null &&
                                branchInfo != null &&
                                (branchInfo.ahead > 0 || branchInfo.behind > 0) && (
                                    <div className="flex items-center gap-3 text-sm text-muted-foreground pl-6">
                                        {branchInfo.ahead > 0 && (
                                            <span className="flex items-center gap-1">
                                                <ArrowUp className="h-3.5 w-3.5 text-green-500" />
                                                <span className="text-foreground font-medium">{branchInfo.ahead}</span> ahead of{" "}
                                                <code className="font-mono">{repoData.defaultBranch}</code>
                                            </span>
                                        )}
                                        {branchInfo.ahead > 0 && branchInfo.behind > 0 && (
                                            <span className="text-muted-foreground/40">·</span>
                                        )}
                                        {branchInfo.behind > 0 && (
                                            <span className="flex items-center gap-1">
                                                <ArrowDown className="h-3.5 w-3.5 text-yellow-500" />
                                                <span className="text-foreground font-medium">{branchInfo.behind}</span> behind
                                            </span>
                                        )}
                                    </div>
                                )}
                        </div>
                    </div>

                    {isLoading && <CommitsSkeleton />}

                    {!isLoading && allCommits.length === 0 && (
                        <div className="flex flex-col items-center justify-center py-24 text-muted-foreground gap-3">
                            <GitCommit className="h-10 w-10 opacity-20" />
                            <p className="text-sm">No commits on this branch.</p>
                        </div>
                    )}

                    {groups.length > 0 && (
                        <div className="space-y-8">
                            {groups.map(({ label: dateLabel, commits: groupCommits }) => (
                                <div key={dateLabel}>
                                    <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-4">
                                        {dateLabel}
                                    </h3>

                                    <div className="space-y-3">
                                        {groupCommits.map((commit) => (
                                            <CommitRow key={commit.sha1} commit={commit} user={user} repo={repo} />
                                        ))}
                                    </div>
                                </div>
                            ))}
                        </div>
                    )}

                    {hasMore && (
                        <div className="flex justify-center mt-8">
                            <button
                                onClick={() => setSize(size + 1)}
                                disabled={isValidating}
                                className="flex items-center gap-2 px-5 py-2 text-sm border border-border rounded-md text-muted-foreground hover:text-foreground hover:bg-accent/50 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                            >
                                {isValidating ? <Spinner className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
                                Load more commits
                            </button>
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
}

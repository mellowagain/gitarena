"use client";

import { useState, Suspense, useMemo } from "react";
import Link from "next/link";
import { useSearchParams, useRouter } from "next/navigation";
import useSWR from "swr";
import useSWRInfinite from "swr/infinite";
import { jsonFetcher } from "@/lib/fetchers";
import { Prism as SyntaxHighlighter } from "react-syntax-highlighter";
import * as allLangs from "linguist-languages";
import { TopBar } from "@/components/top-bar";
import { gitarenaTheme, detectLanguage } from "@/components/code-block";
import { PriorityIndicator, type Priority } from "@/components/priority-indicator";
import {
    Search,
    Code,
    BookOpen,
    AlertCircle,
    GitMerge,
    Users,
    FileCode,
    CheckCircle2,
    Circle,
    CircleDot,
    XCircle,
    Star,
    Lock,
    Globe,
    MessageSquare,
    ChevronDown,
} from "lucide-react";

// ── Types ──────────────────────────────────────────────────────────────────────

type SearchType = "code" | "repositories" | "issues" | "merge-requests" | "users";

interface LineFragment {
    LineOffset: number;
    Offset: number;
    MatchLength: number;
}

interface LineMatch {
    Line: string;
    LineNumber: number;
    LineStart: number;
    LineEnd: number;
    Before: string;
    After: string;
    LineFragments: LineFragment[];
    FileName: boolean;
    Score: number;
}

interface FileMatch {
    FileName: string;
    Repository: string;
    Version: string;
    Language: string;
    Branches: string[];
    Score: number;
    RepositoryID: number;
    LineMatches: LineMatch[];
}

interface CodeSearchResult {
    Files: FileMatch[] | null;
    MatchCount: number;
    FileCount: number;
    Duration: number;
    FilesConsidered: number;
    FilesLoaded: number;
    ShardsScanned: number;
    NgramMatches: number;
}

interface CodeSearchResponse {
    result: CodeSearchResult;
}

interface ExploreRepo {
    id: string;
    name: string;
    description: string;
    ownerId: string;
    ownerName: string;
    visibility: string;
    archivedAt: string | null;
    disabled: boolean;
    languages: Record<string, number>;
    stars: number;
    issues: number;
    mergeRequests: number;
}

interface RepoSearchResponse {
    total: number;
    repositories: ExploreRepo[];
}

interface SearchUser {
    id: string;
    username: string;
    admin: boolean;
}

interface UserSearchResponse {
    total: number;
    users: SearchUser[];
}

interface IssueSearchResult {
    index: number;
    title: string;
    status: string;
    labels: string[];
    priority: Priority;
    commentCount: number;
    authorUsername: string;
    repoOwner: string;
    repoName: string;
    updatedAt: string;
}

interface IssueSearchResponse {
    total: number;
    issues: IssueSearchResult[];
}

// ── Mock data (WIP tabs) ───────────────────────────────────────────────────────

const MOCK_MRS = [
    {
        id: 38,
        repo: "mellowagain/gitarena",
        title: "feat: add SSH key fingerprint display in user settings",
        status: "open",
        author: "wycats",
        createdAt: "1 day ago",
        comments: 3,
        base: "main",
        head: "feat/ssh-fingerprint",
    },
    {
        id: 22,
        repo: "mellowagain/gitarena",
        title: "fix: resolve race condition in concurrent push handler",
        status: "merged",
        author: "torvalds",
        createdAt: "5 days ago",
        comments: 9,
        base: "main",
        head: "fix/push-race",
    },
    {
        id: 104,
        repo: "rust-lang/rust",
        title: "Add experimental support for async closures",
        status: "open",
        author: "gvanrossum",
        createdAt: "3 days ago",
        comments: 24,
        base: "master",
        head: "async-closures-v3",
    },
    {
        id: 77,
        repo: "dhh/rails",
        title: "Deprecate legacy serialization API in ActiveModel",
        status: "closed",
        author: "mellowagain",
        createdAt: "2 weeks ago",
        comments: 6,
        base: "main",
        head: "deprecate-serialization",
    },
    {
        id: 15,
        repo: "mellowagain/gitarena",
        title: "refactor: split auth module into sub-crates",
        status: "merged",
        author: "wycats",
        createdAt: "1 week ago",
        comments: 11,
        base: "main",
        head: "refactor/auth-split",
    },
];

type LinguistEntry = { color?: string };

function languageColor(name: string): string {
    const entry = (allLangs as Record<string, LinguistEntry>)[name];
    if (entry?.color) {
        return entry.color;
    }
    let hash = 0;
    for (let i = 0; i < name.length; i++) {
        hash = name.charCodeAt(i) + ((hash << 5) - hash);
    }
    return `hsl(${Math.abs(hash) % 360}, 60%, 55%)`;
}

// ── Syntax-highlighted match rendering ─────────────────────────────────────────

type AstNode =
    | { type: "text"; value: string }
    | { type: "element"; tagName: string; properties: { className?: string[] }; children: AstNode[] };

type AstElement = Extract<AstNode, { type: "element" }>;

function renderNode(node: AstNode, stylesheet: Record<string, React.CSSProperties>, i: number): React.ReactNode {
    if (node.type === "text") {
        return node.value;
    }
    const style = (node.properties?.className ?? []).reduce<React.CSSProperties>(
        (acc, cls) => ({ ...acc, ...(stylesheet[cls] ?? {}) }),
        {}
    );
    return (
        <span key={i} style={style}>
            {node.children.map((child, j) => renderNode(child, stylesheet, j))}
        </span>
    );
}

/**
 * Merges adjacent/overlapping LineMatches into groups so they render as one block.
 * Two matches are merged if the end of one's range (including after-context) overlaps
 * or is adjacent to the start of the next's range (including before-context).
 */
function groupLineMatches(matches: LineMatch[]): LineMatch[][] {
    if (matches.length === 0) {
        return [];
    }
    const sorted = [...matches].sort((a, b) => a.LineNumber - b.LineNumber);
    const groups: LineMatch[][] = [[sorted[0]]];

    for (let i = 1; i < sorted.length; i++) {
        const prev = groups[groups.length - 1];
        const lastMatch = prev[prev.length - 1];
        const lastAfterCount = lastMatch.After ? atob(lastMatch.After).split("\n").filter(Boolean).length : 0;
        const lastEnd = lastMatch.LineNumber + lastAfterCount;

        const curr = sorted[i];
        const currBeforeCount = curr.Before ? atob(curr.Before).split("\n").filter(Boolean).length : 0;
        const currStart = curr.LineNumber - currBeforeCount;

        if (currStart <= lastEnd + 1) {
            prev.push(curr);
        } else {
            groups.push([curr]);
        }
    }
    return groups;
}

function HighlightedMatchGroup({ matches, fileName }: { matches: LineMatch[]; fileName: string }) {
    // Build a deduplicated list of lines with metadata
    const lineEntries: { lineNum: number; text: string; isMatch: boolean; fragments?: LineFragment[] }[] = [];
    const seenLines = new Set<number>();

    for (const match of matches) {
        const line = atob(match.Line);
        const beforeLines = match.Before ? atob(match.Before).split("\n").filter(Boolean) : [];
        const afterLines = match.After ? atob(match.After).split("\n").filter(Boolean) : [];

        for (let k = 0; k < beforeLines.length; k++) {
            const num = match.LineNumber - (beforeLines.length - k);
            if (!seenLines.has(num)) {
                seenLines.add(num);
                lineEntries.push({ lineNum: num, text: beforeLines[k], isMatch: false });
            }
        }

        if (!seenLines.has(match.LineNumber)) {
            seenLines.add(match.LineNumber);
            lineEntries.push({ lineNum: match.LineNumber, text: line, isMatch: true, fragments: match.LineFragments });
        }

        for (let k = 0; k < afterLines.length; k++) {
            const num = match.LineNumber + k + 1;
            if (!seenLines.has(num)) {
                seenLines.add(num);
                lineEntries.push({ lineNum: num, text: afterLines[k], isMatch: false });
            }
        }
    }

    lineEntries.sort((a, b) => a.lineNum - b.lineNum);

    const content = lineEntries.map((e) => e.text).join("\n");
    const language = detectLanguage(fileName);

    return (
        <SyntaxHighlighter
            language={language}
            style={gitarenaTheme}
            PreTag="div"
            renderer={({ rows, stylesheet }) => (
                <>
                    {(rows as AstElement[]).map((row, i) => {
                        const entry = lineEntries[i];
                        if (!entry) {
                            return null;
                        }
                        return (
                            <div key={i} className={`flex ${entry.isMatch ? "group hover:bg-accent/20 transition-colors" : "opacity-40"}`}>
                                <span className="w-12 shrink-0 px-3 py-0.5 text-xs font-mono text-muted-foreground/50 text-right select-none border-r border-border/40 leading-5">
                                    {entry.lineNum}
                                </span>
                                <span className="px-4 py-0.5 text-xs leading-5 whitespace-pre">
                                    {entry.isMatch && entry.fragments
                                        ? highlightFragmentsOverNodes(row.children, entry.fragments, stylesheet)
                                        : row.children.map((node, j) => renderNode(node, stylesheet, j))}
                                </span>
                            </div>
                        );
                    })}
                </>
            )}
        >
            {content}
        </SyntaxHighlighter>
    );
}

/**
 * Overlays match-highlight marks on top of syntax-highlighted AST nodes.
 * Walks the token tree, inserting <mark> elements at the byte offsets indicated by LineFragments.
 */
function highlightFragmentsOverNodes(
    nodes: AstNode[],
    fragments: LineFragment[],
    stylesheet: Record<string, React.CSSProperties>
): React.ReactNode {
    if (!fragments || fragments.length === 0) {
        return nodes.map((node, i) => renderNode(node, stylesheet, i));
    }

    // Flatten to get the full text, then overlay marks at fragment positions
    const fullText = flattenText(nodes);
    const sorted = [...fragments].sort((a, b) => a.Offset - b.Offset);

    // Build intervals: [start, end, isHighlight]
    const intervals: { start: number; end: number; highlight: boolean }[] = [];
    let cursor = 0;
    for (const frag of sorted) {
        if (frag.Offset > cursor) {
            intervals.push({ start: cursor, end: frag.Offset, highlight: false });
        }
        intervals.push({ start: frag.Offset, end: frag.Offset + frag.MatchLength, highlight: true });
        cursor = frag.Offset + frag.MatchLength;
    }
    if (cursor < fullText.length) {
        intervals.push({ start: cursor, end: fullText.length, highlight: false });
    }

    // Now re-render the AST but wrap highlighted ranges in <mark>
    return renderWithHighlights(nodes, intervals, stylesheet);
}

function flattenText(nodes: AstNode[]): string {
    let text = "";
    for (const node of nodes) {
        if (node.type === "text") {
            text += node.value;
        } else {
            text += flattenText(node.children);
        }
    }
    return text;
}

function renderWithHighlights(
    nodes: AstNode[],
    intervals: { start: number; end: number; highlight: boolean }[],
    stylesheet: Record<string, React.CSSProperties>
): React.ReactNode {
    const result: React.ReactNode[] = [];
    let charOffset = 0;
    let intervalIdx = 0;

    function walk(node: AstNode, inheritedStyle: React.CSSProperties): void {
        if (node.type === "text") {
            let textOffset = 0;
            while (textOffset < node.value.length && intervalIdx < intervals.length) {
                const interval = intervals[intervalIdx];
                const globalPos = charOffset + textOffset;
                const remaining = node.value.length - textOffset;
                const intervalRemaining = interval.end - globalPos;
                const take = Math.min(remaining, intervalRemaining);
                const slice = node.value.slice(textOffset, textOffset + take);

                if (interval.highlight) {
                    result.push(
                        <mark key={`h-${globalPos}`} className="bg-yellow-400/30 text-foreground rounded-sm" style={inheritedStyle}>
                            {slice}
                        </mark>
                    );
                } else {
                    result.push(
                        <span key={`t-${globalPos}`} style={inheritedStyle}>
                            {slice}
                        </span>
                    );
                }

                textOffset += take;
                if (charOffset + textOffset >= interval.end) {
                    intervalIdx++;
                }
            }
            charOffset += node.value.length;
        } else {
            const style = {
                ...inheritedStyle,
                ...(node.properties?.className ?? []).reduce<React.CSSProperties>(
                    (acc, cls) => ({ ...acc, ...(stylesheet[cls] ?? {}) }),
                    {}
                ),
            };
            for (const child of node.children) {
                walk(child, style);
            }
        }
    }

    for (const node of nodes) {
        walk(node, {});
    }
    return result;
}

// ── Sub-components ─────────────────────────────────────────────────────────────

function CodeResultsSkeleton() {
    return (
        <div className="space-y-3">
            {[1, 2, 3].map((i) => (
                <div key={i} className="border border-border rounded-lg overflow-hidden animate-pulse">
                    <div className="flex items-center gap-3 px-4 py-2.5 bg-secondary/40 border-b border-border">
                        <div className="h-4 w-4 rounded bg-muted" />
                        <div className="h-3 w-32 rounded bg-muted" />
                        <div className="h-3 w-48 rounded bg-muted" />
                    </div>
                    <div className="divide-y divide-border/40">
                        {[1, 2].map((j) => (
                            <div key={j} className="flex items-start">
                                <div className="w-12 shrink-0 px-3 py-2.5 border-r border-border/40">
                                    <div className="h-3 w-4 rounded bg-muted mx-auto" />
                                </div>
                                <div className="flex-1 px-4 py-2.5">
                                    <div className="h-3 w-3/4 rounded bg-muted" />
                                </div>
                            </div>
                        ))}
                    </div>
                </div>
            ))}
        </div>
    );
}

function CodeResults({ query }: { query: string }) {
    const { data, error, isLoading } = useSWR<CodeSearchResponse>(query ? `/api/search/code?query=${encodeURIComponent(query)}` : null, {
        revalidateOnFocus: false,
        revalidateOnReconnect: false,
    });

    if (isLoading) {
        return <CodeResultsSkeleton />;
    }

    if (error) {
        return (
            <div className="border border-border rounded-lg px-6 py-12 text-center">
                <p className="text-sm text-muted-foreground">Failed to load results. Please try again.</p>
            </div>
        );
    }

    if (!query) {
        return (
            <div className="border border-border rounded-lg px-6 py-12 text-center">
                <p className="text-sm text-muted-foreground">Enter a query to search code.</p>
            </div>
        );
    }

    const files = data?.result?.Files;

    if (!files || files.length === 0) {
        return (
            <div className="border border-border rounded-lg px-6 py-12 text-center">
                <p className="text-sm text-muted-foreground">No code results for &ldquo;{query}&rdquo;.</p>
            </div>
        );
    }

    return (
        <div className="space-y-3">
            {files.map((file, i) => {
                // Strip first path segment (duplicates the repo name, e.g. "gitarena/src/ipc.rs" -> "src/ipc.rs")
                const strippedPath = file.FileName.includes("/") ? file.FileName.slice(file.FileName.indexOf("/") + 1) : file.FileName;
                const fileHref = `/${file.Repository}/blob/${file.Version}/${strippedPath}`;

                return (
                    <div key={i} className="border border-border rounded-lg overflow-hidden hover:border-border/80 transition-colors">
                        {/* File header */}
                        <div className="flex items-center gap-3 px-4 py-2.5 bg-secondary/40 border-b border-border">
                            <FileCode className="h-4 w-4 text-muted-foreground shrink-0" />
                            <Link
                                href={`/${file.Repository}`}
                                className="text-sm text-muted-foreground hover:text-foreground transition-colors shrink-0"
                            >
                                {file.Repository}
                            </Link>
                            <span className="text-muted-foreground/40">/</span>
                            <Link href={fileHref} className="text-sm font-medium hover:underline font-mono truncate">
                                {strippedPath}
                            </Link>
                            <div className="ml-auto flex items-center gap-1.5 shrink-0">
                                <span className="h-2.5 w-2.5 rounded-full" style={{ backgroundColor: languageColor(file.Language) }} />
                                <span className="text-xs text-muted-foreground">{file.Language}</span>
                            </div>
                        </div>

                        {/* Matched lines */}
                        <div className="overflow-x-auto">
                            <div className="min-w-max">
                                {groupLineMatches(file.LineMatches ?? []).map((group, j) => (
                                    <div key={j}>
                                        {j > 0 && (
                                            <div className="flex items-center gap-2 px-3 py-1 bg-secondary/60 border-y border-border/60">
                                                <span className="text-xs font-mono text-muted-foreground/60 select-none">···</span>
                                            </div>
                                        )}
                                        <HighlightedMatchGroup matches={group} fileName={file.FileName} />
                                    </div>
                                ))}
                            </div>
                        </div>
                    </div>
                );
            })}
        </div>
    );
}

function RepoResults({ query }: { query: string }) {
    const PAGE_SIZE = 20;
    const { data, error, isLoading, isValidating, size, setSize } = useSWRInfinite<RepoSearchResponse>(
        (index) =>
            query ? `/api/search/repositories?query=${encodeURIComponent(query)}&offset=${index * PAGE_SIZE}&limit=${PAGE_SIZE}` : null,
        { revalidateOnFocus: false, revalidateOnReconnect: false }
    );

    const allRepos = data?.flatMap((page) => page.repositories) ?? [];
    const lastPage = data ? data[data.length - 1] : null;
    const hasMore = !isLoading && lastPage != null && lastPage.repositories.length === PAGE_SIZE;

    if (!query) {
        return <p className="text-sm text-muted-foreground">Enter a query to search repositories.</p>;
    }

    if (error) {
        return (
            <div className="flex flex-col items-center justify-center py-20 text-muted-foreground border border-border rounded-lg">
                <Search className="h-12 w-12 mb-4 opacity-30" />
                <p className="text-lg font-medium">Failed to load repositories</p>
            </div>
        );
    }

    if (isLoading) {
        return (
            <div className="border border-border rounded-lg overflow-hidden">
                {Array.from({ length: 5 }, (_, i) => (
                    <div key={i} className={`flex items-start gap-4 px-4 py-4 ${i > 0 ? "border-t border-border" : ""}`}>
                        <div className="h-9 w-9 rounded-md bg-secondary border border-border shrink-0 animate-pulse" />
                        <div className="flex-1 space-y-2">
                            <div className="h-4 bg-secondary rounded animate-pulse w-48" />
                            <div className="h-3 bg-secondary rounded animate-pulse w-72" />
                            <div className="h-3 bg-secondary rounded animate-pulse w-32" />
                        </div>
                    </div>
                ))}
            </div>
        );
    }

    if (allRepos.length === 0) {
        return (
            <div className="flex flex-col items-center justify-center py-20 text-muted-foreground border border-border rounded-lg">
                <Search className="h-12 w-12 mb-4 opacity-30" />
                <p className="text-lg font-medium">No repositories found</p>
            </div>
        );
    }

    const primaryLanguage = (languages: Record<string, number>): string | null => {
        const entries = Object.entries(languages);
        if (entries.length === 0) {
            return null;
        }
        return entries.reduce((a, b) => (b[1] > a[1] ? b : a))[0];
    };

    return (
        <>
            <div className="border border-border rounded-lg overflow-hidden">
                {allRepos.map((repo, i) => {
                    const lang = primaryLanguage(repo.languages);
                    return (
                        <div
                            key={repo.id}
                            className={`flex items-start gap-4 px-4 py-4 hover:bg-accent/20 transition-colors ${i > 0 ? "border-t border-border" : ""}`}
                        >
                            <div className="h-9 w-9 rounded-md bg-secondary border border-border flex items-center justify-center text-sm font-semibold shrink-0">
                                {repo.name[0].toUpperCase()}
                            </div>
                            <div className="flex-1 min-w-0">
                                <div className="flex items-center gap-2 mb-1 flex-wrap">
                                    <Link href={`/${repo.ownerName}/${repo.name}`} className="text-sm font-medium hover:underline">
                                        {repo.ownerName}/{repo.name}
                                    </Link>
                                    <span className="flex items-center gap-1 px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wider border border-border rounded text-muted-foreground bg-secondary">
                                        {repo.visibility === "private" ? (
                                            <Lock className="h-2.5 w-2.5" />
                                        ) : (
                                            <Globe className="h-2.5 w-2.5" />
                                        )}
                                        {repo.visibility}
                                    </span>
                                    {repo.archivedAt && (
                                        <span className="px-1.5 py-0.5 text-[10px] font-medium rounded bg-secondary text-muted-foreground border border-border leading-none shrink-0">
                                            archived
                                        </span>
                                    )}
                                </div>
                                {repo.description && (
                                    <p className="text-sm text-muted-foreground leading-relaxed mb-2">{repo.description}</p>
                                )}
                                <div className="flex items-center gap-4 text-xs text-muted-foreground">
                                    {lang && (
                                        <span className="flex items-center gap-1.5">
                                            <span className="h-2.5 w-2.5 rounded-full" style={{ backgroundColor: languageColor(lang) }} />
                                            {lang}
                                        </span>
                                    )}
                                    <span className="flex items-center gap-1">
                                        <Star className="h-3 w-3" />
                                        {repo.stars.toLocaleString()}
                                    </span>
                                </div>
                            </div>
                        </div>
                    );
                })}
            </div>
            {hasMore && (
                <div className="flex justify-center mt-6">
                    <button
                        onClick={() => setSize(size + 1)}
                        disabled={isValidating}
                        className="flex items-center gap-2 px-5 py-2 text-sm border border-border rounded-md text-muted-foreground hover:text-foreground hover:bg-accent/50 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                        <ChevronDown className="h-4 w-4" />
                        Load more
                    </button>
                </div>
            )}
        </>
    );
}

interface IssueLabel {
    name: string;
    color: string;
}

interface LabelsResponse {
    labels: IssueLabel[];
}

function LabelBadge({ name, color }: { name: string; color: string }) {
    const scopedIndex = name.indexOf("::");
    const isScoped = scopedIndex !== -1;
    const scopeKey = isScoped ? name.slice(0, scopedIndex) : null;
    const scopeValue = isScoped ? name.slice(scopedIndex + 2) : null;

    if (isScoped) {
        return (
            <span className="inline-flex items-center text-xs rounded overflow-hidden shrink-0">
                <span className="px-2 py-0.5 font-medium" style={{ backgroundColor: `${color}35`, color }}>
                    {scopeKey}
                </span>
                <span className="px-2 py-0.5" style={{ backgroundColor: `${color}20`, color }}>
                    {scopeValue}
                </span>
            </span>
        );
    }
    return (
        <span className="shrink-0 px-2 py-0.5 text-xs rounded" style={{ backgroundColor: `${color}20`, color }}>
            {name}
        </span>
    );
}

function IssueResults({ query }: { query: string }) {
    const PAGE_SIZE = 20;
    const { data, error, isLoading, isValidating, size, setSize } = useSWRInfinite<IssueSearchResponse>(
        (index) => (query ? `/api/search/issues?query=${encodeURIComponent(query)}&offset=${index * PAGE_SIZE}&limit=${PAGE_SIZE}` : null),
        { revalidateOnFocus: false, revalidateOnReconnect: false }
    );

    const allIssues = data?.flatMap((page) => page.issues) ?? [];
    const lastPage = data ? data[data.length - 1] : null;
    const hasMore = !isLoading && lastPage != null && lastPage.issues.length === PAGE_SIZE;

    const uniqueRepos = useMemo(
        () => [...new Set(allIssues.map((i) => `${i.repoOwner}/${i.repoName}`))],
        // eslint-disable-next-line react-hooks/exhaustive-deps
        [data]
    );

    const { data: labelColorMap } = useSWR(
        uniqueRepos.length > 0 ? ["issue-search-labels", ...uniqueRepos] : null,
        async ([, ...repos]: string[]) => {
            const results = await Promise.all(repos.map((repo) => jsonFetcher(`/api/repos/${repo}/labels`) as Promise<LabelsResponse>));
            const map = new Map<string, string>();
            results.forEach((labelData, i) => {
                const repo = repos[i];
                labelData.labels?.forEach((l) => {
                    map.set(`${repo}::${l.name}`, l.color);
                });
            });
            return map;
        },
        { revalidateOnFocus: false }
    );

    if (!query) {
        return <p className="text-sm text-muted-foreground">Enter a query to search issues.</p>;
    }

    if (error) {
        return (
            <div className="flex flex-col items-center justify-center py-20 text-muted-foreground border border-border rounded-lg">
                <Search className="h-12 w-12 mb-4 opacity-30" />
                <p className="text-lg font-medium">Failed to load issues</p>
            </div>
        );
    }

    if (isLoading) {
        return (
            <div className="border border-border rounded-lg overflow-hidden">
                {Array.from({ length: 5 }, (_, i) => (
                    <div key={i} className={`flex items-start gap-3 px-4 py-3 ${i > 0 ? "border-t border-border" : ""}`}>
                        <div className="h-4 w-4 rounded-full bg-secondary border border-border shrink-0 animate-pulse mt-0.5" />
                        <div className="flex-1 space-y-2">
                            <div className="h-4 bg-secondary rounded animate-pulse w-64" />
                            <div className="h-3 bg-secondary rounded animate-pulse w-40" />
                        </div>
                    </div>
                ))}
            </div>
        );
    }

    if (allIssues.length === 0) {
        return (
            <div className="flex flex-col items-center justify-center py-20 text-muted-foreground border border-border rounded-lg">
                <Search className="h-12 w-12 mb-4 opacity-30" />
                <p className="text-lg font-medium">No issues found</p>
            </div>
        );
    }

    return (
        <>
            <div className="border border-border rounded-lg overflow-hidden">
                {allIssues.map((issue, i) => (
                    <div
                        key={`${issue.repoOwner}-${issue.repoName}-${issue.index}`}
                        className={`flex items-stretch hover:bg-accent/20 transition-colors ${i > 0 ? "border-t border-border" : ""}`}
                    >
                        <div
                            className={`w-1 shrink-0 ${issue.status === "open" || issue.status === "in_progress" ? "bg-green-500" : "bg-muted-foreground/40"}`}
                        />
                        <div className="flex items-start gap-3 flex-1 px-4 py-3">
                            <div className="mt-0.5 shrink-0">
                                {issue.status === "open" ? (
                                    <Circle className="h-4 w-4 text-green-500" />
                                ) : issue.status === "in_progress" ? (
                                    <CircleDot className="h-4 w-4 text-yellow-500" />
                                ) : issue.status === "not_planned" ? (
                                    <XCircle className="h-4 w-4 text-muted-foreground" />
                                ) : (
                                    <CheckCircle2 className="h-4 w-4 text-muted-foreground" />
                                )}
                            </div>
                            <PriorityIndicator priority={issue.priority} />
                            <div className="flex-1 min-w-0">
                                <div className="flex items-center gap-2 flex-wrap mb-1">
                                    <Link
                                        href={`/${issue.repoOwner}/${issue.repoName}/issues/${issue.index}`}
                                        className="text-sm font-medium hover:underline"
                                    >
                                        {issue.title}
                                    </Link>
                                    {issue.labels.map((l) => (
                                        <LabelBadge
                                            key={l}
                                            name={l}
                                            color={labelColorMap?.get(`${issue.repoOwner}/${issue.repoName}::${l}`) ?? "#888888"}
                                        />
                                    ))}
                                </div>
                                <div className="text-xs text-muted-foreground">
                                    <Link href={`/${issue.repoOwner}/${issue.repoName}`} className="hover:underline">
                                        {issue.repoOwner}/{issue.repoName}
                                    </Link>{" "}
                                    <span className="font-mono">#{issue.index}</span>
                                </div>
                            </div>
                            {issue.commentCount > 0 && (
                                <div className="flex items-center gap-1 text-xs text-muted-foreground shrink-0">
                                    <MessageSquare className="h-3.5 w-3.5" />
                                    {issue.commentCount}
                                </div>
                            )}
                        </div>
                    </div>
                ))}
            </div>
            {hasMore && (
                <div className="flex justify-center mt-6">
                    <button
                        onClick={() => setSize(size + 1)}
                        disabled={isValidating}
                        className="flex items-center gap-2 px-5 py-2 text-sm border border-border rounded-md text-muted-foreground hover:text-foreground hover:bg-accent/50 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                        <ChevronDown className="h-4 w-4" />
                        Load more
                    </button>
                </div>
            )}
        </>
    );
}

function MRResults() {
    const statusColor: Record<string, string> = {
        open: "bg-green-500",
        merged: "bg-violet-500",
        closed: "bg-muted-foreground/40",
    };
    const statusIcon: Record<string, React.ReactNode> = {
        open: <GitMerge className="h-4 w-4 text-green-500" />,
        merged: <GitMerge className="h-4 w-4 text-violet-500" />,
        closed: <GitMerge className="h-4 w-4 text-muted-foreground" />,
    };
    return (
        <div className="border border-border rounded-lg overflow-hidden">
            {MOCK_MRS.map((mr, i) => (
                <div
                    key={`${mr.repo}-${mr.id}`}
                    className={`pl-1 flex items-stretch hover:bg-accent/20 transition-colors ${i > 0 ? "border-t border-border" : ""}`}
                >
                    <div className={`w-1 shrink-0 ${statusColor[mr.status]}`} />
                    <div className="flex items-start gap-3 flex-1 px-4 py-3">
                        <div className="mt-0.5 shrink-0">{statusIcon[mr.status]}</div>
                        <div className="flex-1 min-w-0">
                            <div className="mb-1">
                                <Link href="#" className="text-sm font-medium hover:underline">
                                    {mr.title}
                                </Link>
                            </div>
                            <div className="text-xs text-muted-foreground">
                                <Link href={`/${mr.repo}`} className="hover:underline">
                                    {mr.repo}
                                </Link>
                                {" · "}!{mr.id}
                                {" · "}
                                <code className="font-mono">{mr.head}</code>
                                {" → "}
                                <code className="font-mono">{mr.base}</code>
                                {" · "}opened {mr.createdAt} by {mr.author}
                            </div>
                        </div>
                        {mr.comments > 0 && (
                            <div className="flex items-center gap-1 text-xs text-muted-foreground shrink-0">
                                <MessageSquare className="h-3.5 w-3.5" />
                                {mr.comments}
                            </div>
                        )}
                    </div>
                </div>
            ))}
        </div>
    );
}

function UserResults({ query }: { query: string }) {
    const PAGE_SIZE = 20;
    const { data, error, isLoading, isValidating, size, setSize } = useSWRInfinite<UserSearchResponse>(
        (index) => (query ? `/api/search/users?query=${encodeURIComponent(query)}&offset=${index * PAGE_SIZE}&limit=${PAGE_SIZE}` : null),
        { revalidateOnFocus: false, revalidateOnReconnect: false }
    );

    const allUsers = data?.flatMap((page) => page.users) ?? [];
    const lastPage = data ? data[data.length - 1] : null;
    const hasMore = !isLoading && lastPage != null && lastPage.users.length === PAGE_SIZE;

    if (!query) {
        return <p className="text-sm text-muted-foreground">Enter a query to search users.</p>;
    }

    if (error) {
        return (
            <div className="flex flex-col items-center justify-center py-20 text-muted-foreground border border-border rounded-lg">
                <Search className="h-12 w-12 mb-4 opacity-30" />
                <p className="text-lg font-medium">Failed to load users</p>
            </div>
        );
    }

    if (isLoading) {
        return (
            <div className="border border-border rounded-lg overflow-hidden">
                {Array.from({ length: 5 }, (_, i) => (
                    <div key={i} className={`flex items-center gap-4 px-4 py-3.5 ${i > 0 ? "border-t border-border" : ""}`}>
                        <div className="h-10 w-10 rounded-full bg-secondary border border-border shrink-0 animate-pulse" />
                        <div className="flex-1 space-y-2">
                            <div className="h-4 bg-secondary rounded animate-pulse w-32" />
                            <div className="h-3 bg-secondary rounded animate-pulse w-20" />
                        </div>
                    </div>
                ))}
            </div>
        );
    }

    if (allUsers.length === 0) {
        return (
            <div className="flex flex-col items-center justify-center py-20 text-muted-foreground border border-border rounded-lg">
                <Search className="h-12 w-12 mb-4 opacity-30" />
                <p className="text-lg font-medium">No users found</p>
            </div>
        );
    }

    return (
        <>
            <div className="border border-border rounded-lg overflow-hidden">
                {allUsers.map((user, i) => (
                    <div
                        key={user.id}
                        className={`flex items-center gap-4 px-4 py-3.5 hover:bg-accent/20 transition-colors ${i > 0 ? "border-t border-border" : ""}`}
                    >
                        <div className="h-10 w-10 rounded-full bg-secondary border border-border flex items-center justify-center text-sm font-semibold shrink-0">
                            {user.username[0].toUpperCase()}
                        </div>
                        <div className="flex-1 min-w-0">
                            <div className="flex items-center gap-2 mb-0.5">
                                <Link href={`/${user.username}`} className="text-sm font-medium hover:underline">
                                    {user.username}
                                </Link>
                                {user.admin && (
                                    <span className="px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wider border border-border rounded text-muted-foreground bg-secondary">
                                        admin
                                    </span>
                                )}
                            </div>
                        </div>
                    </div>
                ))}
            </div>
            {hasMore && (
                <div className="flex justify-center mt-6">
                    <button
                        onClick={() => setSize(size + 1)}
                        disabled={isValidating}
                        className="flex items-center gap-2 px-5 py-2 text-sm border border-border rounded-md text-muted-foreground hover:text-foreground hover:bg-accent/50 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                        <ChevronDown className="h-4 w-4" />
                        Load more
                    </button>
                </div>
            )}
        </>
    );
}

// ── Page ───────────────────────────────────────────────────────────────────────

const TABS: { id: SearchType; label: string; icon: React.ElementType }[] = [
    { id: "code", label: "Code", icon: Code },
    { id: "repositories", label: "Repositories", icon: BookOpen },
    { id: "issues", label: "Issues", icon: AlertCircle },
    { id: "merge-requests", label: "Merge Requests", icon: GitMerge },
    { id: "users", label: "Users", icon: Users },
];

function SearchPageContent() {
    const searchParams = useSearchParams();
    const router = useRouter();
    const [activeType, setActiveType] = useState<SearchType>("code");

    const q = searchParams.get("query") ?? "";
    const [inputValue, setInputValue] = useState(q);

    // Fetch code results to get counts for the sidebar
    const { data: codeData } = useSWR<CodeSearchResponse>(q ? `/api/search/code?query=${encodeURIComponent(q)}` : null, {
        revalidateOnFocus: false,
        revalidateOnReconnect: false,
    });

    const { data: repoData } = useSWR<RepoSearchResponse>(q ? `/api/search/repositories?query=${encodeURIComponent(q)}&limit=1` : null, {
        revalidateOnFocus: false,
        revalidateOnReconnect: false,
    });

    const { data: userData } = useSWR<UserSearchResponse>(q ? `/api/search/users?query=${encodeURIComponent(q)}&limit=1` : null, {
        revalidateOnFocus: false,
        revalidateOnReconnect: false,
    });

    const { data: issueData } = useSWR<IssueSearchResponse>(q ? `/api/search/issues?query=${encodeURIComponent(q)}&limit=1` : null, {
        revalidateOnFocus: false,
        revalidateOnReconnect: false,
    });

    const codeFileCount = codeData?.result?.FileCount ?? 0;
    const codeMatchCount = codeData?.result?.MatchCount ?? 0;
    const codeDurationMs = codeData ? Math.round((codeData.result?.Duration ?? 0) / 1000) : null;

    function handleSearchKeyDown(e: React.KeyboardEvent<HTMLInputElement>) {
        if (e.key === "Enter") {
            const trimmed = inputValue.trim();
            if (trimmed) {
                router.push(`/search?query=${encodeURIComponent(trimmed)}`);
            }
        }
    }

    function getTabCount(id: SearchType): number | null {
        if (id === "code") {
            return q ? codeFileCount : null;
        }
        if (id === "repositories") {
            return q && repoData ? repoData.total : null;
        }
        if (id === "issues") {
            return q && issueData ? issueData.total : null;
        }
        if (id === "users") {
            return q && userData ? userData.total : null;
        }
        return null;
    }

    const activeCount = getTabCount(activeType);

    return (
        <div className="flex flex-col h-screen overflow-hidden bg-background text-foreground font-sans">
            <TopBar hasNotifications />

            {/* Search bar strip */}
            <div className="border-b border-border shrink-0">
                <div className="max-w-6xl mx-auto px-6 py-3">
                    <div className="relative max-w-2xl">
                        <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground pointer-events-none" />
                        <input
                            type="text"
                            value={inputValue}
                            onChange={(e) => setInputValue(e.target.value)}
                            onKeyDown={handleSearchKeyDown}
                            placeholder="Search GitArena..."
                            className="w-full h-10 pl-10 pr-4 bg-card border border-border rounded-md text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
                        />
                    </div>
                </div>
            </div>

            {/* Body */}
            <div className="flex-1 overflow-y-auto">
                <div className="max-w-6xl mx-auto px-6 py-6 flex gap-8">
                    {/* Left: type filter sidebar */}
                    <aside className="w-48 shrink-0">
                        <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">Type</h3>
                        <nav className="space-y-0.5">
                            {TABS.map(({ id, label, icon: Icon }) => {
                                const count = getTabCount(id);
                                return (
                                    <button
                                        key={id}
                                        onClick={() => setActiveType(id)}
                                        className={`w-full flex items-center justify-between px-3 py-2 rounded-md text-sm transition-colors text-left ${
                                            activeType === id
                                                ? "bg-accent text-foreground font-medium"
                                                : "text-muted-foreground hover:text-foreground hover:bg-accent/50"
                                        }`}
                                    >
                                        <span className="flex items-center gap-2.5">
                                            <Icon className="h-4 w-4 shrink-0" />
                                            {label}
                                        </span>
                                        {count !== null && (
                                            <span
                                                className={`text-xs font-mono tabular-nums ${activeType === id ? "text-foreground" : "text-muted-foreground/60"}`}
                                            >
                                                {count}
                                            </span>
                                        )}
                                    </button>
                                );
                            })}
                        </nav>
                    </aside>

                    {/* Main: results */}
                    <main className="flex-1 min-w-0">
                        {/* Result count */}
                        {q && (
                            <p className="text-sm text-muted-foreground mb-5">
                                {activeType === "code" && codeData ? (
                                    <>
                                        <span className="text-foreground font-medium">{codeMatchCount.toLocaleString()}</span>{" "}
                                        {codeMatchCount === 1 ? "match" : "matches"} in{" "}
                                        <span className="text-foreground font-medium">{codeFileCount.toLocaleString()}</span>{" "}
                                        {codeFileCount === 1 ? "file" : "files"} for{" "}
                                        <span className="font-medium text-foreground">&ldquo;{q}&rdquo;</span>
                                        {codeDurationMs !== null && (
                                            <span className="ml-2 text-muted-foreground/60">({codeDurationMs}ms)</span>
                                        )}
                                    </>
                                ) : (
                                    <>
                                        <span className="text-foreground font-medium">{(activeCount ?? 0).toLocaleString()}</span>{" "}
                                        {activeCount === 1 ? "result" : "results"} for{" "}
                                        <span className="font-medium text-foreground">&ldquo;{q}&rdquo;</span>
                                    </>
                                )}
                            </p>
                        )}

                        {activeType === "code" && <CodeResults query={q} />}
                        {activeType === "repositories" && <RepoResults query={q} />}
                        {activeType === "issues" && <IssueResults query={q} />}
                        {activeType === "merge-requests" && <MRResults />}
                        {activeType === "users" && <UserResults query={q} />}
                    </main>
                </div>
            </div>
        </div>
    );
}

export default function SearchPage() {
    return (
        <Suspense>
            <SearchPageContent />
        </Suspense>
    );
}

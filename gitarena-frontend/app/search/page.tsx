"use client";

import { useState } from "react";
import Link from "next/link";
import { useSearchParams, useRouter } from "next/navigation";
import useSWR from "swr";
import { TopBar } from "@/components/top-bar";
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
    Star,
    GitFork,
    Lock,
    Globe,
    MessageSquare,
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
    Before: string[];
    After: string[];
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
    Result: CodeSearchResult;
}

// ── Mock data (WIP tabs) ───────────────────────────────────────────────────────

const MOCK_REPOSITORIES = [
    {
        full_name: "mellowagain/gitarena",
        description: "A lightweight, self-hosted git forge written in Rust",
        stars: 892,
        forks: 43,
        language: "Rust",
        visibility: "public",
        updatedAt: "2 hours ago",
    },
    {
        full_name: "torvalds/linux",
        description: "Linux kernel source tree",
        stars: 18423,
        forks: 4821,
        language: "C",
        visibility: "public",
        updatedAt: "1 hour ago",
    },
    {
        full_name: "gvanrossum/cpython",
        description: "The Python programming language",
        stars: 12301,
        forks: 2901,
        language: "Python",
        visibility: "public",
        updatedAt: "3 hours ago",
    },
    {
        full_name: "dhh/rails",
        description: "Ruby on Rails — full-stack web framework",
        stars: 9842,
        forks: 1231,
        language: "Ruby",
        visibility: "public",
        updatedAt: "2 days ago",
    },
    {
        full_name: "rust-lang/rust",
        description: "Empowering everyone to build reliable and efficient software",
        stars: 14201,
        forks: 2102,
        language: "Rust",
        visibility: "public",
        updatedAt: "30 min ago",
    },
    {
        full_name: "acme-inc/internal-api",
        description: "Internal REST API for ACME product suite",
        stars: 0,
        forks: 0,
        language: "TypeScript",
        visibility: "private",
        updatedAt: "4 hours ago",
    },
];

const MOCK_ISSUES = [
    {
        id: 142,
        repo: "mellowagain/gitarena",
        title: "SSH key authentication fails on ed25519 keys",
        status: "open",
        author: "wycats",
        createdAt: "3 days ago",
        comments: 7,
        labels: ["bug", "auth"],
    },
    {
        id: 87,
        repo: "mellowagain/gitarena",
        title: "Add support for GPG commit verification",
        status: "open",
        author: "torvalds",
        createdAt: "1 week ago",
        comments: 12,
        labels: ["enhancement"],
    },
    {
        id: 234,
        repo: "rust-lang/rust",
        title: "Clippy incorrectly flags iterator chaining patterns",
        status: "closed",
        author: "gvanrossum",
        createdAt: "2 weeks ago",
        comments: 4,
        labels: ["bug", "clippy"],
    },
    {
        id: 56,
        repo: "dhh/rails",
        title: "N+1 query detection in ActiveRecord associations",
        status: "open",
        author: "mellowagain",
        createdAt: "5 days ago",
        comments: 18,
        labels: ["performance"],
    },
    {
        id: 901,
        repo: "torvalds/linux",
        title: "Memory leak in XFS journal recovery path",
        status: "closed",
        author: "wycats",
        createdAt: "1 month ago",
        comments: 31,
        labels: ["bug", "critical"],
    },
    {
        id: 63,
        repo: "mellowagain/gitarena",
        title: "Repository mirroring does not respect SSH proxy settings",
        status: "open",
        author: "dhh",
        createdAt: "2 days ago",
        comments: 2,
        labels: ["mirrors"],
    },
];

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

const MOCK_USERS = [
    { username: "mellowagain", displayName: "Mari", bio: "Building GitArena. Rust enthusiast.", repos: 34, followers: 412, following: 58 },
    { username: "torvalds", displayName: "Linus Torvalds", bio: "I'm not a people person.", repos: 12, followers: 18201, following: 0 },
    {
        username: "gvanrossum",
        displayName: "Guido van Rossum",
        bio: "Python's BDFL (emeritus). Now at Microsoft.",
        repos: 27,
        followers: 9432,
        following: 14,
    },
    {
        username: "dhh",
        displayName: "DHH",
        bio: "Creator of Ruby on Rails, Founder at 37signals.",
        repos: 19,
        followers: 7821,
        following: 3,
    },
    {
        username: "wycats",
        displayName: "Yehuda Katz",
        bio: "Rust, Ember, Ruby. @tildeio co-founder.",
        repos: 58,
        followers: 3201,
        following: 102,
    },
];

const LANG_COLORS: Record<string, string> = {
    Rust: "bg-orange-500",
    C: "bg-gray-500",
    Python: "bg-blue-500",
    Ruby: "bg-red-500",
    TypeScript: "bg-blue-400",
    JavaScript: "bg-yellow-400",
    Go: "bg-cyan-500",
};

// ── Highlight helper ───────────────────────────────────────────────────────────

function highlightFragments(text: string, fragments: LineFragment[]): React.ReactNode {
    if (!fragments || fragments.length === 0) {
        return text;
    }

    const nodes: React.ReactNode[] = [];
    let cursor = 0;

    // Sort fragments by offset in case they come out of order
    const sorted = [...fragments].sort((a, b) => a.Offset - b.Offset);

    for (const frag of sorted) {
        if (frag.Offset > cursor) {
            nodes.push(text.slice(cursor, frag.Offset));
        }
        const end = frag.Offset + frag.MatchLength;
        nodes.push(
            <mark key={frag.Offset} className="bg-yellow-400/30 text-foreground rounded-sm">
                {text.slice(frag.Offset, end)}
            </mark>
        );
        cursor = end;
    }

    if (cursor < text.length) {
        nodes.push(text.slice(cursor));
    }

    return nodes;
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
    const { data, error, isLoading } = useSWR<CodeSearchResponse>(query ? `/api/search/code?q=${encodeURIComponent(query)}` : null);

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

    const files = data?.Result?.Files;

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
                                <span className={`h-2.5 w-2.5 rounded-full ${LANG_COLORS[file.Language] ?? "bg-muted-foreground"}`} />
                                <span className="text-xs text-muted-foreground">{file.Language}</span>
                            </div>
                        </div>

                        {/* Matched lines */}
                        <div className="divide-y divide-border/40">
                            {(file.LineMatches ?? []).map((match, j) => {
                                const line = atob(match.Line);
                                const beforeLines = (match.Before ?? []).map((b) => atob(b));
                                const afterLines = (match.After ?? []).map((a) => atob(a));

                                return (
                                    <div key={j}>
                                        {/* Before context lines */}
                                        {beforeLines.map((ctx, k) => (
                                            <div key={`before-${k}`} className="flex items-start opacity-40">
                                                <span className="w-12 shrink-0 px-3 py-1.5 text-xs font-mono text-muted-foreground/50 text-right select-none border-r border-border/40 leading-5">
                                                    {match.LineNumber - (beforeLines.length - k)}
                                                </span>
                                                <pre className="flex-1 px-4 py-1.5 text-xs font-mono leading-5 overflow-x-auto whitespace-pre text-foreground/60">
                                                    {ctx}
                                                </pre>
                                            </div>
                                        ))}

                                        {/* Match line */}
                                        <div className="flex items-start group hover:bg-accent/20 transition-colors">
                                            <span className="w-12 shrink-0 px-3 py-2.5 text-xs font-mono text-muted-foreground/50 text-right select-none border-r border-border/40 leading-5">
                                                {match.LineNumber}
                                            </span>
                                            <pre className="flex-1 px-4 py-2.5 text-xs font-mono leading-5 overflow-x-auto whitespace-pre text-foreground/80">
                                                {highlightFragments(line, match.LineFragments)}
                                            </pre>
                                        </div>

                                        {/* After context lines */}
                                        {afterLines.map((ctx, k) => (
                                            <div key={`after-${k}`} className="flex items-start opacity-40">
                                                <span className="w-12 shrink-0 px-3 py-1.5 text-xs font-mono text-muted-foreground/50 text-right select-none border-r border-border/40 leading-5">
                                                    {match.LineNumber + k + 1}
                                                </span>
                                                <pre className="flex-1 px-4 py-1.5 text-xs font-mono leading-5 overflow-x-auto whitespace-pre text-foreground/60">
                                                    {ctx}
                                                </pre>
                                            </div>
                                        ))}
                                    </div>
                                );
                            })}
                        </div>
                    </div>
                );
            })}
        </div>
    );
}

function RepoResults() {
    return (
        <div className="border border-border rounded-lg overflow-hidden">
            {MOCK_REPOSITORIES.map((repo, i) => (
                <div
                    key={repo.full_name}
                    className={`flex items-start gap-4 px-4 py-4 hover:bg-accent/20 transition-colors ${i > 0 ? "border-t border-border" : ""}`}
                >
                    <div className="h-9 w-9 rounded-md bg-secondary border border-border flex items-center justify-center text-sm font-semibold shrink-0">
                        {repo.full_name.split("/")[1][0].toUpperCase()}
                    </div>
                    <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2 mb-1 flex-wrap">
                            <Link href={`/${repo.full_name}`} className="text-sm font-medium hover:underline">
                                {repo.full_name}
                            </Link>
                            <span
                                className={`flex items-center gap-1 px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wider border border-border rounded text-muted-foreground bg-secondary`}
                            >
                                {repo.visibility === "private" ? <Lock className="h-2.5 w-2.5" /> : <Globe className="h-2.5 w-2.5" />}
                                {repo.visibility}
                            </span>
                        </div>
                        {repo.description && <p className="text-sm text-muted-foreground leading-relaxed mb-2">{repo.description}</p>}
                        <div className="flex items-center gap-4 text-xs text-muted-foreground">
                            {repo.language && (
                                <span className="flex items-center gap-1.5">
                                    <span className={`h-2.5 w-2.5 rounded-full ${LANG_COLORS[repo.language] ?? "bg-muted-foreground"}`} />
                                    {repo.language}
                                </span>
                            )}
                            <span className="flex items-center gap-1">
                                <Star className="h-3 w-3" />
                                {repo.stars.toLocaleString()}
                            </span>
                            <span className="flex items-center gap-1">
                                <GitFork className="h-3 w-3" />
                                {repo.forks.toLocaleString()}
                            </span>
                            <span>Updated {repo.updatedAt}</span>
                        </div>
                    </div>
                </div>
            ))}
        </div>
    );
}

function IssueResults() {
    return (
        <div className="border border-border rounded-lg overflow-hidden">
            {MOCK_ISSUES.map((issue, i) => (
                <div
                    key={`${issue.repo}-${issue.id}`}
                    className={`pl-1 flex items-stretch hover:bg-accent/20 transition-colors ${i > 0 ? "border-t border-border" : ""}`}
                >
                    <div className={`w-1 shrink-0 ${issue.status === "open" ? "bg-green-500" : "bg-muted-foreground/40"}`} />
                    <div className="flex items-start gap-3 flex-1 px-4 py-3">
                        <div className="mt-0.5 shrink-0">
                            {issue.status === "open" ? (
                                <Circle className="h-4 w-4 text-green-500" />
                            ) : (
                                <CheckCircle2 className="h-4 w-4 text-muted-foreground" />
                            )}
                        </div>
                        <div className="flex-1 min-w-0">
                            <div className="flex items-center gap-2 flex-wrap mb-1">
                                <Link href="#" className="text-sm font-medium hover:underline">
                                    {issue.title}
                                </Link>
                                {issue.labels.map((l) => (
                                    <span
                                        key={l}
                                        className="px-1.5 py-0.5 text-[10px] font-medium border border-border rounded bg-secondary text-muted-foreground"
                                    >
                                        {l}
                                    </span>
                                ))}
                            </div>
                            <div className="text-xs text-muted-foreground">
                                <Link href={`/${issue.repo}`} className="hover:underline">
                                    {issue.repo}
                                </Link>
                                {" · "}#{issue.id} opened {issue.createdAt} by {issue.author}
                            </div>
                        </div>
                        {issue.comments > 0 && (
                            <div className="flex items-center gap-1 text-xs text-muted-foreground shrink-0">
                                <MessageSquare className="h-3.5 w-3.5" />
                                {issue.comments}
                            </div>
                        )}
                    </div>
                </div>
            ))}
        </div>
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

function UserResults() {
    return (
        <div className="border border-border rounded-lg overflow-hidden">
            {MOCK_USERS.map((user, i) => (
                <div
                    key={user.username}
                    className={`flex items-center gap-4 px-4 py-3.5 hover:bg-accent/20 transition-colors ${i > 0 ? "border-t border-border" : ""}`}
                >
                    <div className="h-10 w-10 rounded-full bg-secondary border border-border flex items-center justify-center text-sm font-semibold shrink-0">
                        {user.displayName[0].toUpperCase()}
                    </div>
                    <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2 mb-0.5">
                            <Link href={`/${user.username}`} className="text-sm font-medium hover:underline">
                                {user.displayName}
                            </Link>
                            <span className="text-xs text-muted-foreground font-mono">{user.username}</span>
                        </div>
                        {user.bio && <p className="text-xs text-muted-foreground leading-relaxed">{user.bio}</p>}
                    </div>
                    <div className="flex items-center gap-4 text-xs text-muted-foreground shrink-0">
                        <span className="flex items-center gap-1">
                            <BookOpen className="h-3.5 w-3.5" />
                            {user.repos}
                        </span>
                        <span className="flex items-center gap-1">
                            <Users className="h-3.5 w-3.5" />
                            {user.followers.toLocaleString()}
                        </span>
                    </div>
                </div>
            ))}
        </div>
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

const MOCK_COUNTS: Record<Exclude<SearchType, "code">, number> = {
    repositories: MOCK_REPOSITORIES.length,
    issues: MOCK_ISSUES.length,
    "merge-requests": MOCK_MRS.length,
    users: MOCK_USERS.length,
};

export default function SearchPage() {
    const searchParams = useSearchParams();
    const router = useRouter();
    const [activeType, setActiveType] = useState<SearchType>("code");

    const q = searchParams.get("q") ?? "";
    const [inputValue, setInputValue] = useState(q);

    // Fetch code results to get counts for the sidebar
    const { data: codeData } = useSWR<CodeSearchResponse>(q ? `/api/search/code?q=${encodeURIComponent(q)}` : null);

    const codeFileCount = codeData?.Result?.FileCount ?? 0;
    const codeMatchCount = codeData?.Result?.MatchCount ?? 0;
    const codeDurationMs = codeData ? Math.round((codeData.Result?.Duration ?? 0) / 1000) : null;

    function handleSearchKeyDown(e: React.KeyboardEvent<HTMLInputElement>) {
        if (e.key === "Enter") {
            const trimmed = inputValue.trim();
            if (trimmed) {
                router.push(`/search?q=${encodeURIComponent(trimmed)}`);
            }
        }
    }

    function getTabCount(id: SearchType): number | null {
        if (id === "code") {
            return q ? codeFileCount : null;
        }
        return MOCK_COUNTS[id as Exclude<SearchType, "code">];
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
                        {activeType === "repositories" && <RepoResults />}
                        {activeType === "issues" && <IssueResults />}
                        {activeType === "merge-requests" && <MRResults />}
                        {activeType === "users" && <UserResults />}
                    </main>
                </div>
            </div>
        </div>
    );
}

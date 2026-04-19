"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import useSWR from "swr";
import Link from "next/link";
import {
    Plus,
    Compass,
    GitMerge,
    Star,
    AlertCircle,
    CheckCircle2,
    ChevronRight,
    Lock,
    GitPullRequest,
    Activity,
    Globe,
    ExternalLink,
    Users,
} from "lucide-react";
import { TopBar } from "@/components/top-bar";
import { ErrorDisplay } from "@/components/error-display";

interface CurrentUser {
    name: string;
    username: string;
    email: string;
    isAdmin: boolean;
}

const repositories = [
    {
        id: 1,
        name: "gitarena",
        org: "mellowagain",
        description: "A lightweight git hosting solution",
        language: "Rust",
        languageColor: "#dea584",
        stars: 128,
        forks: 12,
        updatedAt: "2h ago",
        isPrivate: false,
    },
    {
        id: 2,
        name: "pastemyst",
        org: "mellowagain",
        description: "Powerful code pasting service",
        language: "D",
        languageColor: "#ba595e",
        stars: 256,
        forks: 34,
        updatedAt: "5h ago",
        isPrivate: false,
    },
    {
        id: 3,
        name: "config-files",
        org: "mellowagain",
        description: "Personal dotfiles and configurations",
        language: "Shell",
        languageColor: "#89e051",
        stars: 8,
        forks: 1,
        updatedAt: "1d ago",
        isPrivate: true,
    },
    {
        id: 4,
        name: "website",
        org: "gitarena",
        description: "GitArena marketing website",
        language: "TypeScript",
        languageColor: "#3178c6",
        stars: 24,
        forks: 5,
        updatedAt: "3d ago",
        isPrivate: false,
    },
    {
        id: 5,
        name: "docs",
        org: "gitarena",
        description: "Official documentation site",
        language: "Markdown",
        languageColor: "#083fa1",
        stars: 11,
        forks: 2,
        updatedAt: "5d ago",
        isPrivate: false,
    },
];

const activityFeed = [
    {
        id: 1,
        type: "push",
        repo: "mellowagain/gitarena",
        repoOrg: "mellowagain",
        repoName: "gitarena",
        branch: "main",
        commits: 3,
        message: "feat: add webhook support",
        time: "2h ago",
        user: "mellowagain",
    },
    {
        id: 2,
        type: "issue_opened",
        repo: "mellowagain/pastemyst",
        repoOrg: "mellowagain",
        repoName: "pastemyst",
        issueNumber: 42,
        title: "Add syntax highlighting for Zig",
        time: "5h ago",
        user: "mellowagain",
    },
    {
        id: 3,
        type: "merge",
        repo: "gitarena/website",
        repoOrg: "gitarena",
        repoName: "website",
        mrNumber: 15,
        title: "Update landing page design",
        time: "1d ago",
        user: "mellowagain",
    },
    {
        id: 4,
        type: "star",
        repo: "mellowagain/gitarena",
        repoOrg: "mellowagain",
        repoName: "gitarena",
        user: "torvalds",
        time: "2d ago",
    },
    {
        id: 5,
        type: "fork",
        repo: "mellowagain/pastemyst",
        repoOrg: "mellowagain",
        repoName: "pastemyst",
        user: "dhh",
        time: "3d ago",
    },
    {
        id: 6,
        type: "push",
        repo: "gitarena/docs",
        repoOrg: "gitarena",
        repoName: "docs",
        branch: "main",
        commits: 1,
        message: "docs: update API reference",
        time: "4d ago",
        user: "mellowagain",
    },
];

const assignedIssues = [
    {
        id: 1,
        number: 42,
        title: "Fix authentication flow for OAuth providers",
        repo: "mellowagain/gitarena",
        repoOrg: "mellowagain",
        repoName: "gitarena",
        labels: [
            { name: "bug", color: "#f87171" },
            { name: "priority::high", color: "#fbbf24" },
        ],
        updatedAt: "3h ago",
    },
    {
        id: 2,
        number: 15,
        title: "Add dark mode support for email templates",
        repo: "gitarena/website",
        repoOrg: "gitarena",
        repoName: "website",
        labels: [{ name: "enhancement", color: "#a2eeef" }],
        updatedAt: "1d ago",
    },
    {
        id: 3,
        number: 8,
        title: "Implement rate limiting for API endpoints",
        repo: "mellowagain/pastemyst",
        repoOrg: "mellowagain",
        repoName: "pastemyst",
        labels: [{ name: "component::security", color: "#ef4444" }],
        updatedAt: "2d ago",
    },
];

const pendingMergeRequests = [
    {
        id: 1,
        number: 23,
        title: "Add webhook notification system",
        repo: "mellowagain/gitarena",
        repoOrg: "mellowagain",
        repoName: "gitarena",
        status: "review_required" as const,
        updatedAt: "1h ago",
    },
    {
        id: 2,
        number: 7,
        title: "Refactor database connection pooling",
        repo: "mellowagain/pastemyst",
        repoOrg: "mellowagain",
        repoName: "pastemyst",
        status: "approved" as const,
        updatedAt: "4h ago",
    },
    {
        id: 3,
        number: 31,
        title: "Migrate CI pipeline to new runner",
        repo: "gitarena/website",
        repoOrg: "gitarena",
        repoName: "website",
        status: "draft" as const,
        updatedAt: "2d ago",
    },
];

function LabelBadge({ label }: { label: { name: string; color: string } }) {
    const idx = label.name.indexOf("::");
    if (idx !== -1) {
        const key = label.name.slice(0, idx);
        const val = label.name.slice(idx + 2);
        return (
            <span className="inline-flex items-center text-[10px] rounded overflow-hidden shrink-0">
                <span className="px-1.5 py-0.5 font-medium" style={{ backgroundColor: `${label.color}35`, color: label.color }}>
                    {key}
                </span>
                <span className="px-1.5 py-0.5" style={{ backgroundColor: `${label.color}20`, color: label.color }}>
                    {val}
                </span>
            </span>
        );
    }
    return (
        <span className="px-1.5 py-0.5 text-[10px] rounded shrink-0" style={{ backgroundColor: `${label.color}20`, color: label.color }}>
            {label.name}
        </span>
    );
}

function ActivityRow({ item, currentUser }: { item: (typeof activityFeed)[0]; currentUser: CurrentUser }) {
    const borderColor =
        item.type === "merge"
            ? "border-purple-500/50"
            : item.type === "issue_opened"
              ? "border-green-500/50"
              : item.type === "push"
                ? "border-blue-500/50"
                : item.type === "star"
                  ? "border-yellow-500/50"
                  : "border-border";

    return (
        <div className={`pl-4 border-l-2 ${borderColor} py-0.5`}>
            <div className="flex items-center gap-2 text-xs text-muted-foreground">
                <span className="font-medium text-foreground">{item.user ?? currentUser.username}</span>
                <span className="opacity-40">·</span>
                <Link href={`/${item.repoOrg}/${item.repoName}`} className="font-mono hover:text-foreground transition-colors">
                    {item.repo}
                </Link>
                <span className="ml-auto tabular-nums opacity-60">{item.time}</span>
            </div>
            <p className="text-sm text-muted-foreground mt-0.5">
                {item.type === "push" && (
                    <>
                        Pushed{" "}
                        <span className="text-foreground">
                            {item.commits} {item.commits === 1 ? "commit" : "commits"}
                        </span>{" "}
                        to <span className="font-mono text-xs">{item.branch}</span> —{" "}
                        <span className="opacity-70 text-xs">{item.message}</span>
                    </>
                )}
                {item.type === "issue_opened" && (
                    <>
                        Opened issue{" "}
                        <Link href="#" className="text-foreground hover:underline">
                            #{item.issueNumber}
                        </Link>{" "}
                        — <span className="opacity-70 text-xs">{item.title}</span>
                    </>
                )}
                {item.type === "merge" && (
                    <>
                        Merged{" "}
                        <Link href="#" className="text-foreground hover:underline">
                            !{item.mrNumber}
                        </Link>{" "}
                        — <span className="opacity-70 text-xs">{item.title}</span>
                    </>
                )}
                {item.type === "star" && (
                    <>
                        <span className="text-foreground">{item.user}</span> starred this repository
                    </>
                )}
                {item.type === "fork" && (
                    <>
                        <span className="text-foreground">{item.user}</span> forked this repository
                    </>
                )}
            </p>
        </div>
    );
}

export default function DashboardPage() {
    const router = useRouter();
    const {
        data: currentUser,
        error,
        isLoading,
    } = useSWR<CurrentUser>("/api/users/me", (url: string) => {
        return fetch(url).then((res) => {
            if (res.status === 401) {
                router.push("/about");
                return null;
            }

            if (!res.ok) {
                throw new Error(`${res.status} ${res.statusText}`);
            }

            return res.json();
        });
    });

    const [repoFilter, setRepoFilter] = useState<"all" | "owned" | "starred">("all");

    const filteredRepos = repositories.filter((r) => {
        if (repoFilter === "owned") {
            return r.org === currentUser?.username;
        }
        return true;
    });

    if (isLoading) {
        return <DashboardSkeleton />;
    }

    if (error) {
        return <ErrorDisplay failed="Dashboard" error={error} />;
    }

    if (!currentUser) {
        // user will get redirected to /about
        return null;
    }

    return (
        <div className="min-h-screen bg-background text-foreground flex flex-col">
            <TopBar
                search={{ placeholder: "Search repositories, users, issues..." }}
                navLinks={[
                    { label: "Explore", href: "/explore", icon: <Compass className="h-[18px] w-[18px]" /> },
                    { label: "Merge Requests", href: "#", icon: <GitMerge className="h-[18px] w-[18px]" /> },
                ]}
                hasNotifications
            />

            <div className="flex flex-1 min-h-0">
                <aside className="w-64 shrink-0 border-r border-border flex flex-col overflow-hidden">
                    <div className="flex items-center justify-between px-4 py-2.5 border-b border-border shrink-0">
                        <span className="text-xs font-medium uppercase tracking-widest text-muted-foreground">Repositories</span>
                        <Link href="/new" className="h-5 w-5 flex items-center justify-center rounded hover:bg-accent transition-colors">
                            <Plus className="h-3.5 w-3.5 text-muted-foreground" />
                        </Link>
                    </div>

                    <div className="flex items-center border-b border-border shrink-0">
                        {(["all", "owned", "starred"] as const).map((f) => (
                            <button
                                key={f}
                                onClick={() => setRepoFilter(f)}
                                className={`flex-1 py-2 text-xs border-b-2 -mb-px transition-colors capitalize ${
                                    repoFilter === f
                                        ? "border-foreground text-foreground"
                                        : "border-transparent text-muted-foreground hover:text-foreground"
                                }`}
                            >
                                {f}
                            </button>
                        ))}
                    </div>

                    <div className="flex-1 overflow-y-auto">
                        {filteredRepos.map((repo) => (
                            <Link
                                key={repo.id}
                                href={`/${repo.org}/${repo.name}`}
                                className="group flex items-start gap-3 px-4 py-3 border-b border-border hover:bg-accent/40 transition-colors"
                            >
                                <div className="mt-0.5 shrink-0">
                                    {repo.isPrivate ? (
                                        <Lock className="h-3.5 w-3.5 text-muted-foreground" />
                                    ) : (
                                        <Globe className="h-3.5 w-3.5 text-muted-foreground" />
                                    )}
                                </div>
                                <div className="flex-1 min-w-0">
                                    <div className="text-sm font-medium truncate">
                                        <span className="text-muted-foreground font-normal">{repo.org}/</span>
                                        {repo.name}
                                    </div>
                                    {repo.description && (
                                        <p className="text-xs text-muted-foreground truncate mt-0.5">{repo.description}</p>
                                    )}
                                    <div className="flex items-center gap-3 mt-1.5 text-xs text-muted-foreground">
                                        <span className="flex items-center gap-1">
                                            <span
                                                className="w-2 h-2 rounded-full shrink-0"
                                                style={{ backgroundColor: repo.languageColor }}
                                            />
                                            {repo.language}
                                        </span>
                                        <span className="flex items-center gap-1">
                                            <Star className="h-3 w-3" />
                                            {repo.stars}
                                        </span>
                                        <span className="ml-auto">{repo.updatedAt}</span>
                                    </div>
                                </div>
                            </Link>
                        ))}
                    </div>
                </aside>

                <main className="flex-1 min-w-0 overflow-y-auto">
                    <div className="flex items-center divide-x divide-border border-b border-border text-sm">
                        <div className="flex-1 flex items-center justify-center gap-1.5 py-2.5">
                            <span className="font-semibold tabular-nums">{repositories.length}</span>
                            <span className="text-muted-foreground">repositories</span>
                        </div>
                        <div className="flex-1 flex items-center justify-center gap-1.5 py-2.5">
                            <span className="font-semibold tabular-nums">{assignedIssues.length}</span>
                            <span className="text-muted-foreground">open issues</span>
                        </div>
                        <div className="flex-1 flex items-center justify-center gap-1.5 py-2.5">
                            <span className="font-semibold tabular-nums">{pendingMergeRequests.length}</span>
                            <span className="text-muted-foreground">pending MRs</span>
                        </div>
                        <div className="flex-1 flex items-center justify-center gap-1.5 py-2.5">
                            <span className="font-semibold tabular-nums">3</span>
                            <span className="text-muted-foreground">organizations</span>
                        </div>
                    </div>

                    <div className="grid grid-cols-2 divide-x divide-border border-b border-border">
                        <div>
                            <div className="flex items-center justify-between px-4 py-2.5 border-b border-border">
                                <span className="text-xs font-medium uppercase tracking-widest text-muted-foreground flex items-center gap-2">
                                    <AlertCircle className="h-3.5 w-3.5" />
                                    Assigned to you
                                </span>
                                <Link
                                    href="#"
                                    className="text-xs text-muted-foreground hover:text-foreground transition-colors flex items-center gap-1"
                                >
                                    View all <ChevronRight className="h-3 w-3" />
                                </Link>
                            </div>
                            {assignedIssues.map((issue) => (
                                <Link
                                    key={issue.id}
                                    href={`/${issue.repoOrg}/${issue.repoName}/issues/${issue.number}`}
                                    className="group flex items-start gap-3 px-4 py-3 border-b border-border last:border-0 hover:bg-accent/40 transition-colors"
                                >
                                    <AlertCircle className="h-3.5 w-3.5 text-green-500 mt-0.5 shrink-0" />
                                    <div className="flex-1 min-w-0">
                                        <div className="text-sm truncate">{issue.title}</div>
                                        <div className="flex items-center gap-2 mt-1 flex-wrap">
                                            <span className="text-xs text-muted-foreground font-mono">
                                                {issue.repo}#{issue.number}
                                            </span>
                                            {issue.labels.map((l) => (
                                                <LabelBadge key={l.name} label={l} />
                                            ))}
                                        </div>
                                    </div>
                                    <span className="text-xs text-muted-foreground/60 shrink-0 tabular-nums">{issue.updatedAt}</span>
                                </Link>
                            ))}
                        </div>

                        <div>
                            <div className="flex items-center justify-between px-4 py-2.5 border-b border-border">
                                <span className="text-xs font-medium uppercase tracking-widest text-muted-foreground flex items-center gap-2">
                                    <GitPullRequest className="h-3.5 w-3.5" />
                                    Merge Requests
                                </span>
                                <Link
                                    href="#"
                                    className="text-xs text-muted-foreground hover:text-foreground transition-colors flex items-center gap-1"
                                >
                                    View all <ChevronRight className="h-3 w-3" />
                                </Link>
                            </div>
                            {pendingMergeRequests.map((mr) => (
                                <Link
                                    key={mr.id}
                                    href={`/${mr.repoOrg}/${mr.repoName}/merge-requests/${mr.number}`}
                                    className="group flex items-start gap-3 px-4 py-3 border-b border-border last:border-0 hover:bg-accent/40 transition-colors"
                                >
                                    {mr.status === "approved" ? (
                                        <CheckCircle2 className="h-3.5 w-3.5 text-green-500 mt-0.5 shrink-0" />
                                    ) : mr.status === "draft" ? (
                                        <GitPullRequest className="h-3.5 w-3.5 text-muted-foreground mt-0.5 shrink-0" />
                                    ) : (
                                        <GitMerge className="h-3.5 w-3.5 text-yellow-500 mt-0.5 shrink-0" />
                                    )}
                                    <div className="flex-1 min-w-0">
                                        <div className="text-sm truncate">{mr.title}</div>
                                        <div className="flex items-center gap-2 mt-1">
                                            <span className="text-xs text-muted-foreground font-mono">
                                                {mr.repo}!{mr.number}
                                            </span>
                                            <span
                                                className={`px-1.5 py-0.5 text-[10px] rounded ${
                                                    mr.status === "approved"
                                                        ? "bg-green-500/10 text-green-500"
                                                        : mr.status === "draft"
                                                          ? "bg-secondary text-muted-foreground"
                                                          : "bg-yellow-500/10 text-yellow-500"
                                                }`}
                                            >
                                                {mr.status === "approved"
                                                    ? "Approved"
                                                    : mr.status === "draft"
                                                      ? "Draft"
                                                      : "Review required"}
                                            </span>
                                        </div>
                                    </div>
                                    <span className="text-xs text-muted-foreground/60 shrink-0 tabular-nums">{mr.updatedAt}</span>
                                </Link>
                            ))}
                        </div>
                    </div>

                    <div>
                        <div className="flex items-center justify-between px-4 py-2.5 border-b border-border">
                            <span className="text-xs font-medium uppercase tracking-widest text-muted-foreground flex items-center gap-2">
                                <Activity className="h-3.5 w-3.5" />
                                Recent Activity
                            </span>
                        </div>
                        <div className="px-4 py-4 space-y-4">
                            {activityFeed.map((item) => (
                                <ActivityRow key={item.id} item={item} currentUser={currentUser} />
                            ))}
                        </div>
                    </div>
                </main>

                <aside className="w-56 shrink-0 border-l border-border overflow-y-auto p-4">
                    <div className="mb-4">
                        <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-2">Quick Actions</h3>
                        <div className="space-y-0.5 -mx-2">
                            <Link
                                href="/new"
                                className="flex items-center gap-2 px-2 py-1.5 text-sm text-muted-foreground hover:text-foreground hover:bg-accent/50 rounded-md transition-colors"
                            >
                                <Plus className="h-3.5 w-3.5 shrink-0" />
                                New repository
                            </Link>
                            <Link
                                href="/import"
                                className="flex items-center gap-2 px-2 py-1.5 text-sm text-muted-foreground hover:text-foreground hover:bg-accent/50 rounded-md transition-colors"
                            >
                                <ExternalLink className="h-3.5 w-3.5 shrink-0" />
                                Import repository
                            </Link>
                            <button className="w-full flex items-center gap-2 px-2 py-1.5 text-sm text-muted-foreground hover:text-foreground hover:bg-accent/50 rounded-md transition-colors">
                                <Users className="h-3.5 w-3.5 shrink-0" />
                                New organization
                            </button>
                        </div>
                    </div>

                    <div className="pt-4 border-t border-border mb-4">
                        <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-2">Explore</h3>
                        <div className="space-y-0.5 -mx-2">
                            <Link
                                href="/explore"
                                className="flex items-center justify-between px-2 py-1.5 text-sm text-muted-foreground hover:text-foreground hover:bg-accent/50 rounded-md transition-colors"
                            >
                                <span>Trending repos</span>
                                <ChevronRight className="h-3.5 w-3.5" />
                            </Link>
                            <Link
                                href="#"
                                className="flex items-center justify-between px-2 py-1.5 text-sm text-muted-foreground hover:text-foreground hover:bg-accent/50 rounded-md transition-colors"
                            >
                                <span>Topics</span>
                                <ChevronRight className="h-3.5 w-3.5" />
                            </Link>
                            <Link
                                href="#"
                                className="flex items-center justify-between px-2 py-1.5 text-sm text-muted-foreground hover:text-foreground hover:bg-accent/50 rounded-md transition-colors"
                            >
                                <span>Collections</span>
                                <ChevronRight className="h-3.5 w-3.5" />
                            </Link>
                        </div>
                    </div>

                    <div className="pt-4 border-t border-border">
                        <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-2">Resources</h3>
                        <div className="space-y-0.5 -mx-2">
                            <Link
                                href="/about"
                                className="flex items-center justify-between px-2 py-1.5 text-sm text-muted-foreground hover:text-foreground hover:bg-accent/50 rounded-md transition-colors"
                            >
                                <span>About GitArena</span>
                                <ChevronRight className="h-3.5 w-3.5" />
                            </Link>
                            <Link
                                href="#"
                                className="flex items-center justify-between px-2 py-1.5 text-sm text-muted-foreground hover:text-foreground hover:bg-accent/50 rounded-md transition-colors"
                            >
                                <span>Documentation</span>
                                <ChevronRight className="h-3.5 w-3.5" />
                            </Link>
                            <Link
                                href="#"
                                className="flex items-center justify-between px-2 py-1.5 text-sm text-muted-foreground hover:text-foreground hover:bg-accent/50 rounded-md transition-colors"
                            >
                                <span>API Reference</span>
                                <ChevronRight className="h-3.5 w-3.5" />
                            </Link>
                        </div>
                    </div>
                </aside>
            </div>
        </div>
    );
}

export function DashboardSkeleton() {
    return (
        <div className="min-h-screen bg-background text-foreground flex flex-col">
            <TopBar
                search={{ placeholder: "Search repositories, users, issues..." }}
                navLinks={[
                    { label: "Explore", href: "/explore", icon: <Compass className="h-[18px] w-[18px]" /> },
                    { label: "Merge Requests", href: "#", icon: <GitMerge className="h-[18px] w-[18px]" /> },
                ]}
                hasNotifications
            />
            <div className="flex flex-1 min-h-0 animate-pulse">
                {/* Left sidebar */}
                <aside className="w-64 shrink-0 border-r border-border flex flex-col">
                    <div className="flex items-center justify-between px-4 py-2.5 border-b border-border">
                        <div className="h-3 w-24 rounded bg-accent" />
                    </div>
                    <div className="flex border-b border-border">
                        {[1, 2, 3].map((i) => (
                            <div key={i} className="flex-1 py-2 mx-3 my-2 h-3 rounded bg-accent" />
                        ))}
                    </div>
                    <div className="flex-1 divide-y divide-border">
                        {[1, 2, 3, 4, 5].map((i) => (
                            <div key={i} className="flex items-start gap-3 px-4 py-3">
                                <div className="h-3.5 w-3.5 rounded bg-accent shrink-0 mt-0.5" />
                                <div className="flex-1 space-y-2">
                                    <div className="h-3 w-3/4 rounded bg-accent" />
                                    <div className="h-2.5 w-full rounded bg-accent" />
                                    <div className="h-2.5 w-1/2 rounded bg-accent" />
                                </div>
                            </div>
                        ))}
                    </div>
                </aside>

                {/* Main content */}
                <main className="flex-1 min-w-0">
                    <div className="flex divide-x divide-border border-b border-border">
                        {[1, 2, 3, 4].map((i) => (
                            <div key={i} className="flex-1 flex flex-col items-center gap-1.5 py-2.5">
                                <div className="h-4 w-8 rounded bg-accent" />
                                <div className="h-3 w-20 rounded bg-accent" />
                            </div>
                        ))}
                    </div>
                    <div className="grid grid-cols-2 divide-x divide-border border-b border-border">
                        {[1, 2].map((col) => (
                            <div key={col}>
                                <div className="flex items-center justify-between px-4 py-2.5 border-b border-border">
                                    <div className="h-3 w-32 rounded bg-accent" />
                                </div>
                                {[1, 2, 3].map((i) => (
                                    <div key={i} className="flex items-start gap-3 px-4 py-3 border-b border-border last:border-0">
                                        <div className="h-3.5 w-3.5 rounded-full bg-accent shrink-0 mt-0.5" />
                                        <div className="flex-1 space-y-2">
                                            <div className="h-3 w-5/6 rounded bg-accent" />
                                            <div className="h-2.5 w-1/3 rounded bg-accent" />
                                        </div>
                                        <div className="h-2.5 w-10 rounded bg-accent shrink-0" />
                                    </div>
                                ))}
                            </div>
                        ))}
                    </div>
                    <div className="px-4 py-4 space-y-4">
                        {[1, 2, 3, 4].map((i) => (
                            <div key={i} className="pl-4 border-l-2 border-border py-0.5 space-y-1.5">
                                <div className="h-3 w-2/5 rounded bg-accent" />
                                <div className="h-3 w-3/4 rounded bg-accent" />
                            </div>
                        ))}
                    </div>
                </main>

                {/* Right sidebar */}
                <aside className="w-56 shrink-0 border-l border-border p-4 space-y-4">
                    <div className="h-3 w-24 rounded bg-accent" />
                    {[1, 2, 3].map((i) => (
                        <div key={i} className="h-7 rounded bg-accent" />
                    ))}
                    <div className="pt-4 border-t border-border space-y-3">
                        <div className="h-3 w-16 rounded bg-accent" />
                        {[1, 2, 3].map((i) => (
                            <div key={i} className="h-7 rounded bg-accent" />
                        ))}
                    </div>
                    <div className="pt-4 border-t border-border space-y-3">
                        <div className="h-3 w-20 rounded bg-accent" />
                        {[1, 2, 3].map((i) => (
                            <div key={i} className="h-7 rounded bg-accent" />
                        ))}
                    </div>
                </aside>
            </div>
        </div>
    );
}

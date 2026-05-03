"use client";

import { useState, useEffect } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";
import useSWR from "swr";
import {
    Plus,
    Compass,
    GitMerge,
    Star,
    AlertCircle,
    ChevronRight,
    Lock,
    GitPullRequest,
    Activity,
    Globe,
    ExternalLink,
    BookOpen,
    FileCode2,
    Construction,
} from "lucide-react";
import { TopBar } from "@/components/top-bar";
import { ErrorDisplay } from "@/components/error-display";
import { useAuth } from "@/hooks/use-auth";

import * as allLangs from "linguist-languages";

function languageColor(name: string): string {
    const color = (allLangs as Record<string, { color?: string }>)[name]?.color;
    if (color) {
        return color;
    }
    let hash = 0;
    for (let i = 0; i < name.length; i++) {
        hash = name.charCodeAt(i) + ((hash << 5) - hash);
    }
    return `hsl(${Math.abs(hash) % 360}, 60%, 55%)`;
}

interface UserProfileRepo {
    id: number;
    name: string;
    description: string;
    visibility: "public" | "internal" | "private";
    archived: boolean;
    languages: Record<string, number>;
    stars: number;
}

interface UserProfileResponse {
    id: number;
    username: string;
    admin: boolean;
    createdAt: string;
    repos: UserProfileRepo[];
    stats: {
        repos: number;
        starsEarned: number;
        starsGiven: number;
    };
}

/** Returns the primary language (most bytes) from the languages map. */
function getPrimaryLanguage(languages: Record<string, number>): { name: string; color: string } | null {
    let best: string | null = null;
    let bestBytes = 0;
    for (const [lang, bytes] of Object.entries(languages)) {
        if (bytes > bestBytes) {
            best = lang;
            bestBytes = bytes;
        }
    }
    if (!best) {
        return null;
    }
    return { name: best, color: languageColor(best) };
}

function WipBadge() {
    return (
        <span className="inline-flex items-center gap-1 px-2 py-0.5 text-[10px] font-medium rounded bg-yellow-500/10 text-yellow-600 dark:text-yellow-400">
            <Construction className="h-3 w-3" />
            WIP
        </span>
    );
}

export default function DashboardPage() {
    const router = useRouter();
    const { user, error, isLoading } = useAuth();

    useEffect(() => {
        if (!isLoading && !user) {
            router.push("/about");
        }
    }, [isLoading, user, router]);

    const [repoFilter, setRepoFilter] = useState<"all" | "owned" | "starred">("all");

    const { data: profile, isLoading: profileLoading } = useSWR<UserProfileResponse>(user ? `/api/users/${user.username}` : null);

    const repos = profile?.repos ?? [];
    const filteredRepos = repos.filter(() => {
        // "starred" filter not yet supported by API — show all for now
        if (repoFilter === "starred") {
            return true;
        }
        // "owned" and "all" both show all repos from the user profile endpoint
        return true;
    });

    if (isLoading) {
        return <DashboardSkeleton />;
    }

    if (error) {
        return (
            <>
                <TopBar />
                <ErrorDisplay failed="Dashboard" error={error} />
            </>
        );
    }

    if (!user) {
        // user will get redirected to /about via useEffect
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

            <div className="flex flex-col lg:flex-row flex-1 min-h-0">
                <main className="flex-1 overflow-y-auto">
                    <div className="max-w-4xl mx-auto px-4 py-6 sm:px-6 sm:py-8">
                        {/* Welcome header */}
                        <div className="mb-6 sm:mb-8">
                            <h1 className="text-xl sm:text-2xl font-semibold">
                                Welcome back, <span className="text-muted-foreground">{user.username}</span>
                            </h1>
                            <p className="text-sm text-muted-foreground mt-1">
                                Here&apos;s what&apos;s happening across your repositories.
                            </p>
                        </div>

                        {/* Repositories */}
                        <section className="mb-8">
                            <div className="flex items-center justify-between mb-3">
                                <h2 className="text-xs font-medium uppercase tracking-widest text-muted-foreground">Repositories</h2>
                                <div className="flex items-center gap-3">
                                    <div className="flex items-center gap-0.5">
                                        {(["all", "owned", "starred"] as const).map((f) => (
                                            <button
                                                key={f}
                                                onClick={() => setRepoFilter(f)}
                                                className={`px-2.5 py-1 text-xs rounded-md transition-colors capitalize ${
                                                    repoFilter === f
                                                        ? "bg-secondary text-foreground"
                                                        : "text-muted-foreground hover:text-foreground"
                                                }`}
                                            >
                                                {f}
                                            </button>
                                        ))}
                                    </div>
                                    <Link
                                        href="/new"
                                        className="h-6 w-6 flex items-center justify-center rounded-md hover:bg-accent transition-colors"
                                    >
                                        <Plus className="h-3.5 w-3.5 text-muted-foreground" />
                                    </Link>
                                </div>
                            </div>
                            <div className="border border-border rounded-md divide-y divide-border">
                                {profileLoading ? (
                                    [1, 2, 3].map((i) => (
                                        <div key={i} className="flex items-center gap-3 px-4 py-3 animate-pulse">
                                            <div className="h-4 w-4 rounded bg-accent shrink-0" />
                                            <div className="flex-1 space-y-1.5">
                                                <div className="h-3.5 w-48 rounded bg-accent" />
                                                <div className="h-3 w-64 rounded bg-accent" />
                                            </div>
                                        </div>
                                    ))
                                ) : filteredRepos.length === 0 ? (
                                    <div className="px-4 py-8 text-center text-sm text-muted-foreground">
                                        No repositories yet.{" "}
                                        <Link href="/new" className="text-foreground hover:underline">
                                            Create one
                                        </Link>
                                    </div>
                                ) : (
                                    filteredRepos.map((repo) => {
                                        const primaryLang = getPrimaryLanguage(repo.languages);
                                        return (
                                            <Link
                                                key={repo.id}
                                                href={`/${user.username}/${repo.name}`}
                                                className="flex flex-col gap-1.5 sm:flex-row sm:items-center sm:gap-3 px-4 py-3 hover:bg-accent transition-colors first:rounded-t-md last:rounded-b-md"
                                            >
                                                <div className="flex items-center gap-3 min-w-0">
                                                    <div className="shrink-0">
                                                        {repo.visibility === "private" ? (
                                                            <Lock className="h-4 w-4 text-muted-foreground" />
                                                        ) : (
                                                            <Globe className="h-4 w-4 text-muted-foreground" />
                                                        )}
                                                    </div>
                                                    <div className="min-w-0">
                                                        <div className="text-sm font-medium">
                                                            <span className="text-muted-foreground font-normal">{user.username}/</span>
                                                            {repo.name}
                                                            {repo.archived && (
                                                                <span className="ml-2 px-1.5 py-0.5 text-[10px] rounded bg-secondary text-muted-foreground">
                                                                    archived
                                                                </span>
                                                            )}
                                                        </div>
                                                        {repo.description && (
                                                            <p className="text-xs text-muted-foreground truncate mt-0.5">
                                                                {repo.description}
                                                            </p>
                                                        )}
                                                    </div>
                                                </div>
                                                <div className="flex items-center gap-3 text-xs text-muted-foreground shrink-0 pl-7 sm:pl-0 sm:ml-auto">
                                                    {primaryLang && (
                                                        <span className="flex items-center gap-1">
                                                            <span
                                                                className="w-2 h-2 rounded-full shrink-0"
                                                                style={{ backgroundColor: primaryLang.color }}
                                                            />
                                                            {primaryLang.name}
                                                        </span>
                                                    )}
                                                    <span className="flex items-center gap-1">
                                                        <Star className="h-3 w-3" />
                                                        {repo.stars}
                                                    </span>
                                                </div>
                                            </Link>
                                        );
                                    })
                                )}
                            </div>
                        </section>

                        {/* Assigned Issues */}
                        <section className="mb-8">
                            <div className="flex items-center justify-between mb-3">
                                <h2 className="text-xs font-medium uppercase tracking-widest text-muted-foreground flex items-center gap-2">
                                    <AlertCircle className="h-3.5 w-3.5" />
                                    Assigned to you
                                    <WipBadge />
                                </h2>
                            </div>
                            <div className="border border-border rounded-md px-4 py-6 text-center text-sm text-muted-foreground">
                                Assigned issues will appear here once the API is available.
                            </div>
                        </section>

                        {/* Merge Requests */}
                        <section className="mb-8">
                            <div className="flex items-center justify-between mb-3">
                                <h2 className="text-xs font-medium uppercase tracking-widest text-muted-foreground flex items-center gap-2">
                                    <GitPullRequest className="h-3.5 w-3.5" />
                                    Merge Requests
                                    <WipBadge />
                                </h2>
                            </div>
                            <div className="border border-border rounded-md px-4 py-6 text-center text-sm text-muted-foreground">
                                Merge requests will appear here once the API is available.
                            </div>
                        </section>

                        {/* Recent Activity */}
                        <section>
                            <h2 className="text-xs font-medium uppercase tracking-widest text-muted-foreground flex items-center gap-2 mb-3">
                                <Activity className="h-3.5 w-3.5" />
                                Recent Activity
                                <WipBadge />
                            </h2>
                            <div className="border border-border rounded-md px-4 py-6 text-center text-sm text-muted-foreground">
                                Activity feed will appear here once the API is available.
                            </div>
                        </section>
                    </div>
                </main>

                {/* Right sidebar */}
                <aside className="w-full border-t border-border lg:w-72 lg:border-t-0 lg:border-l shrink-0 overflow-y-auto p-4 sm:p-5">
                    <div className="grid grid-cols-2 gap-4 sm:gap-6 lg:grid-cols-1">
                        <div>
                            <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">Quick Actions</h3>
                            <div className="space-y-1">
                                <Link
                                    href="/new"
                                    className="flex items-center gap-2.5 px-3 py-2 text-sm border border-border rounded-md hover:bg-accent transition-colors"
                                >
                                    <Plus className="h-4 w-4 text-muted-foreground shrink-0" />
                                    New repository
                                </Link>
                                <Link
                                    href="/new/import"
                                    className="flex items-center gap-2.5 px-3 py-2 text-sm border border-border rounded-md hover:bg-accent transition-colors"
                                >
                                    <ExternalLink className="h-4 w-4 text-muted-foreground shrink-0" />
                                    Import repository
                                </Link>
                            </div>
                        </div>

                        <div>
                            <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">Explore</h3>
                            <div className="space-y-1">
                                <Link
                                    href="/explore"
                                    className="flex items-center justify-between px-3 py-2 text-sm border border-border rounded-md hover:bg-accent transition-colors"
                                >
                                    <span className="flex items-center gap-2.5">
                                        <Compass className="h-4 w-4 text-muted-foreground shrink-0" />
                                        Trending repos
                                    </span>
                                    <ChevronRight className="h-3.5 w-3.5 text-muted-foreground" />
                                </Link>
                                <Link
                                    href="#"
                                    className="flex items-center justify-between px-3 py-2 text-sm border border-border rounded-md hover:bg-accent transition-colors"
                                >
                                    <span className="flex items-center gap-2.5">
                                        <BookOpen className="h-4 w-4 text-muted-foreground shrink-0" />
                                        Topics
                                    </span>
                                    <ChevronRight className="h-3.5 w-3.5 text-muted-foreground" />
                                </Link>
                                <Link
                                    href="#"
                                    className="flex items-center justify-between px-3 py-2 text-sm border border-border rounded-md hover:bg-accent transition-colors"
                                >
                                    <span className="flex items-center gap-2.5">
                                        <FileCode2 className="h-4 w-4 text-muted-foreground shrink-0" />
                                        Collections
                                    </span>
                                    <ChevronRight className="h-3.5 w-3.5 text-muted-foreground" />
                                </Link>
                            </div>
                        </div>

                        <div>
                            <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">Resources</h3>
                            <div className="space-y-1">
                                <Link
                                    href="/about"
                                    className="flex items-center justify-between px-3 py-2 text-sm border border-border rounded-md hover:bg-accent transition-colors"
                                >
                                    <span>About GitArena</span>
                                    <ChevronRight className="h-3.5 w-3.5 text-muted-foreground" />
                                </Link>
                                <a
                                    href="/docs"
                                    className="flex items-center justify-between px-3 py-2 text-sm border border-border rounded-md hover:bg-accent transition-colors"
                                >
                                    <span>Documentation</span>
                                    <ChevronRight className="h-3.5 w-3.5 text-muted-foreground" />
                                </a>
                                <a
                                    href="/docs/api-reference/introduction"
                                    className="flex items-center justify-between px-3 py-2 text-sm border border-border rounded-md hover:bg-accent transition-colors"
                                >
                                    <span>API Reference</span>
                                    <ChevronRight className="h-3.5 w-3.5 text-muted-foreground" />
                                </a>
                            </div>
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
            <div className="flex flex-col lg:flex-row flex-1 min-h-0 animate-pulse">
                {/* Main content */}
                <main className="flex-1 overflow-y-auto">
                    <div className="max-w-4xl mx-auto px-4 py-6 sm:px-6 sm:py-8">
                        {/* Welcome header skeleton */}
                        <div className="mb-6 sm:mb-8 space-y-2">
                            <div className="h-7 w-72 rounded bg-accent" />
                            <div className="h-4 w-56 rounded bg-accent" />
                        </div>

                        {/* Repos section skeleton */}
                        <div className="mb-8">
                            <div className="flex items-center justify-between mb-3">
                                <div className="h-3 w-24 rounded bg-accent" />
                                <div className="flex items-center gap-1">
                                    {[1, 2, 3].map((i) => (
                                        <div key={i} className="h-6 w-12 rounded-md bg-accent" />
                                    ))}
                                </div>
                            </div>
                            <div className="border border-border rounded-md divide-y divide-border">
                                {[1, 2, 3, 4, 5].map((i) => (
                                    <div key={i} className="flex items-center gap-3 px-4 py-3">
                                        <div className="h-4 w-4 rounded bg-accent shrink-0" />
                                        <div className="flex-1 space-y-1.5">
                                            <div className="h-3.5 w-48 rounded bg-accent" />
                                            <div className="h-3 w-64 rounded bg-accent" />
                                        </div>
                                        <div className="flex items-center gap-3">
                                            <div className="h-3 w-12 rounded bg-accent" />
                                            <div className="h-3 w-8 rounded bg-accent" />
                                            <div className="h-3 w-10 rounded bg-accent" />
                                        </div>
                                    </div>
                                ))}
                            </div>
                        </div>

                        {/* Issues skeleton */}
                        <div className="mb-8">
                            <div className="h-3 w-28 rounded bg-accent mb-3" />
                            <div className="h-16 rounded-md bg-accent" />
                        </div>

                        {/* MRs skeleton */}
                        <div className="mb-8">
                            <div className="h-3 w-28 rounded bg-accent mb-3" />
                            <div className="h-16 rounded-md bg-accent" />
                        </div>

                        {/* Activity skeleton */}
                        <div>
                            <div className="h-3 w-28 rounded bg-accent mb-3" />
                            <div className="h-16 rounded-md bg-accent" />
                        </div>
                    </div>
                </main>

                {/* Right sidebar skeleton */}
                <aside className="w-full border-t border-border lg:w-72 lg:border-t-0 lg:border-l shrink-0 p-4 sm:p-5">
                    <div className="grid grid-cols-2 gap-4 sm:gap-6 lg:grid-cols-1">
                        {[1, 2, 3].map((section) => (
                            <div key={section}>
                                <div className="h-3 w-24 rounded bg-accent mb-3" />
                                <div className="space-y-1">
                                    {[1, 2, 3].map((i) => (
                                        <div key={i} className="h-9 rounded-md bg-accent" />
                                    ))}
                                </div>
                            </div>
                        ))}
                    </div>
                </aside>
            </div>
        </div>
    );
}

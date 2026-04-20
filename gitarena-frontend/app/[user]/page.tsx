"use client";

import { useState } from "react";
import Link from "next/link";
import { useParams } from "next/navigation";
import {
    Star,
    Lock,
    MapPin,
    Calendar,
    Users,
    Package,
    AlertCircle,
    GitMerge,
    GitCommit,
    Pin,
    PinOff,
    ShieldCheck,
    Settings,
    ExternalLink,
    Plus,
} from "lucide-react";
import { TopBar } from "@/components/top-bar";

const profileData = {
    username: "mellowagain",
    displayName: "Mari",
    bio: "Building open-source tools for developers. Creator of GitArena and pastemyst.",
    avatarUrl: null as string | null,
    location: "Vienna, Austria",
    website: "https://mariari.dev",
    joinedAt: "January 2021",
    isCurrentUser: true,
    stats: {
        repos: 24,
        followers: 312,
        following: 48,
        stars: 1_843,
    },
    orgs: [
        { name: "gitarena", avatarUrl: null },
        { name: "pastemyst", avatarUrl: null },
        { name: "rust-vienna", avatarUrl: null },
    ],
    pinnedRepos: [
        {
            name: "gitarena",
            org: "mellowagain",
            description: "A lightweight, performant, self-hosted Git platform written in Rust.",
            language: "Rust",
            languageColor: "#dea584",
            stars: 128,
            forks: 12,
            isPrivate: false,
            updatedAt: "2 hours ago",
        },
        {
            name: "pastemyst",
            org: "mellowagain",
            description: "Powerful code pasting service with syntax highlighting and expiry.",
            language: "D",
            languageColor: "#ba595e",
            stars: 256,
            forks: 34,
            isPrivate: false,
            updatedAt: "5 hours ago",
        },
        {
            name: "config-files",
            org: "mellowagain",
            description: "Personal dotfiles and configurations.",
            language: "Shell",
            languageColor: "#89e051",
            stars: 8,
            forks: 1,
            isPrivate: true,
            updatedAt: "1 day ago",
        },
        {
            name: "website",
            org: "gitarena",
            description: "GitArena marketing website and documentation.",
            language: "TypeScript",
            languageColor: "#3178c6",
            stars: 24,
            forks: 5,
            isPrivate: false,
            updatedAt: "3 days ago",
        },
    ],
    allRepos: [
        {
            name: "gitarena",
            org: "mellowagain",
            description: "A lightweight, performant self-hosted Git platform.",
            language: "Rust",
            languageColor: "#dea584",
            stars: 128,
            forks: 12,
            isPrivate: false,
            updatedAt: "2h",
        },
        {
            name: "pastemyst",
            org: "mellowagain",
            description: "Powerful code pasting service.",
            language: "D",
            languageColor: "#ba595e",
            stars: 256,
            forks: 34,
            isPrivate: false,
            updatedAt: "5h",
        },
        {
            name: "config-files",
            org: "mellowagain",
            description: "Personal dotfiles and configurations.",
            language: "Shell",
            languageColor: "#89e051",
            stars: 8,
            forks: 1,
            isPrivate: true,
            updatedAt: "1d",
        },
        {
            name: "website",
            org: "gitarena",
            description: "GitArena marketing website.",
            language: "TypeScript",
            languageColor: "#3178c6",
            stars: 24,
            forks: 5,
            isPrivate: false,
            updatedAt: "3d",
        },
        {
            name: "docs",
            org: "gitarena",
            description: "Official documentation site.",
            language: "Markdown",
            languageColor: "#083fa1",
            stars: 11,
            forks: 2,
            isPrivate: false,
            updatedAt: "1w",
        },
        {
            name: "advent-of-code",
            org: "mellowagain",
            description: "My AoC solutions over the years.",
            language: "Rust",
            languageColor: "#dea584",
            stars: 3,
            forks: 0,
            isPrivate: false,
            updatedAt: "2w",
        },
        {
            name: "raytracer",
            org: "mellowagain",
            description: "Weekend raytracer in Rust.",
            language: "Rust",
            languageColor: "#dea584",
            stars: 17,
            forks: 2,
            isPrivate: false,
            updatedAt: "1mo",
        },
    ],
    recentActivity: [
        {
            type: "push" as const,
            repo: "gitarena",
            org: "mellowagain",
            message: "fix: resolve SSH key parsing edge case",
            branch: "main",
            time: "2 hours ago",
        },
        {
            type: "issue_open" as const,
            repo: "pastemyst",
            org: "mellowagain",
            message: "Add support for expiring pastes via API",
            time: "6 hours ago",
        },
        {
            type: "mr_merged" as const,
            repo: "gitarena",
            org: "mellowagain",
            message: "feat: implement SSH key management UI",
            time: "1 day ago",
        },
        {
            type: "push" as const,
            repo: "website",
            org: "gitarena",
            message: "chore: update dependencies and fix lint errors",
            branch: "main",
            time: "2 days ago",
        },
        {
            type: "mr_open" as const,
            repo: "gitarena",
            org: "mellowagain",
            message: "feat: add webhook support for push events",
            time: "3 days ago",
        },
        {
            type: "push" as const,
            repo: "config-files",
            org: "mellowagain",
            message: "update nvim config",
            branch: "main",
            time: "4 days ago",
        },
        {
            type: "issue_closed" as const,
            repo: "pastemyst",
            org: "mellowagain",
            message: "Syntax highlighting broken for D lang",
            time: "5 days ago",
        },
    ],
};

function ContributionGraph() {
    const weeks = 52;
    const days = 7;
    const levels = [0, 1, 2, 3, 4];

    const data = Array.from({ length: weeks }, () =>
        Array.from({ length: days }, (_, d) => {
            if (d === 0 || d === 6) {
                return 0;
            }
            const rand = Math.random();
            if (rand > 0.7) {
                return Math.floor(Math.random() * 4) + 1;
            }
            return 0;
        })
    );

    const levelClass = (level: number) => {
        if (level === 0) {
            return "bg-secondary";
        }
        if (level === 1) {
            return "bg-foreground/15";
        }
        if (level === 2) {
            return "bg-foreground/35";
        }
        if (level === 3) {
            return "bg-foreground/60";
        }
        return "bg-foreground/85";
    };

    return (
        <div className="w-full">
            <div className="grid gap-[3px]" style={{ gridTemplateColumns: `repeat(${weeks}, 1fr)` }}>
                {data.map((week, wi) => (
                    <div key={wi} className="flex flex-col gap-[3px]">
                        {week.map((level, di) => (
                            <div
                                key={di}
                                title={`${level} contributions`}
                                className={`aspect-square w-full rounded-sm ${levelClass(level)} transition-opacity hover:opacity-70`}
                            />
                        ))}
                    </div>
                ))}
            </div>
            <div className="flex items-center justify-end gap-1.5 mt-2">
                <span className="text-[10px] text-muted-foreground">Less</span>
                {levels.map((l) => (
                    <div key={l} className={`w-[11px] h-[11px] rounded-sm ${levelClass(l)}`} />
                ))}
                <span className="text-[10px] text-muted-foreground">More</span>
            </div>
        </div>
    );
}

function ActivityIcon({ type }: { type: (typeof profileData.recentActivity)[0]["type"] }) {
    if (type === "push") {
        return <GitCommit className="h-3.5 w-3.5 text-muted-foreground" />;
    }
    if (type === "issue_open") {
        return <AlertCircle className="h-3.5 w-3.5 text-green-500" />;
    }
    if (type === "issue_closed") {
        return <AlertCircle className="h-3.5 w-3.5 text-muted-foreground" />;
    }
    if (type === "mr_merged") {
        return <GitMerge className="h-3.5 w-3.5 text-purple-500" />;
    }
    return <GitMerge className="h-3.5 w-3.5 text-green-500" />;
}

type ActivityEvent = (typeof profileData.recentActivity)[number];

function ActivityFeed({ events }: { events: ActivityEvent[] }) {
    return (
        <div className="space-y-4">
            {events.map((event, i) => {
                const borderColor =
                    event.type === "mr_merged"
                        ? "border-purple-500/50 hover:border-purple-500"
                        : event.type === "mr_open"
                          ? "border-green-500/50 hover:border-green-500"
                          : event.type === "issue_open"
                            ? "border-green-500/50 hover:border-green-500"
                            : event.type === "issue_closed"
                              ? "border-muted-foreground/30 hover:border-muted-foreground/50"
                              : "border-border hover:border-muted-foreground/40";

                return (
                    <div key={i} className={`group pl-4 border-l-2 transition-colors ${borderColor}`}>
                        <div className="flex items-center gap-2 mb-1.5">
                            <div className="h-5 w-5 flex items-center justify-center rounded-full bg-secondary border border-border text-[10px] font-medium shrink-0">
                                {profileData.displayName[0].toUpperCase()}
                            </div>
                            <span className="text-sm font-medium">{profileData.displayName}</span>
                            <span className="text-xs text-muted-foreground/50">·</span>
                            <Link
                                href={`/${event.org}/${event.repo}`}
                                className="text-xs text-muted-foreground hover:text-foreground transition-colors font-mono"
                            >
                                {event.org}/{event.repo}
                            </Link>
                            <span className="text-xs text-muted-foreground ml-auto">{event.time}</span>
                        </div>
                        <div className="flex items-start gap-2 text-sm text-muted-foreground">
                            <ActivityIcon type={event.type} />
                            <div className="space-y-0.5">
                                <p>{event.message}</p>
                                {"branch" in event && event.branch && <p className="text-xs font-mono">on {event.branch}</p>}
                            </div>
                        </div>
                    </div>
                );
            })}
        </div>
    );
}

export default function UserProfilePage() {
    const params = useParams();
    const username = (params.org as string) ?? profileData.username;
    const [pinnedKeys, setPinnedKeys] = useState<Set<string>>(new Set(profileData.pinnedRepos.map((r) => `${r.org}/${r.name}`)));

    function togglePin(key: string) {
        setPinnedKeys((prev) => {
            const next = new Set(prev);
            if (next.has(key)) {
                next.delete(key);
            } else {
                next.add(key);
            }
            return next;
        });
    }

    return (
        <div className="min-h-screen bg-background text-foreground flex flex-col">
            <TopBar breadcrumb={[{ label: username }]} hasNotifications />

            <div className="flex flex-1 min-h-0">
                <aside className="w-64 shrink-0 border-r border-border overflow-y-auto p-5">
                    <div className="mb-4">
                        <div className="h-20 w-20 flex items-center justify-center rounded-full bg-secondary border-2 border-border text-3xl font-semibold text-foreground mb-3">
                            {profileData.displayName[0].toUpperCase()}
                        </div>
                        <div className="flex items-center gap-2">
                            <p className="text-base font-semibold leading-tight">{profileData.displayName}</p>
                            {profileData.isCurrentUser && (
                                <span className="inline-flex items-center gap-1 px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wider border border-border rounded text-muted-foreground bg-secondary">
                                    <ShieldCheck className="h-2.5 w-2.5" />
                                    Admin
                                </span>
                            )}
                        </div>
                        <p className="text-sm text-muted-foreground font-mono">{username}</p>
                        {profileData.bio && <p className="text-sm text-muted-foreground leading-relaxed mt-2">{profileData.bio}</p>}
                        {profileData.isCurrentUser && (
                            <Link
                                href="/settings/profile"
                                className="mt-3 w-full flex items-center justify-center gap-1.5 px-3 py-1.5 text-xs border border-border rounded-md hover:bg-accent/50 transition-colors"
                            >
                                <Settings className="h-3 w-3" />
                                Edit profile
                            </Link>
                        )}
                    </div>

                    <div className="pt-4 border-t border-border space-y-2">
                        {profileData.location && (
                            <div className="flex items-center gap-2 text-sm text-muted-foreground">
                                <MapPin className="h-3.5 w-3.5 shrink-0" />
                                {profileData.location}
                            </div>
                        )}
                        {profileData.website && (
                            <a
                                href={profileData.website}
                                target="_blank"
                                rel="noreferrer"
                                className="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
                            >
                                <ExternalLink className="h-3.5 w-3.5 shrink-0" />
                                {profileData.website.replace(/^https?:\/\//, "")}
                            </a>
                        )}
                        <div className="flex items-center gap-2 text-sm text-muted-foreground">
                            <Calendar className="h-3.5 w-3.5 shrink-0" />
                            Joined {profileData.joinedAt}
                        </div>
                    </div>

                    <div className="pt-4 border-t border-border mt-4 space-y-1">
                        <h3 className="text-sm font-medium uppercase tracking-wider text-muted-foreground mb-2">Stats</h3>
                        <Link
                            href="#"
                            className="flex items-center justify-between text-sm text-muted-foreground hover:text-foreground transition-colors -mx-2 px-2 py-1.5 rounded-md hover:bg-accent/50"
                        >
                            <span className="flex items-center gap-2">
                                <Package className="h-3.5 w-3.5" />
                                Repositories
                            </span>
                            <span className="font-mono text-xs text-foreground">{profileData.stats.repos}</span>
                        </Link>
                        <Link
                            href="#"
                            className="flex items-center justify-between text-sm text-muted-foreground hover:text-foreground transition-colors -mx-2 px-2 py-1.5 rounded-md hover:bg-accent/50"
                        >
                            <span className="flex items-center gap-2">
                                <Star className="h-3.5 w-3.5" />
                                Stars earned
                            </span>
                            <span className="font-mono text-xs text-foreground">{profileData.stats.stars.toLocaleString()}</span>
                        </Link>
                        <Link
                            href="#"
                            className="flex items-center justify-between text-sm text-muted-foreground hover:text-foreground transition-colors -mx-2 px-2 py-1.5 rounded-md hover:bg-accent/50"
                        >
                            <span className="flex items-center gap-2">
                                <Star className="h-3.5 w-3.5" />
                                Stars given
                            </span>
                            <span className="font-mono text-xs text-foreground">47</span>
                        </Link>
                        <Link
                            href="#"
                            className="flex items-center justify-between text-sm text-muted-foreground hover:text-foreground transition-colors -mx-2 px-2 py-1.5 rounded-md hover:bg-accent/50"
                        >
                            <span className="flex items-center gap-2">
                                <Users className="h-3.5 w-3.5" />
                                Followers
                            </span>
                            <span className="font-mono text-xs text-foreground">{profileData.stats.followers}</span>
                        </Link>
                        <Link
                            href="#"
                            className="flex items-center justify-between text-sm text-muted-foreground hover:text-foreground transition-colors -mx-2 px-2 py-1.5 rounded-md hover:bg-accent/50"
                        >
                            <span className="flex items-center gap-2">
                                <Users className="h-3.5 w-3.5" />
                                Following
                            </span>
                            <span className="font-mono text-xs text-foreground">{profileData.stats.following}</span>
                        </Link>
                    </div>

                    <div className="pt-4 border-t border-border mt-4">
                        <h3 className="text-sm font-medium uppercase tracking-wider text-muted-foreground mb-2">Organizations</h3>
                        {profileData.orgs.map((org) => (
                            <Link
                                key={org.name}
                                href={`/${org.name}`}
                                className="flex items-center gap-2.5 text-sm text-muted-foreground hover:text-foreground transition-colors -mx-2 px-2 py-1.5 rounded-md hover:bg-accent/50"
                            >
                                <div className="h-5 w-5 flex items-center justify-center rounded-sm bg-secondary border border-border text-[10px] font-medium shrink-0">
                                    {org.name[0].toUpperCase()}
                                </div>
                                {org.name}
                            </Link>
                        ))}
                    </div>
                </aside>

                <main className="flex-1 min-w-0 overflow-y-auto p-6 space-y-8">
                    <section>
                        <div className="flex items-center justify-between mb-3">
                            <h2 className="text-xs font-medium uppercase tracking-widest text-muted-foreground">Contributions</h2>
                            <span className="text-xs text-muted-foreground font-mono">1,284 in the last year</span>
                        </div>
                        <ContributionGraph />
                    </section>

                    <section>
                        <h2 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">Activity</h2>
                        <ActivityFeed events={profileData.recentActivity} />
                    </section>
                </main>

                <aside className="w-72 shrink-0 border-l border-border overflow-y-auto p-5 space-y-6">
                    <div>
                        <h3 className="text-sm font-medium uppercase tracking-wider text-muted-foreground mb-3">Pinned</h3>
                        <div className="-mx-2">
                            {profileData.allRepos
                                .filter((r) => pinnedKeys.has(`${r.org}/${r.name}`))
                                .map((repo) => {
                                    const key = `${repo.org}/${repo.name}`;
                                    return (
                                        <div
                                            key={key}
                                            className="group flex flex-col gap-1 px-2 py-3 border-b border-border last:border-0 hover:bg-accent/30 transition-colors rounded-sm"
                                        >
                                            <div className="flex items-center gap-2">
                                                {repo.isPrivate ? (
                                                    <Lock className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                                                ) : (
                                                    <Package className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                                                )}
                                                <Link
                                                    href={`/${repo.org}/${repo.name}`}
                                                    className="text-sm font-medium truncate flex-1 hover:underline"
                                                >
                                                    {repo.org !== username && (
                                                        <span className="text-muted-foreground font-normal">{repo.org}/</span>
                                                    )}
                                                    {repo.name}
                                                </Link>
                                                <button
                                                    onClick={() => togglePin(key)}
                                                    title="Unpin repository"
                                                    className="shrink-0 flex items-center gap-1 px-1.5 py-0.5 text-[10px] rounded border border-transparent text-muted-foreground opacity-0 group-hover:opacity-100 hover:border-border hover:bg-secondary transition-all"
                                                >
                                                    <PinOff className="h-3 w-3" />
                                                    Unpin
                                                </button>
                                            </div>
                                            {repo.description && (
                                                <p className="text-xs text-muted-foreground leading-relaxed line-clamp-1 pl-5">
                                                    {repo.description}
                                                </p>
                                            )}
                                            <div className="flex items-center gap-3 text-xs text-muted-foreground pl-5">
                                                <span className="flex items-center gap-1">
                                                    <span
                                                        className="h-2 w-2 rounded-full shrink-0"
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
                                    );
                                })}
                        </div>
                    </div>

                    <div>
                        <div className="flex items-center justify-between mb-3">
                            <h3 className="text-sm font-medium uppercase tracking-wider text-muted-foreground">Repositories</h3>
                            <Link
                                href="/new"
                                className="text-xs text-muted-foreground hover:text-foreground transition-colors flex items-center gap-1"
                            >
                                <Plus className="h-3 w-3" />
                                New
                            </Link>
                        </div>
                        <div className="-mx-2">
                            {profileData.allRepos.map((repo) => {
                                const key = `${repo.org}/${repo.name}`;
                                const isPinned = pinnedKeys.has(key);
                                return (
                                    <div
                                        key={key}
                                        className="group flex flex-col gap-1 px-2 py-3 border-b border-border last:border-0 hover:bg-accent/30 transition-colors rounded-sm"
                                    >
                                        <div className="flex items-center gap-2">
                                            {repo.isPrivate ? (
                                                <Lock className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                                            ) : (
                                                <Package className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                                            )}
                                            <Link
                                                href={`/${repo.org}/${repo.name}`}
                                                className="text-sm font-medium truncate flex-1 hover:underline"
                                            >
                                                {repo.org !== username && (
                                                    <span className="text-muted-foreground font-normal">{repo.org}/</span>
                                                )}
                                                {repo.name}
                                            </Link>
                                            <button
                                                onClick={() => togglePin(key)}
                                                title={isPinned ? "Unpin repository" : "Pin repository"}
                                                className="shrink-0 flex items-center gap-1 px-1.5 py-0.5 text-[10px] rounded border border-transparent text-muted-foreground opacity-0 group-hover:opacity-100 hover:border-border hover:bg-secondary transition-all"
                                            >
                                                {isPinned ? (
                                                    <>
                                                        <PinOff className="h-3 w-3" />
                                                        Unpin
                                                    </>
                                                ) : (
                                                    <>
                                                        <Pin className="h-3 w-3" />
                                                        Pin
                                                    </>
                                                )}
                                            </button>
                                        </div>
                                        {repo.description && (
                                            <p className="text-xs text-muted-foreground leading-relaxed line-clamp-1 pl-5">
                                                {repo.description}
                                            </p>
                                        )}
                                        <div className="flex items-center gap-3 text-xs text-muted-foreground pl-5">
                                            <span className="flex items-center gap-1">
                                                <span
                                                    className="h-2 w-2 rounded-full shrink-0"
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
                                );
                            })}
                        </div>
                    </div>
                </aside>
            </div>
        </div>
    );
}

"use client";

import { Fragment, useState } from "react";
import Link from "next/link";
import { useParams } from "next/navigation";
import { format } from "date-fns";
import useSWR from "swr";
import { Star, Lock, Globe, Calendar, Pin, PinOff, ShieldCheck, Settings, Plus } from "lucide-react";
import { TopBar } from "@/components/top-bar";
import { ErrorDisplay } from "@/components/error-display";
import { Skeleton } from "@/components/ui/skeleton";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import { Badge } from "@/components/ui/badge";
import { useAuth } from "@/hooks/use-auth";
import { jsonFetcher } from "@/lib/fetchers";
import * as allLangs from "linguist-languages";

interface UserProfileRepo {
    id: string;
    name: string;
    description: string;
    visibility: "public" | "internal" | "private";
    archived: boolean;
    languages: Record<string, number> | null;
    stars: number;
}

interface UserProfileStats {
    repos: number;
    starsEarned: number;
    starsGiven: number;
}

interface UserProfileResponse {
    id: string;
    username: string;
    admin: boolean;
    createdAt: string;
    repos: UserProfileRepo[];
    stats: UserProfileStats;
}

function getTopLanguage(languages: Record<string, number> | null): { name: string; color: string } | null {
    if (!languages || Object.keys(languages).length === 0) {
        return null;
    }

    const top = Object.entries(languages).sort((a, b) => b[1] - a[1])[0];
    const color = (allLangs as Record<string, { color?: string }>)[top[0]]?.color;
    if (color) {
        return { name: top[0], color };
    }
    let hash = 0;
    for (let i = 0; i < top[0].length; i++) {
        hash = top[0].charCodeAt(i) + ((hash << 5) - hash);
    }
    return { name: top[0], color: `hsl(${Math.abs(hash) % 360}, 60%, 55%)` };
}

function ContributionGraph() {
    const weeks = 52;
    const days = 7;
    const levels = [0, 1, 2, 3, 4];

    const data = Array.from({ length: weeks }, () =>
        Array.from({ length: days }, () => {
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

    const dayLabels = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

    const now = new Date();
    const dayOfWeek = (now.getDay() + 6) % 7;
    const endMonday = new Date(now);
    endMonday.setDate(now.getDate() - dayOfWeek);
    const startDate = new Date(endMonday);
    startDate.setDate(endMonday.getDate() - (weeks - 1) * 7);

    const monthLabels: { label: string; col: number }[] = [];
    const monthNames = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let lastMonth = -1;
    for (let w = 0; w < weeks; w++) {
        const weekStart = new Date(startDate);
        weekStart.setDate(weekStart.getDate() + w * 7);
        const month = weekStart.getMonth();
        if (month !== lastMonth) {
            monthLabels.push({ label: monthNames[month], col: w });
            lastMonth = month;
        }
    }

    return (
        <TooltipProvider>
            <div className="w-full">
                <div className="grid gap-[3px]" style={{ gridTemplateColumns: `auto repeat(${weeks}, 1fr)` }}>
                    <div />
                    {Array.from({ length: weeks }, (_, wi) => {
                        const ml = monthLabels.find((m) => m.col === wi);
                        return (
                            <div key={wi} className="text-[10px] text-muted-foreground leading-none truncate">
                                {ml ? ml.label : ""}
                            </div>
                        );
                    })}

                    {Array.from({ length: days }, (_, di) => (
                        <Fragment key={di}>
                            <div className="text-[10px] text-muted-foreground leading-none flex items-center justify-end pr-1">
                                {di % 2 === 1 ? dayLabels[di] : ""}
                            </div>
                            {data.map((week, wi) => {
                                const tileDate = new Date(startDate);
                                tileDate.setDate(tileDate.getDate() + wi * 7 + di);
                                const dateStr = format(tileDate, "EEEE, d MMMM yyyy");
                                const tile = (
                                    <div
                                        key={wi}
                                        className={`aspect-square w-full rounded-sm ${levelClass(week[di])} transition-opacity hover:opacity-70`}
                                    />
                                );
                                if (week[di] === 0) {
                                    return tile;
                                }
                                return (
                                    <Tooltip key={wi}>
                                        <TooltipTrigger asChild>{tile}</TooltipTrigger>
                                        <TooltipContent>
                                            <p>
                                                {week[di]} contribution{week[di] !== 1 ? "s" : ""} on {dateStr}
                                            </p>
                                        </TooltipContent>
                                    </Tooltip>
                                );
                            })}
                        </Fragment>
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
        </TooltipProvider>
    );
}

function ProfileSkeleton({ username }: { username: string }) {
    return (
        <div className="min-h-screen bg-background text-foreground flex flex-col">
            <TopBar breadcrumb={[{ label: username }]} hasNotifications />
            <div className="flex flex-col lg:flex-row flex-1 min-h-0">
                <aside className="w-full lg:w-64 shrink-0 lg:border-r border-b lg:border-b-0 border-border overflow-y-auto p-5">
                    <div className="mb-4 flex items-center gap-4 lg:block">
                        <Skeleton className="h-16 w-16 lg:h-20 lg:w-20 rounded-full lg:mb-3 shrink-0" />
                        <div className="min-w-0">
                            <Skeleton className="h-5 w-32 mb-1" />
                            <Skeleton className="h-4 w-24" />
                        </div>
                    </div>
                    <div className="pt-4 border-t border-border space-y-2">
                        <Skeleton className="h-4 w-40" />
                    </div>
                    <div className="pt-4 border-t border-border mt-4 space-y-2">
                        <Skeleton className="h-4 w-20 mb-2" />
                        <Skeleton className="h-8 w-full" />
                        <Skeleton className="h-8 w-full" />
                        <Skeleton className="h-8 w-full" />
                    </div>
                </aside>
                <main className="flex-1 min-w-0 overflow-y-auto p-4 lg:p-6 space-y-8">
                    <Skeleton className="h-40 w-full" />
                </main>
                <aside className="w-full lg:w-72 shrink-0 lg:border-l border-t lg:border-t-0 border-border overflow-y-auto p-4 lg:p-5 space-y-6">
                    <Skeleton className="h-4 w-24 mb-3" />
                    <Skeleton className="h-20 w-full" />
                    <Skeleton className="h-20 w-full" />
                    <Skeleton className="h-20 w-full" />
                </aside>
            </div>
        </div>
    );
}

export default function UserProfilePage() {
    const params = useParams();
    const username = params.user as string;
    const { user: authUser, isLoading: authLoading } = useAuth();

    const { data: profile, error, isLoading } = useSWR<UserProfileResponse>(`/api/users/${username}`, jsonFetcher);

    const [pinnedKeys, setPinnedKeys] = useState<Set<string>>(new Set());

    if (isLoading || authLoading) {
        return <ProfileSkeleton username={username} />;
    }

    if (error || !profile) {
        return <ErrorDisplay failed="user profile" error={error} />;
    }

    const isCurrentUser = authUser != null && authUser.username.toLowerCase() === profile.username.toLowerCase();
    const joinedFormatted = format(new Date(profile.createdAt), "MMMM yyyy");

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

    const pinnedRepos = profile.repos.filter((r) => pinnedKeys.has(`${profile.username}/${r.name}`));

    return (
        <div className="min-h-screen bg-background text-foreground flex flex-col">
            <TopBar breadcrumb={[{ label: profile.username }]} hasNotifications />

            <div className="flex flex-col lg:flex-row flex-1 min-h-0">
                <aside className="w-full lg:w-64 shrink-0 lg:border-r border-b lg:border-b-0 border-border overflow-y-auto p-5">
                    <div className="mb-4 flex items-center gap-4 lg:block">
                        <div className="h-16 w-16 lg:h-20 lg:w-20 flex items-center justify-center rounded-full bg-secondary border-2 border-border text-2xl lg:text-3xl font-semibold text-foreground lg:mb-3 shrink-0">
                            {profile.username[0].toUpperCase()}
                        </div>
                        <div className="min-w-0">
                            <div className="flex items-center gap-2">
                                <p className="text-base font-semibold leading-tight">{profile.username}</p>
                                {profile.admin && (
                                    <span className="inline-flex items-center gap-1 px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wider border border-border rounded text-muted-foreground bg-secondary">
                                        <ShieldCheck className="h-2.5 w-2.5" />
                                        Admin
                                    </span>
                                )}
                            </div>
                            <p className="text-sm text-muted-foreground font-mono">{profile.username}</p>
                            {isCurrentUser && (
                                <Link
                                    href="/settings"
                                    className="mt-3 w-full lg:w-full inline-flex items-center justify-center gap-1.5 px-3 py-1.5 text-xs border border-border rounded-md hover:bg-accent/50 transition-colors"
                                >
                                    <Settings className="h-3 w-3" />
                                    Edit profile
                                </Link>
                            )}
                        </div>
                    </div>

                    <div className="pt-4 border-t border-border space-y-2">
                        <div className="flex items-center gap-2 text-sm text-muted-foreground">
                            <Calendar className="h-3.5 w-3.5 shrink-0" />
                            Joined {joinedFormatted}
                        </div>
                    </div>

                    <div className="pt-4 border-t border-border mt-4 space-y-1">
                        <h3 className="text-sm font-medium uppercase tracking-wider text-muted-foreground mb-2">Stats</h3>
                        <div className="grid grid-cols-3 gap-2 lg:grid-cols-1 lg:gap-0">
                            <div className="flex items-center justify-between text-sm text-muted-foreground -mx-2 px-2 py-1.5 rounded-md">
                                <span className="flex items-center gap-2">
                                    <Globe className="h-3.5 w-3.5" />
                                    Repositories
                                </span>
                                <span className="font-mono text-xs text-foreground">{profile.stats.repos}</span>
                            </div>
                            <div className="flex items-center justify-between text-sm text-muted-foreground -mx-2 px-2 py-1.5 rounded-md">
                                <span className="flex items-center gap-2">
                                    <Star className="h-3.5 w-3.5" />
                                    Stars earned
                                </span>
                                <span className="font-mono text-xs text-foreground">{profile.stats.starsEarned.toLocaleString()}</span>
                            </div>
                            <div className="flex items-center justify-between text-sm text-muted-foreground -mx-2 px-2 py-1.5 rounded-md">
                                <span className="flex items-center gap-2">
                                    <Star className="h-3.5 w-3.5" />
                                    Stars given
                                </span>
                                <span className="font-mono text-xs text-foreground">{profile.stats.starsGiven.toLocaleString()}</span>
                            </div>
                        </div>
                    </div>
                </aside>

                <main className="flex-1 min-w-0 overflow-y-auto p-4 lg:p-6 space-y-8">
                    <section>
                        <div className="flex items-center justify-between mb-3">
                            <h2 className="text-xs font-medium uppercase tracking-widest text-muted-foreground">Contributions</h2>
                        </div>
                        <div className="overflow-x-auto -mx-4 px-4 lg:mx-0 lg:px-0">
                            <div className="min-w-[640px]">
                                <ContributionGraph />
                            </div>
                        </div>
                    </section>
                </main>

                <aside className="w-full lg:w-72 shrink-0 lg:border-l border-t lg:border-t-0 border-border overflow-y-auto p-4 lg:p-5 space-y-6">
                    {pinnedRepos.length > 0 && (
                        <div>
                            <h3 className="text-sm font-medium uppercase tracking-wider text-muted-foreground mb-3">Pinned</h3>
                            <div className="-mx-2">
                                {pinnedRepos.map((repo) => {
                                    const key = `${profile.username}/${repo.name}`;
                                    const lang = getTopLanguage(repo.languages);
                                    return (
                                        <div
                                            key={key}
                                            className="group flex flex-col gap-1 px-2 py-3 border-b border-border last:border-0 hover:bg-accent/30 transition-colors rounded-sm"
                                        >
                                            <div className="flex items-center gap-2">
                                                <Globe className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                                                <Link
                                                    href={`/${profile.username}/${repo.name}`}
                                                    className="text-sm font-medium truncate hover:underline"
                                                >
                                                    {repo.name}
                                                </Link>
                                                {repo.visibility === "private" && (
                                                    <Badge variant="secondary" className="shrink-0">
                                                        <Lock className="h-3 w-3" />
                                                        Private
                                                    </Badge>
                                                )}
                                                {repo.visibility === "internal" && (
                                                    <Badge variant="outline" className="shrink-0">
                                                        <Globe className="h-3 w-3" />
                                                        Internal
                                                    </Badge>
                                                )}
                                                {isCurrentUser && (
                                                    <button
                                                        onClick={() => togglePin(key)}
                                                        title="Unpin repository"
                                                        className="shrink-0 ml-auto flex items-center gap-1 px-1.5 py-0.5 text-[10px] rounded border border-transparent text-muted-foreground opacity-0 group-hover:opacity-100 hover:border-border hover:bg-secondary transition-all"
                                                    >
                                                        <PinOff className="h-3 w-3" />
                                                        Unpin
                                                    </button>
                                                )}
                                            </div>
                                            {repo.description && (
                                                <p className="text-xs text-muted-foreground leading-relaxed line-clamp-1 pl-5">
                                                    {repo.description}
                                                </p>
                                            )}
                                            <div className="flex items-center gap-3 text-xs text-muted-foreground pl-5">
                                                {lang && (
                                                    <span className="flex items-center gap-1">
                                                        <span
                                                            className="h-2 w-2 rounded-full shrink-0"
                                                            style={{ backgroundColor: lang.color }}
                                                        />
                                                        {lang.name}
                                                    </span>
                                                )}
                                                <span className="flex items-center gap-1">
                                                    <Star className="h-3 w-3" />
                                                    {repo.stars}
                                                </span>
                                            </div>
                                        </div>
                                    );
                                })}
                            </div>
                        </div>
                    )}

                    <div>
                        <div className="flex items-center justify-between mb-3">
                            <h3 className="text-sm font-medium uppercase tracking-wider text-muted-foreground">Repositories</h3>
                            {isCurrentUser && (
                                <Link
                                    href="/new"
                                    className="text-xs text-muted-foreground hover:text-foreground transition-colors flex items-center gap-1"
                                >
                                    <Plus className="h-3 w-3" />
                                    New
                                </Link>
                            )}
                        </div>
                        <div className="-mx-2">
                            {profile.repos.map((repo) => {
                                const key = `${profile.username}/${repo.name}`;
                                const isPinned = pinnedKeys.has(key);
                                const lang = getTopLanguage(repo.languages);
                                return (
                                    <div
                                        key={key}
                                        className="group flex flex-col gap-1 px-2 py-3 border-b border-border last:border-0 hover:bg-accent/30 transition-colors rounded-sm"
                                    >
                                        <div className="flex items-center gap-2">
                                            <Globe className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                                            <Link
                                                href={`/${profile.username}/${repo.name}`}
                                                className="text-sm font-medium truncate hover:underline"
                                            >
                                                {repo.name}
                                            </Link>
                                            {repo.visibility === "private" && (
                                                <Badge variant="secondary" className="shrink-0">
                                                    <Lock className="h-3 w-3" />
                                                    Private
                                                </Badge>
                                            )}
                                            {repo.visibility === "internal" && (
                                                <Badge variant="outline" className="shrink-0">
                                                    <Globe className="h-3 w-3" />
                                                    Internal
                                                </Badge>
                                            )}
                                            {isCurrentUser && (
                                                <button
                                                    onClick={() => togglePin(key)}
                                                    title={isPinned ? "Unpin repository" : "Pin repository"}
                                                    className="shrink-0 ml-auto flex items-center gap-1 px-1.5 py-0.5 text-[10px] rounded border border-transparent text-muted-foreground opacity-0 group-hover:opacity-100 hover:border-border hover:bg-secondary transition-all"
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
                                            )}
                                        </div>
                                        {repo.description && (
                                            <p className="text-xs text-muted-foreground leading-relaxed line-clamp-1 pl-5">
                                                {repo.description}
                                            </p>
                                        )}
                                        <div className="flex items-center gap-3 text-xs text-muted-foreground pl-5">
                                            {lang && (
                                                <span className="flex items-center gap-1">
                                                    <span
                                                        className="h-2 w-2 rounded-full shrink-0"
                                                        style={{ backgroundColor: lang.color }}
                                                    />
                                                    {lang.name}
                                                </span>
                                            )}
                                            <span className="flex items-center gap-1">
                                                <Star className="h-3 w-3" />
                                                {repo.stars}
                                            </span>
                                        </div>
                                    </div>
                                );
                            })}
                            {profile.repos.length === 0 && (
                                <p className="text-sm text-muted-foreground py-4 text-center">No repositories yet</p>
                            )}
                        </div>
                    </div>
                </aside>
            </div>
        </div>
    );
}

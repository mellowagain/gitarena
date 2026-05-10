"use client";

import { Fragment, useState } from "react";
import Link from "next/link";
import { useParams } from "next/navigation";
import { format } from "date-fns";
import { uuidToDate } from "@/lib/utils";
import useSWR from "swr";
import { Star, Lock, Globe, Calendar, Pin, PinOff, ShieldCheck, Settings, Plus, Users, Building2 } from "lucide-react";
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
    repos: UserProfileRepo[];
    stats: UserProfileStats;
}

interface OrgInfo {
    id: string;
    name: string;
    description: string;
}

interface OrgMemberRaw {
    userId: string;
    role: "owner" | "admin" | "member";
}

interface OrgMember {
    userId: string;
    username: string;
    role: "owner" | "admin" | "member";
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

function RepoList({
    repos,
    namespace,
    canPin,
    pinnedKeys,
    onTogglePin,
}: {
    repos: UserProfileRepo[];
    namespace: string;
    canPin: boolean;
    pinnedKeys: Set<string>;
    onTogglePin: (key: string) => void;
}) {
    return (
        <div className="-mx-2">
            {repos.map((repo) => {
                const key = `${namespace}/${repo.name}`;
                const isPinned = pinnedKeys.has(key);
                const lang = getTopLanguage(repo.languages);
                return (
                    <div
                        key={key}
                        className="group flex flex-col gap-1 px-2 py-3 border-b border-border last:border-0 hover:bg-accent/30 transition-colors rounded-sm"
                    >
                        <div className="flex items-center gap-2">
                            <Globe className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                            <Link href={`/${namespace}/${repo.name}`} className="text-sm font-medium truncate hover:underline">
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
                            {canPin && (
                                <button
                                    onClick={() => onTogglePin(key)}
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
                            <p className="text-xs text-muted-foreground leading-relaxed line-clamp-1 pl-5">{repo.description}</p>
                        )}
                        <div className="flex items-center gap-3 text-xs text-muted-foreground pl-5">
                            {lang && (
                                <span className="flex items-center gap-1">
                                    <span className="h-2 w-2 rounded-full shrink-0" style={{ backgroundColor: lang.color }} />
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
            {repos.length === 0 && <p className="text-sm text-muted-foreground py-4 text-center">No repositories yet</p>}
        </div>
    );
}

const roleColors: Record<string, string> = {
    owner: "text-amber-500 border-amber-500/30 bg-amber-500/10",
    admin: "text-blue-500 border-blue-500/30 bg-blue-500/10",
    member: "text-muted-foreground border-border bg-secondary",
};

function OrgProfilePage({ name, authUserId }: { name: string; authUserId: string | null }) {
    const { data: org, error, isLoading } = useSWR<OrgInfo>(`/api/orgs/${name}`, jsonFetcher);
    const { data: rawMembers, isLoading: membersLoading } = useSWR<OrgMemberRaw[]>(`/api/orgs/${name}/members`, jsonFetcher);

    const [resolvedNames, setResolvedNames] = useState<Map<number, string>>(new Map());
    const [activeTab, setActiveTab] = useState<"overview" | "repos" | "members">("overview");

    // Resolve usernames for each member ID
    if (rawMembers) {
        rawMembers.forEach(async (m) => {
            if (!resolvedNames.has(m.userId)) {
                try {
                    const res = await fetch(`/api/users/by-id/${m.userId}`);
                    if (res.ok) {
                        const data: { id: string; username: string } = await res.json();
                        setResolvedNames((prev) => new Map(prev).set(m.userId, data.username));
                    }
                } catch {
                    // ignore
                }
            }
        });
    }

    if (isLoading || membersLoading) {
        return <ProfileSkeleton username={name} />;
    }

    if (error || !org) {
        return <ErrorDisplay failed="organization" error={error} />;
    }

    const members: OrgMember[] = (rawMembers ?? []).map((m) => ({
        userId: m.userId,
        username: resolvedNames.get(m.userId) ?? `#${m.userId}`,
        role: m.role,
    }));

    const currentUserMember = authUserId != null ? members.find((m) => m.userId === authUserId) : undefined;
    const isAdmin = currentUserMember != null && (currentUserMember.role === "owner" || currentUserMember.role === "admin");
    const memberCount = members.length;

    const tabs = [
        { id: "overview", label: "Overview" },
        { id: "repos", label: "Repositories" },
        { id: "members", label: "Members", count: memberCount },
    ] as const;

    return (
        <div className="min-h-screen bg-background text-foreground flex flex-col">
            <TopBar breadcrumb={[{ label: org.name }]} hasNotifications />

            <div className="flex flex-1 min-h-0">
                {/* ── Left sidebar ─────────────────────────────────────────── */}
                <aside className="w-64 shrink-0 border-r border-border overflow-y-auto p-5">
                    {/* Org avatar + name */}
                    <div className="mb-4">
                        <div className="h-20 w-20 flex items-center justify-center rounded-xl bg-secondary border-2 border-border text-3xl font-semibold text-foreground mb-3">
                            <Building2 className="h-10 w-10" />
                        </div>
                        <div className="flex items-center gap-2 flex-wrap">
                            <p className="text-base font-semibold leading-tight">{org.name}</p>
                        </div>
                        <p className="text-sm text-muted-foreground font-mono">@{org.name}</p>
                        {org.description && <p className="text-sm text-muted-foreground leading-relaxed mt-2">{org.description}</p>}

                        {/* Action buttons */}
                        <div className="flex flex-col gap-2 mt-3">
                            {isAdmin ? (
                                <Link
                                    href={`/orgs/${org.name}/settings`}
                                    className="w-full flex items-center justify-center gap-1.5 px-3 py-1.5 text-xs border border-border rounded-md hover:bg-accent/50 transition-colors"
                                >
                                    <Settings className="h-3 w-3" />
                                    Organization settings
                                </Link>
                            ) : (
                                <button className="w-full flex items-center justify-center gap-1.5 px-3 py-1.5 text-xs border border-border rounded-md hover:bg-accent/50 transition-colors">
                                    <Users className="h-3 w-3" />
                                    Follow organization
                                </button>
                            )}
                        </div>
                    </div>

                    {/* Stats */}
                    <div className="pt-4 border-t border-border space-y-1">
                        <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-2">Stats</h3>
                        <div className="flex items-center justify-between text-sm text-muted-foreground -mx-2 px-2 py-1.5 rounded-md">
                            <span className="flex items-center gap-2">
                                <Users className="h-3.5 w-3.5" />
                                Members
                            </span>
                            <span className="font-mono text-xs text-foreground">{memberCount}</span>
                        </div>
                    </div>

                    {/* Members preview */}
                    {members.length > 0 && (
                        <div className="pt-4 border-t border-border mt-4">
                            <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">Members</h3>
                            <div className="flex flex-wrap gap-1.5">
                                {members.slice(0, 12).map((m) => (
                                    <Link
                                        key={m.userId}
                                        href={`/${m.username}`}
                                        title={`${m.username} (${m.role})`}
                                        className="h-7 w-7 flex items-center justify-center rounded-full bg-secondary border border-border text-[11px] font-medium hover:ring-2 hover:ring-ring transition-all"
                                    >
                                        {m.username[0].toUpperCase()}
                                    </Link>
                                ))}
                            </div>
                        </div>
                    )}
                </aside>

                {/* ── Main content ─────────────────────────────────────────── */}
                <main className="flex-1 min-w-0 overflow-y-auto">
                    {/* Tab bar */}
                    <div className="border-b border-border px-6 flex items-center gap-1">
                        {tabs.map((tab) => (
                            <button
                                key={tab.id}
                                onClick={() => setActiveTab(tab.id)}
                                className={`flex items-center gap-2 px-3 py-3 text-sm border-b-2 transition-colors ${
                                    activeTab === tab.id
                                        ? "border-foreground text-foreground font-medium"
                                        : "border-transparent text-muted-foreground hover:text-foreground"
                                }`}
                            >
                                {tab.label}
                                {"count" in tab && (
                                    <span className="px-1.5 py-0.5 text-[10px] font-mono rounded bg-secondary border border-border text-muted-foreground">
                                        {tab.count}
                                    </span>
                                )}
                            </button>
                        ))}
                    </div>

                    <div className="p-6 space-y-8">
                        {/* ── Overview tab ── */}
                        {activeTab === "overview" && (
                            <div className="text-sm text-muted-foreground">
                                <p>No recent activity to display yet.</p>
                            </div>
                        )}

                        {/* ── Repositories tab ── */}
                        {activeTab === "repos" && (
                            <section>
                                <div className="flex items-center justify-between gap-4 mb-4">
                                    <p className="text-sm text-muted-foreground">
                                        Repository listing is not yet available for organizations.
                                    </p>
                                    {isAdmin && (
                                        <Link
                                            href={`/new?namespace=${org.name}`}
                                            className="flex items-center gap-1.5 px-3 py-1.5 text-xs border border-border rounded-md hover:bg-accent/50 transition-colors whitespace-nowrap"
                                        >
                                            <Plus className="h-3.5 w-3.5" />
                                            New repository
                                        </Link>
                                    )}
                                </div>
                            </section>
                        )}

                        {/* ── Members tab ── */}
                        {activeTab === "members" && (
                            <section>
                                {isAdmin && (
                                    <div className="flex justify-end mb-4">
                                        <Link
                                            href={`/orgs/${org.name}/settings?tab=members`}
                                            className="flex items-center gap-1.5 px-3 py-1.5 text-xs border border-border rounded-md hover:bg-accent/50 transition-colors"
                                        >
                                            <Plus className="h-3.5 w-3.5" />
                                            Manage members
                                        </Link>
                                    </div>
                                )}

                                <div className="border border-border rounded-md overflow-hidden">
                                    {members.map((member, i) => (
                                        <div
                                            key={member.userId}
                                            className={`flex items-center gap-4 px-4 py-3.5 hover:bg-accent/30 transition-colors ${i > 0 ? "border-t border-border" : ""}`}
                                        >
                                            <div className="h-9 w-9 flex items-center justify-center rounded-full bg-secondary border border-border text-sm font-semibold shrink-0">
                                                {member.username[0].toUpperCase()}
                                            </div>
                                            <div className="flex-1 min-w-0">
                                                <div className="flex items-center gap-2">
                                                    <Link href={`/${member.username}`} className="text-sm font-medium hover:underline">
                                                        {member.username}
                                                    </Link>
                                                    <span
                                                        className={`inline-flex px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wider border rounded ${roleColors[member.role] ?? roleColors.member}`}
                                                    >
                                                        {member.role}
                                                    </span>
                                                </div>
                                                <p className="text-xs text-muted-foreground font-mono">@{member.username}</p>
                                            </div>
                                        </div>
                                    ))}
                                    {members.length === 0 && (
                                        <div className="px-4 py-8 text-center text-sm text-muted-foreground">No members found.</div>
                                    )}
                                </div>
                            </section>
                        )}
                    </div>
                </main>
            </div>
        </div>
    );
}

// Fetcher that returns null on 404 instead of throwing, so we can fall through to org lookup
async function userFetcherWith404(url: string): Promise<UserProfileResponse | null> {
    const res = await fetch(url);
    if (res.status === 404) {
        return null;
    }
    if (!res.ok) {
        throw new Error(res.statusText);
    }
    return res.json();
}

export default function NamespacePage() {
    const params = useParams();
    const namespace = params.user as string;
    const { user: authUser, isLoading: authLoading } = useAuth();

    const {
        data: profile,
        error: userError,
        isLoading: userLoading,
    } = useSWR<UserProfileResponse | null>(`/api/users/${namespace}`, userFetcherWith404);

    const [pinnedKeys, setPinnedKeys] = useState<Set<string>>(new Set());

    // Only fetch org if user lookup returned null (404)
    const isUserNotFound = !userLoading && profile === null && !userError;

    if (userLoading || authLoading) {
        return <ProfileSkeleton username={namespace} />;
    }

    // User lookup gave a real error (not 404)
    if (userError) {
        return <ErrorDisplay failed="profile" error={userError} />;
    }

    // User not found — try org
    if (isUserNotFound) {
        return <OrgProfilePage name={namespace} authUserId={authUser?.id ?? null} />;
    }

    if (!profile) {
        return <ErrorDisplay failed="user profile" error={undefined} />;
    }

    const isCurrentUser = authUser != null && authUser.username.toLowerCase() === profile.username.toLowerCase();
    const joinedFormatted = format(uuidToDate(profile.id), "MMMM yyyy");

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
                            <RepoList
                                repos={pinnedRepos}
                                namespace={profile.username}
                                canPin={isCurrentUser}
                                pinnedKeys={pinnedKeys}
                                onTogglePin={togglePin}
                            />
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
                        <RepoList
                            repos={profile.repos}
                            namespace={profile.username}
                            canPin={isCurrentUser}
                            pinnedKeys={pinnedKeys}
                            onTogglePin={togglePin}
                        />
                    </div>
                </aside>
            </div>
        </div>
    );
}

"use client";

import { useState } from "react";
import Link from "next/link";
import { useParams } from "next/navigation";
import { format } from "date-fns";
import { uuidToDate } from "@/lib/utils";
import useSWR from "swr";
import { Star, Lock, Globe, Calendar, Pin, PinOff, ShieldCheck, Settings, Plus, Users, Building2 } from "lucide-react";
import { TopBar } from "@/components/top-bar";
import { ErrorDisplay } from "@/components/error-display";
import { Skeleton } from "@/components/ui/skeleton";
import { Badge } from "@/components/ui/badge";
import { useAuth } from "@/hooks/use-auth";
import { jsonFetcher } from "@/lib/fetchers";
import { ActivityEvent, type EventResponse } from "@/components/activity-event";
import { ContributionGraph } from "@/components/contribution-graph";
import * as allLangs from "linguist-languages";

interface UserProfileRepo {
    id: string;
    name: string;
    description: string;
    visibility: "public" | "internal" | "private";
    archivedAt: string | null;
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

interface UserOrgEntry {
    id: string;
    name: string;
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

interface OrgRepo {
    id: string;
    name: string;
    description: string;
    visibility: string;
    archivedAt: string | null;
    languages: Record<string, number> | null;
    stars: number;
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
                            {repo.archivedAt && (
                                <span className="px-1.5 py-0.5 text-[10px] font-medium rounded bg-secondary text-muted-foreground border border-border leading-none shrink-0">
                                    archived
                                </span>
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

function OrgMemberCard({ member, variant = "row" }: { member: OrgMemberRaw; variant?: "row" | "avatar" }) {
    const { data: user } = useSWR<{ id: string; username: string }>(`/api/users/by-id/${member.userId}`, jsonFetcher);
    const username = user?.username ?? "…";

    if (variant === "avatar") {
        return (
            <Link
                href={user ? `/${username}` : "#"}
                title={user ? `${username} (${member.role})` : member.role}
                className="h-7 w-7 flex items-center justify-center rounded-full bg-secondary border border-border text-[11px] font-medium hover:ring-2 hover:ring-ring transition-all"
            >
                {username[0].toUpperCase()}
            </Link>
        );
    }

    return (
        <div className="flex items-center gap-4 px-4 py-3.5 hover:bg-accent/30 transition-colors border-t border-border first:border-t-0">
            <div className="h-9 w-9 flex items-center justify-center rounded-full bg-secondary border border-border text-sm font-semibold shrink-0">
                {username[0].toUpperCase()}
            </div>
            <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                    {user ? (
                        <Link href={`/${username}`} className="text-sm font-medium hover:underline">
                            {username}
                        </Link>
                    ) : (
                        <span className="text-sm font-medium text-muted-foreground">Loading…</span>
                    )}
                    <span
                        className={`inline-flex px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wider border rounded ${roleColors[member.role] ?? roleColors.member}`}
                    >
                        {member.role}
                    </span>
                </div>
                {user && <p className="text-xs text-muted-foreground font-mono">@{username}</p>}
            </div>
        </div>
    );
}

function OrgProfilePage({ name, authUserId }: { name: string; authUserId: string | null }) {
    const { data: org, error, isLoading } = useSWR<OrgInfo>(`/api/orgs/${name}`, jsonFetcher);
    const { data: rawMembers, isLoading: membersLoading } = useSWR<OrgMemberRaw[]>(`/api/orgs/${name}/members`, jsonFetcher);
    const [activeTab, setActiveTab] = useState<"overview" | "repos" | "members">("overview");
    const { data: repos, isLoading: reposLoading } = useSWR<OrgRepo[]>(
        activeTab === "repos" ? `/api/orgs/${name}/repos` : null,
        jsonFetcher
    );

    if (isLoading || membersLoading) {
        return <ProfileSkeleton username={name} />;
    }

    if (error || !org) {
        return <ErrorDisplay failed="organization" error={error} />;
    }

    const memberCount = (rawMembers ?? []).length;
    const currentUserRaw = authUserId != null ? (rawMembers ?? []).find((m) => m.userId === authUserId) : undefined;
    const isAdmin = currentUserRaw != null && (currentUserRaw.role === "owner" || currentUserRaw.role === "admin");

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
                    {memberCount > 0 && (
                        <div className="pt-4 border-t border-border mt-4">
                            <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">Members</h3>
                            <div className="flex flex-wrap gap-1.5">
                                {(rawMembers ?? []).slice(0, 12).map((m) => (
                                    <OrgMemberCard key={m.userId} member={m} variant="avatar" />
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
                                    <p className="text-sm font-semibold">Repositories</p>
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
                                {reposLoading ? (
                                    <div className="space-y-2">
                                        {[0, 1, 2].map((i) => (
                                            <div key={i} className="h-14 bg-secondary/50 rounded-md animate-pulse" />
                                        ))}
                                    </div>
                                ) : (repos ?? []).length === 0 ? (
                                    <p className="text-sm text-muted-foreground py-4 text-center">No repositories yet.</p>
                                ) : (
                                    <div className="border border-border rounded-md overflow-hidden divide-y divide-border">
                                        {(repos ?? []).map((repo) => {
                                            const topLang = getTopLanguage(repo.languages);
                                            return (
                                                <div
                                                    key={repo.id}
                                                    className="flex items-center gap-3 px-4 py-3 hover:bg-accent/20 transition-colors"
                                                >
                                                    <div className="flex-1 min-w-0">
                                                        <Link
                                                            href={`/${org.name}/${repo.name}`}
                                                            className="text-sm font-medium hover:underline"
                                                        >
                                                            {repo.name}
                                                        </Link>
                                                        {repo.archivedAt && (
                                                            <span className="px-1.5 py-0.5 text-[10px] font-medium rounded bg-secondary text-muted-foreground border border-border leading-none shrink-0">
                                                                archived
                                                            </span>
                                                        )}
                                                        {repo.description && (
                                                            <p className="text-xs text-muted-foreground line-clamp-1">{repo.description}</p>
                                                        )}
                                                    </div>
                                                    <div className="flex items-center gap-3 text-xs text-muted-foreground shrink-0">
                                                        {topLang && (
                                                            <span className="flex items-center gap-1">
                                                                <span
                                                                    className="h-2 w-2 rounded-full shrink-0"
                                                                    style={{ backgroundColor: topLang.color }}
                                                                />
                                                                {topLang.name}
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
                                )}
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
                                    {(rawMembers ?? []).map((member) => (
                                        <OrgMemberCard key={member.userId} member={member} variant="row" />
                                    ))}
                                    {(rawMembers ?? []).length === 0 && (
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

    const { data: orgs, isLoading: orgsLoading } = useSWR<UserOrgEntry[]>(`/api/users/${namespace}/orgs`, jsonFetcher);
    const { data: activityFeed, isLoading: activityLoading } = useSWR<EventResponse[]>(`/api/users/${namespace}/events`, jsonFetcher);

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

                    {(orgsLoading || (orgs && orgs.length > 0)) && (
                        <div className="pt-4 border-t border-border mt-4 space-y-1">
                            <h3 className="text-sm font-medium uppercase tracking-wider text-muted-foreground mb-2">Organizations</h3>
                            {orgsLoading ? (
                                <div className="space-y-2">
                                    <Skeleton className="h-6 w-full" />
                                    <Skeleton className="h-6 w-3/4" />
                                </div>
                            ) : (
                                <div className="flex flex-wrap gap-2">
                                    {orgs!.map((org) => (
                                        <Link
                                            key={org.id}
                                            href={`/${org.name}`}
                                            className="flex items-center gap-1.5 px-2 py-1 text-xs rounded-md bg-secondary hover:bg-accent/50 text-foreground transition-colors"
                                        >
                                            <Building2 className="h-3 w-3 shrink-0 text-muted-foreground" />
                                            {org.name}
                                        </Link>
                                    ))}
                                </div>
                            )}
                        </div>
                    )}
                </aside>

                <main className="flex-1 min-w-0 overflow-y-auto p-4 lg:p-6 space-y-8">
                    <section>
                        <div className="flex items-center justify-between mb-3">
                            <h2 className="text-xs font-medium uppercase tracking-widest text-muted-foreground">Contributions</h2>
                        </div>
                        <div className="overflow-x-auto -mx-4 px-4 lg:mx-0 lg:px-0">
                            <div className="min-w-[640px]">
                                <ContributionGraph username={profile.username} />
                            </div>
                        </div>
                    </section>

                    <section>
                        <h2 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">Activity</h2>
                        {activityLoading && (
                            <div className="space-y-3">
                                <Skeleton className="h-12 w-full" />
                                <Skeleton className="h-12 w-full" />
                                <Skeleton className="h-12 w-full" />
                            </div>
                        )}
                        {!activityLoading && (!activityFeed || activityFeed.length === 0) && (
                            <div className="border border-border rounded-md px-4 py-6 text-center text-sm text-muted-foreground">
                                No public activity yet.
                            </div>
                        )}
                        {!activityLoading && activityFeed && activityFeed.length > 0 && (
                            <div className="space-y-4">
                                {activityFeed.map((event) => (
                                    <ActivityEvent key={event.id} event={event} />
                                ))}
                            </div>
                        )}
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

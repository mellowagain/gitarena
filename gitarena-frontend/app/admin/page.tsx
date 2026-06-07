"use client";

import { useState } from "react";
import useSWR from "swr";
import Link from "next/link";
import { formatDistanceToNow } from "date-fns";
import { Button } from "@/components/ui/button";
import {
    Settings,
    Users,
    Server,
    Shield,
    Database,
    Mail,
    Key,
    Globe,
    Activity,
    AlertTriangle,
    CheckCircle2,
    XCircle,
    HardDrive,
    Clock,
    GitBranch,
    FileText,
    Webhook,
    Lock,
    UserPlus,
    Ban,
    RefreshCw,
    Download,
    Search,
    MoreHorizontal,
    Plus,
    Compass,
    Bell,
    ExternalLink,
    ChevronDown,
    ChevronUp,
} from "lucide-react";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
    DropdownMenuSeparator,
} from "@/components/ui/dropdown-menu";
import { Checkbox } from "@/components/ui/checkbox";
import { TopBar } from "@/components/top-bar";
import { Skeleton } from "@/components/ui/skeleton";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import { useInstanceConfig } from "@/components/instance-config-provider";
import { jsonFetcher, authFetcher } from "@/lib/fetchers";
import { uuidToDate } from "@/lib/utils";
import type { EventResponse } from "@/components/activity-event";

interface InstanceStats {
    users: number;
    orgs: number;
    repositories: number;
    totalSpace: number;
    usedSpace: number;
}

interface InstanceHealth {
    components: InstanceComponent[];
}

interface InstanceComponent {
    name: string;
    status: ComponentStatus;
    latency: number | null;
}

type ComponentStatus = "healthy" | "unhealthy" | "disabled" | { degraded: string };

interface AdminUser {
    id: string;
    username: string;
    disabled: boolean;
    admin: boolean;
    email: string;
    verifiedAt: string | null;
}

type AdminUserStatus = "active" | "pending" | "disabled";

const adminSections = [
    {
        title: "Overview",
        items: [
            { id: "dashboard", label: "Dashboard", icon: Activity },
            { id: "announcements", label: "Announcements", icon: Bell },
        ],
    },
    {
        title: "Users & Access",
        items: [
            { id: "users", label: "Users", icon: Users },
            { id: "organizations", label: "Organizations", icon: Globe },
            { id: "access-tokens", label: "Access Tokens", icon: Key },
            { id: "oauth-apps", label: "OAuth Applications", icon: Shield },
        ],
    },
    {
        title: "Repository",
        items: [
            { id: "repositories", label: "All Repositories", icon: GitBranch },
            { id: "hooks", label: "System Hooks", icon: Webhook },
        ],
    },
    {
        title: "System",
        items: [
            { id: "settings", label: "General Settings", icon: Settings },
            { id: "email", label: "Email Configuration", icon: Mail },
            { id: "storage", label: "Storage", icon: Database },
            { id: "security", label: "Security", icon: Lock },
            { id: "integrations", label: "Integrations", icon: ExternalLink },
        ],
    },
    {
        title: "Maintenance",
        items: [
            { id: "background-jobs", label: "Background Jobs", icon: RefreshCw },
            { id: "audit-log", label: "Audit Log", icon: FileText },
            { id: "backup", label: "Backup & Restore", icon: Download },
        ],
    },
];

function StatusBadge({ status }: { status: string }) {
    const config = {
        healthy: { bg: "bg-green-500/10", text: "text-green-500", icon: CheckCircle2 },
        degraded: { bg: "bg-yellow-500/10", text: "text-yellow-500", icon: AlertTriangle },
        warning: { bg: "bg-yellow-500/10", text: "text-yellow-500", icon: AlertTriangle },
        error: { bg: "bg-red-500/10", text: "text-red-500", icon: XCircle },
        unhealthy: { bg: "bg-red-500/10", text: "text-red-500", icon: XCircle },
        disabled: { bg: "bg-muted", text: "text-muted-foreground", icon: Clock },
        active: { bg: "bg-green-500/10", text: "text-green-500", icon: CheckCircle2 },
        pending: { bg: "bg-yellow-500/10", text: "text-yellow-500", icon: Clock },
        banned: { bg: "bg-red-500/10", text: "text-red-500", icon: Ban },
    };
    const { bg, text, icon: Icon } = config[status as keyof typeof config] || config.healthy;
    return (
        <span className={`inline-flex items-center gap-1 px-2 py-0.5 text-xs rounded-full ${bg} ${text}`}>
            <Icon className="h-3 w-3" />
            {status}
        </span>
    );
}

function WipTag() {
    return (
        <span className="inline-flex items-center px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wider border border-amber-500/40 rounded bg-amber-500/10 text-amber-600 dark:text-amber-400">
            WIP
        </span>
    );
}

function getHealthStatus(status: ComponentStatus) {
    if (typeof status === "string") {
        const dotClassName = status === "healthy" ? "bg-green-500" : status === "disabled" ? "bg-muted-foreground" : "bg-red-500";

        return {
            badgeStatus: status,
            dotClassName,
            message: undefined,
        };
    }

    return {
        badgeStatus: "degraded",
        dotClassName: "bg-yellow-500",
        message: status.degraded,
    };
}

function formatLatency(latency: number | null) {
    if (latency === null) {
        return "—";
    }

    return `${latency}ms`;
}

function getUserStatus(user: AdminUser): AdminUserStatus {
    if (user.disabled) {
        return "disabled";
    }

    if (user.verifiedAt !== null) {
        return "active";
    }

    return "pending";
}

function formatUserCreatedAt(user: AdminUser) {
    return formatDistanceToNow(uuidToDate(user.id), { addSuffix: true });
}

function AuditTableRow({ event }: { event: EventResponse }) {
    const [expanded, setExpanded] = useState(false);
    const hasPayload = Object.keys(event.payload).length > 0;
    const hasExpandable = hasPayload || !!event.userAgent;

    return (
        <>
            <tr className="border-t border-border first:border-t-0 hover:bg-accent/30 transition-colors">
                <td className="px-4 py-3 font-mono text-xs">{event.type}</td>
                <td className="px-4 py-3 text-muted-foreground">{event.actorUsername ?? "system"}</td>
                <td className="px-4 py-3 text-muted-foreground font-mono text-xs">{event.subjectName ?? "—"}</td>
                <td className="px-4 py-3 text-muted-foreground font-mono text-xs">{event.ipAddress ?? "—"}</td>
                <td className="px-4 py-3 text-muted-foreground font-mono text-xs">
                    {event.traceId ? (
                        <TooltipProvider>
                            <Tooltip>
                                <TooltipTrigger asChild>
                                    <span className="cursor-default">{event.traceId.slice(0, 8)}</span>
                                </TooltipTrigger>
                                <TooltipContent>
                                    <p className="font-mono">{event.traceId}</p>
                                </TooltipContent>
                            </Tooltip>
                        </TooltipProvider>
                    ) : (
                        "—"
                    )}
                </td>
                <td className="px-4 py-3 text-muted-foreground">{formatDistanceToNow(uuidToDate(event.id), { addSuffix: true })}</td>
                <td className="px-4 py-3 text-right">
                    {hasExpandable && (
                        <button
                            onClick={() => setExpanded((v) => !v)}
                            className="p-1 rounded hover:bg-accent transition-colors text-muted-foreground"
                            title={expanded ? "Hide details" : "Show details"}
                        >
                            {expanded ? <ChevronUp className="h-3.5 w-3.5" /> : <ChevronDown className="h-3.5 w-3.5" />}
                        </button>
                    )}
                </td>
            </tr>
            {expanded && (
                <tr className="border-t border-border bg-secondary/20">
                    <td colSpan={7} className="px-4 pt-2 pb-3 space-y-1.5">
                        {event.userAgent && <p className="text-xs text-muted-foreground font-mono break-all">{event.userAgent}</p>}
                        {hasPayload && (
                            <pre className="text-xs font-mono bg-secondary/50 rounded p-2 overflow-x-auto whitespace-pre-wrap break-words max-h-48 overflow-y-auto">
                                {JSON.stringify(event.payload, null, 2)}
                            </pre>
                        )}
                    </td>
                </tr>
            )}
        </>
    );
}

export default function AdminDashboardPage() {
    const [activeSection, setActiveSection] = useState("dashboard");
    const instanceConfig = useInstanceConfig();
    const { data: stats } = useSWR<InstanceStats>("/api/admin/stats", jsonFetcher);
    const usersKey =
        activeSection === "dashboard"
            ? "/api/admin/users?sort=newest&limit=4"
            : activeSection === "users"
              ? "/api/admin/users?sort=newest"
              : null;
    const { data: adminUsers, isLoading: areUsersLoading, error: usersError } = useSWR<AdminUser[]>(usersKey, jsonFetcher);
    const { data: auditEvents, isLoading: isAuditLoading } = useSWR<EventResponse[] | null>(
        activeSection === "dashboard" ? "/api/admin/audit-log?limit=5" : activeSection === "audit-log" ? "/api/admin/audit-log" : null,
        authFetcher
    );
    const {
        data: health,
        isLoading: isHealthLoading,
        error: healthError,
        mutate: refreshHealth,
        isValidating: isHealthRefreshing,
    } = useSWR<InstanceHealth>("/api/admin/health", jsonFetcher);
    const showHealthLoading = isHealthLoading || (!health && !healthError);
    const showHealthRefreshing = isHealthRefreshing && !showHealthLoading;

    return (
        <div className="min-h-screen bg-background flex flex-col">
            <TopBar
                breadcrumb={[{ label: "Admin" }]}
                search={{ placeholder: "Search users, repositories, settings..." }}
                navLinks={[{ label: "Back to GitArena", href: "/", icon: <Compass className="h-[18px] w-[18px]" /> }]}
            />

            <div className="flex-1 flex">
                <aside className="w-64 border-r border-border shrink-0 overflow-y-auto">
                    <nav className="p-3 space-y-6">
                        {adminSections.map((section) => (
                            <div key={section.title}>
                                <h3 className="px-3 mb-2 text-xs font-medium uppercase tracking-wider text-muted-foreground">
                                    {section.title}
                                </h3>
                                <div className="space-y-1">
                                    {section.items.map((item) => (
                                        <button
                                            key={item.id}
                                            onClick={() => setActiveSection(item.id)}
                                            className={`w-full flex items-center gap-2.5 px-3 py-2 text-sm rounded-md transition-colors ${
                                                activeSection === item.id
                                                    ? "bg-accent text-foreground"
                                                    : "text-muted-foreground hover:text-foreground hover:bg-accent/50"
                                            }`}
                                        >
                                            <item.icon className="h-4 w-4" />
                                            {item.label}
                                        </button>
                                    ))}
                                </div>
                            </div>
                        ))}
                    </nav>
                </aside>

                <main className="flex-1 overflow-y-auto p-6">
                    {activeSection === "dashboard" && (
                        <div className="space-y-6">
                            <div className="grid grid-cols-4 gap-4">
                                <div className="p-5 rounded-lg bg-card border border-border">
                                    <div className="flex items-center justify-between">
                                        <Users className="h-5 w-5 text-muted-foreground" />
                                    </div>
                                    <div className="mt-3">
                                        {stats ? (
                                            <>
                                                <div className="text-3xl font-semibold">{stats.users}</div>
                                                <div className="text-sm text-muted-foreground">Users</div>
                                            </>
                                        ) : (
                                            <>
                                                <Skeleton className="h-9 w-24 mb-1" />
                                                <Skeleton className="h-4 w-20" />
                                            </>
                                        )}
                                    </div>
                                </div>
                                <div className="p-5 rounded-lg bg-card border border-border">
                                    <div className="flex items-center justify-between">
                                        <Globe className="h-5 w-5 text-muted-foreground" />
                                    </div>
                                    <div className="mt-3">
                                        {stats ? (
                                            <>
                                                <div className="text-3xl font-semibold">{stats.orgs}</div>
                                                <div className="text-sm text-muted-foreground">Organizations</div>
                                            </>
                                        ) : (
                                            <>
                                                <Skeleton className="h-9 w-24 mb-1" />
                                                <Skeleton className="h-4 w-20" />
                                            </>
                                        )}
                                    </div>
                                </div>
                                <div className="p-5 rounded-lg bg-card border border-border">
                                    <div className="flex items-center justify-between">
                                        <GitBranch className="h-5 w-5 text-muted-foreground" />
                                    </div>
                                    <div className="mt-3">
                                        {stats ? (
                                            <>
                                                <div className="text-3xl font-semibold">{stats.repositories}</div>
                                                <div className="text-sm text-muted-foreground">Repositories</div>
                                            </>
                                        ) : (
                                            <>
                                                <Skeleton className="h-9 w-24 mb-1" />
                                                <Skeleton className="h-4 w-20" />
                                            </>
                                        )}
                                    </div>
                                </div>
                                <div className="p-5 rounded-lg bg-card border border-border">
                                    <div className="flex items-center justify-between">
                                        <HardDrive className="h-5 w-5 text-muted-foreground" />
                                        {stats && (
                                            <span className="text-xs text-muted-foreground">
                                                {Math.round((stats.usedSpace / stats.totalSpace) * 100)}%
                                            </span>
                                        )}
                                    </div>
                                    <div className="mt-3">
                                        {stats ? (
                                            <>
                                                <div className="text-3xl font-semibold">{(stats.usedSpace / 1073741824).toFixed(1)} GB</div>
                                                <div className="text-sm text-muted-foreground">
                                                    of {(stats.totalSpace / 1073741824).toFixed(1)} GB
                                                </div>
                                            </>
                                        ) : (
                                            <>
                                                <Skeleton className="h-9 w-24 mb-1" />
                                                <Skeleton className="h-4 w-20" />
                                            </>
                                        )}
                                    </div>
                                    <div className="mt-2 h-1.5 bg-secondary rounded-full overflow-hidden">
                                        <div
                                            className="h-full bg-blue-500 rounded-full"
                                            style={{ width: stats ? `${(stats.usedSpace / stats.totalSpace) * 100}%` : "0%" }}
                                        />
                                    </div>
                                </div>
                            </div>

                            <div className="grid grid-cols-2 gap-6">
                                <div className="rounded-lg border border-border overflow-hidden">
                                    <div className="flex items-center justify-between px-4 py-3 bg-card border-b border-border">
                                        <h3 className="text-sm font-medium flex items-center gap-2">
                                            <Server className="h-4 w-4 text-muted-foreground" />
                                            System Health
                                        </h3>
                                        <Button
                                            variant="ghost"
                                            size="sm"
                                            className="h-7 px-2 gap-1"
                                            disabled={showHealthRefreshing}
                                            onClick={() => {
                                                void refreshHealth();
                                            }}
                                        >
                                            <RefreshCw className={`h-3.5 w-3.5 ${showHealthRefreshing ? "animate-spin" : ""}`} />
                                            Refresh
                                        </Button>
                                    </div>
                                    <div className="divide-y divide-border/50">
                                        {showHealthLoading ? (
                                            Array.from({ length: 4 }).map((_, index) => (
                                                <div key={index} className="flex items-center justify-between px-4 py-3">
                                                    <div className="flex items-center gap-3">
                                                        <Skeleton className="h-2 w-2 rounded-full" />
                                                        <Skeleton className="h-4 w-28" />
                                                    </div>
                                                    <div className="flex items-center gap-3">
                                                        <Skeleton className="h-4 w-10" />
                                                        <Skeleton className="h-5 w-20 rounded-full" />
                                                    </div>
                                                </div>
                                            ))
                                        ) : healthError ? (
                                            <div className="px-4 py-3 text-sm text-red-500">Unable to load system health.</div>
                                        ) : health?.components.length ? (
                                            health.components.map((component) => {
                                                const healthStatus = getHealthStatus(component.status);

                                                return (
                                                    <div key={component.name} className="flex items-center justify-between px-4 py-3">
                                                        <div className="flex items-center gap-3">
                                                            <div className={`w-2 h-2 rounded-full ${healthStatus.dotClassName}`} />
                                                            <div>
                                                                <span className="text-sm">{component.name}</span>
                                                                {healthStatus.message && (
                                                                    <div className="text-xs text-muted-foreground">
                                                                        {healthStatus.message}
                                                                    </div>
                                                                )}
                                                            </div>
                                                        </div>
                                                        <div className="flex items-center gap-3">
                                                            <span className="text-xs text-muted-foreground font-mono">
                                                                {formatLatency(component.latency)}
                                                            </span>
                                                            <StatusBadge status={healthStatus.badgeStatus} />
                                                        </div>
                                                    </div>
                                                );
                                            })
                                        ) : (
                                            <div className="px-4 py-3 text-sm text-muted-foreground">No health components reported.</div>
                                        )}
                                    </div>
                                </div>

                                <div className="rounded-lg border border-border overflow-hidden">
                                    <div className="flex items-center justify-between px-4 py-3 bg-card border-b border-border">
                                        <h3 className="text-sm font-medium flex items-center gap-2">
                                            <UserPlus className="h-4 w-4 text-muted-foreground" />
                                            Recent Users
                                        </h3>
                                        <button
                                            type="button"
                                            onClick={() => setActiveSection("users")}
                                            className="text-xs text-muted-foreground hover:text-foreground"
                                        >
                                            View all
                                        </button>
                                    </div>
                                    <div className="divide-y divide-border/50">
                                        {areUsersLoading ? (
                                            Array.from({ length: 4 }).map((_, index) => (
                                                <div key={index} className="flex items-center justify-between px-4 py-3">
                                                    <div className="flex items-center gap-3">
                                                        <Skeleton className="h-8 w-8 rounded-full" />
                                                        <div className="space-y-1">
                                                            <Skeleton className="h-4 w-24" />
                                                            <Skeleton className="h-3 w-36" />
                                                        </div>
                                                    </div>
                                                    <div className="flex items-center gap-3">
                                                        <Skeleton className="h-3 w-20" />
                                                        <Skeleton className="h-5 w-16 rounded-full" />
                                                        <Skeleton className="h-7 w-7 rounded-md" />
                                                    </div>
                                                </div>
                                            ))
                                        ) : usersError ? (
                                            <div className="px-4 py-3 text-sm text-red-500">Unable to load users.</div>
                                        ) : adminUsers?.length ? (
                                            adminUsers.map((user) => {
                                                const status = getUserStatus(user);

                                                return (
                                                    <div
                                                        key={user.id}
                                                        className="flex items-center justify-between px-4 py-3 hover:bg-accent/30 transition-colors"
                                                    >
                                                        <div className="flex items-center gap-3">
                                                            <div className="flex h-8 w-8 items-center justify-center rounded-full bg-secondary text-sm font-medium">
                                                                {user.username.charAt(0).toUpperCase()}
                                                            </div>
                                                            <div>
                                                                <div className="text-sm font-medium">{user.username}</div>
                                                                <div className="text-xs text-muted-foreground">{user.email}</div>
                                                            </div>
                                                        </div>
                                                        <div className="flex items-center gap-3">
                                                            <span className="text-xs text-muted-foreground">
                                                                {formatUserCreatedAt(user)}
                                                            </span>
                                                            <StatusBadge status={status} />
                                                            <DropdownMenu>
                                                                <DropdownMenuTrigger asChild>
                                                                    <Button variant="ghost" size="sm" className="h-7 w-7 p-0">
                                                                        <MoreHorizontal className="h-4 w-4" />
                                                                    </Button>
                                                                </DropdownMenuTrigger>
                                                                <DropdownMenuContent align="end">
                                                                    <DropdownMenuItem asChild>
                                                                        <Link href={`/${user.username}`}>View profile</Link>
                                                                    </DropdownMenuItem>
                                                                    <DropdownMenuItem>Edit user</DropdownMenuItem>
                                                                    <DropdownMenuSeparator />
                                                                    <DropdownMenuItem className="text-yellow-500">
                                                                        Suspend user
                                                                    </DropdownMenuItem>
                                                                    <DropdownMenuItem className="text-red-500">Ban user</DropdownMenuItem>
                                                                </DropdownMenuContent>
                                                            </DropdownMenu>
                                                        </div>
                                                    </div>
                                                );
                                            })
                                        ) : (
                                            <div className="px-4 py-3 text-sm text-muted-foreground">No users found.</div>
                                        )}
                                    </div>
                                </div>
                            </div>

                            <div className="rounded-lg border border-border overflow-hidden">
                                <div className="flex items-center justify-between px-4 py-3 bg-card border-b border-border">
                                    <h3 className="text-sm font-medium flex items-center gap-2">
                                        <FileText className="h-4 w-4 text-muted-foreground" />
                                        Recent Audit Log
                                    </h3>
                                    <button
                                        type="button"
                                        onClick={() => setActiveSection("audit-log")}
                                        className="text-xs text-muted-foreground hover:text-foreground"
                                    >
                                        View all
                                    </button>
                                </div>
                                <div className="divide-y divide-border/50">
                                    {isAuditLoading ? (
                                        Array.from({ length: 3 }).map((_, index) => (
                                            <div key={index} className="h-12 px-4 py-3 animate-pulse bg-secondary/30" />
                                        ))
                                    ) : !auditEvents || auditEvents.length === 0 ? (
                                        <div className="px-4 py-3 text-sm text-muted-foreground">No audit events yet.</div>
                                    ) : (
                                        auditEvents.map((event) => (
                                            <div key={event.id} className="flex items-center justify-between px-4 py-3">
                                                <div className="flex items-center gap-3">
                                                    <code className="px-2 py-0.5 text-xs bg-secondary rounded font-mono">{event.type}</code>
                                                    <span className="text-sm">
                                                        <span className="font-medium">{event.actorUsername ?? "system"}</span>
                                                        <span className="text-muted-foreground"> → </span>
                                                        <span className="font-medium">{event.subjectName ?? "—"}</span>
                                                    </span>
                                                </div>
                                                <span className="text-xs text-muted-foreground shrink-0">
                                                    {formatDistanceToNow(uuidToDate(event.id), { addSuffix: true })}
                                                </span>
                                            </div>
                                        ))
                                    )}
                                </div>
                            </div>

                            <div className="flex items-center gap-4 p-4 rounded-lg bg-card border border-border">
                                <span className="text-sm text-muted-foreground">Quick actions:</span>
                                <WipTag />
                                <Button variant="secondary" size="sm" className="gap-2">
                                    <Download className="h-4 w-4" />
                                    Create Backup
                                </Button>
                                <Button variant="secondary" size="sm" className="gap-2">
                                    <RefreshCw className="h-4 w-4" />
                                    Clear Cache
                                </Button>
                                <Button variant="secondary" size="sm" className="gap-2">
                                    <Mail className="h-4 w-4" />
                                    Test Email
                                </Button>
                                <div className="flex-1" />
                                <div className="text-xs text-muted-foreground">
                                    GitArena{instanceConfig?.version ? ` v${instanceConfig.version}` : ""}
                                </div>
                            </div>
                        </div>
                    )}

                    {activeSection === "users" && (
                        <div className="space-y-6">
                            <div className="flex items-center justify-between">
                                <div>
                                    <h1 className="text-2xl font-semibold">Users</h1>
                                    <p className="text-muted-foreground">Manage all users on this instance</p>
                                </div>
                                <div className="flex items-center gap-3">
                                    <div className="relative">
                                        <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                                        <input
                                            type="text"
                                            placeholder="Search users..."
                                            className="h-9 pl-9 pr-4 bg-card border border-border rounded-md text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
                                        />
                                    </div>
                                    <Button className="gap-2">
                                        <Plus className="h-4 w-4" />
                                        Add User
                                    </Button>
                                </div>
                            </div>

                            <div className="rounded-lg border border-border overflow-hidden">
                                <table className="w-full">
                                    <thead className="bg-card border-b border-border">
                                        <tr>
                                            <th className="text-left px-4 py-3 text-xs font-medium uppercase tracking-wider text-muted-foreground">
                                                User
                                            </th>
                                            <th className="text-left px-4 py-3 text-xs font-medium uppercase tracking-wider text-muted-foreground">
                                                Primary email
                                            </th>
                                            <th className="text-left px-4 py-3 text-xs font-medium uppercase tracking-wider text-muted-foreground">
                                                Created
                                            </th>
                                            <th className="text-left px-4 py-3 text-xs font-medium uppercase tracking-wider text-muted-foreground">
                                                Status
                                            </th>
                                            <th className="text-left px-4 py-3 text-xs font-medium uppercase tracking-wider text-muted-foreground">
                                                Admin
                                            </th>
                                            <th className="text-right px-4 py-3 text-xs font-medium uppercase tracking-wider text-muted-foreground">
                                                Actions
                                            </th>
                                        </tr>
                                    </thead>
                                    <tbody className="divide-y divide-border/50">
                                        {areUsersLoading ? (
                                            Array.from({ length: 6 }).map((_, index) => (
                                                <tr key={index}>
                                                    <td className="px-4 py-3">
                                                        <div className="flex items-center gap-3">
                                                            <Skeleton className="h-8 w-8 rounded-full" />
                                                            <Skeleton className="h-4 w-24" />
                                                        </div>
                                                    </td>
                                                    <td className="px-4 py-3">
                                                        <Skeleton className="h-4 w-40" />
                                                    </td>
                                                    <td className="px-4 py-3">
                                                        <Skeleton className="h-4 w-24" />
                                                    </td>
                                                    <td className="px-4 py-3">
                                                        <Skeleton className="h-5 w-16 rounded-full" />
                                                    </td>
                                                    <td className="px-4 py-3">
                                                        <Skeleton className="h-4 w-4 rounded" />
                                                    </td>
                                                    <td className="px-4 py-3">
                                                        <div className="flex justify-end">
                                                            <Skeleton className="h-7 w-7 rounded-md" />
                                                        </div>
                                                    </td>
                                                </tr>
                                            ))
                                        ) : usersError ? (
                                            <tr>
                                                <td colSpan={7} className="px-4 py-3 text-sm text-red-500">
                                                    Unable to load users.
                                                </td>
                                            </tr>
                                        ) : adminUsers?.length ? (
                                            adminUsers.map((user) => {
                                                const status = getUserStatus(user);

                                                return (
                                                    <tr key={user.id} className="hover:bg-accent/30 transition-colors">
                                                        <td className="px-4 py-3">
                                                            <div className="flex items-center gap-3">
                                                                <div className="flex h-8 w-8 items-center justify-center rounded-full bg-secondary text-sm font-medium">
                                                                    {user.username.charAt(0).toUpperCase()}
                                                                </div>
                                                                <span className="font-medium">{user.username}</span>
                                                            </div>
                                                        </td>
                                                        <td className="px-4 py-3 text-sm text-muted-foreground">{user.email}</td>
                                                        <td className="px-4 py-3 text-sm text-muted-foreground">
                                                            {formatUserCreatedAt(user)}
                                                        </td>
                                                        <td className="px-4 py-3">
                                                            <StatusBadge status={status} />
                                                        </td>
                                                        <td className="px-4 py-3">
                                                            <Checkbox checked={user.admin} aria-label={`${user.username} admin status`} />
                                                        </td>
                                                        <td className="px-4 py-3 text-right">
                                                            <DropdownMenu>
                                                                <DropdownMenuTrigger asChild>
                                                                    <Button variant="ghost" size="sm" className="h-7 w-7 p-0">
                                                                        <MoreHorizontal className="h-4 w-4" />
                                                                    </Button>
                                                                </DropdownMenuTrigger>
                                                                <DropdownMenuContent align="end">
                                                                    <DropdownMenuItem>Edit</DropdownMenuItem>
                                                                    <DropdownMenuItem>View activity</DropdownMenuItem>
                                                                    <DropdownMenuSeparator />
                                                                    <DropdownMenuItem className="text-yellow-500">Suspend</DropdownMenuItem>
                                                                    <DropdownMenuItem className="text-red-500">Delete</DropdownMenuItem>
                                                                </DropdownMenuContent>
                                                            </DropdownMenu>
                                                        </td>
                                                    </tr>
                                                );
                                            })
                                        ) : (
                                            <tr>
                                                <td colSpan={7} className="px-4 py-3 text-sm text-muted-foreground">
                                                    No users found.
                                                </td>
                                            </tr>
                                        )}
                                    </tbody>
                                </table>
                            </div>
                        </div>
                    )}

                    {activeSection === "audit-log" && (
                        <div className="space-y-6">
                            <div>
                                <h1 className="text-2xl font-semibold">Audit Log</h1>
                                <p className="text-sm text-muted-foreground mt-1">Security-relevant actions performed on this instance.</p>
                            </div>
                            <div className="flex items-center gap-3">
                                <div className="relative flex-1 max-w-xs">
                                    <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground pointer-events-none" />
                                    <input
                                        placeholder="Search events…"
                                        className="w-full h-9 pl-9 pr-3 bg-card border border-border rounded-md text-sm focus:outline-none focus:ring-1 focus:ring-ring"
                                    />
                                </div>
                                <select className="h-9 px-3 bg-card border border-border rounded-md text-sm focus:outline-none">
                                    <option>All classes</option>
                                    <option value="security">Security</option>
                                    <option value="activity">Activity</option>
                                </select>
                                <Button variant="outline" size="sm" className="gap-2 ml-auto">
                                    <Download className="h-4 w-4" />
                                    Export CSV
                                </Button>
                            </div>
                            <div className="border border-border rounded-lg overflow-hidden">
                                <table className="w-full text-sm">
                                    <thead className="border-b border-border bg-secondary/30">
                                        <tr>
                                            {["Action", "Actor", "Target", "IP", "Trace", "Time", ""].map((h) => (
                                                <th key={h} className="px-4 py-3 text-left text-xs font-medium text-muted-foreground">
                                                    {h}
                                                </th>
                                            ))}
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {isAuditLoading ? (
                                            Array.from({ length: 8 }).map((_, index) => (
                                                <tr key={index}>
                                                    <td colSpan={7} className="px-4 py-3">
                                                        <div className="h-4 bg-secondary/50 rounded animate-pulse" />
                                                    </td>
                                                </tr>
                                            ))
                                        ) : !auditEvents || auditEvents.length === 0 ? (
                                            <tr>
                                                <td colSpan={7} className="px-4 py-8 text-center text-sm text-muted-foreground">
                                                    No audit events yet.
                                                </td>
                                            </tr>
                                        ) : (
                                            auditEvents.map((event) => <AuditTableRow key={event.id} event={event} />)
                                        )}
                                    </tbody>
                                </table>
                            </div>
                        </div>
                    )}

                    {!["dashboard", "users", "audit-log"].includes(activeSection) && (
                        <div className="flex items-center justify-center h-full">
                            <div className="text-center">
                                <Settings className="h-12 w-12 text-muted-foreground mx-auto mb-4" />
                                <h2 className="text-xl font-semibold mb-2">
                                    {adminSections.flatMap((s) => s.items).find((i) => i.id === activeSection)?.label}
                                </h2>
                                <p className="text-muted-foreground">This section is under construction</p>
                            </div>
                        </div>
                    )}
                </main>
            </div>
        </div>
    );
}

"use client";

import { useState } from "react";
import useSWR, { mutate } from "swr";
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
import { DeleteUserDialog } from "@/components/delete-user-dialog";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import { useInstanceConfig } from "@/components/instance-config-provider";
import { jsonFetcher, authFetcher } from "@/lib/fetchers";
import { cn, uuidToDate } from "@/lib/utils";
import type { EventResponse } from "@/components/activity-event";
import { UserAvatar } from "@/components/ui/user-avatar";
import { TokenManager } from "@/components/token-manager";
import { InputGroup, InputGroupAddon, InputGroupInput } from "@/components/ui/input-group";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import { Progress } from "@/components/ui/progress";
import { WipBadge } from "@/components/wip-badge";
import { Badge } from "@/components/ui/badge";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";

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
        healthy: { variant: "success", icon: CheckCircle2 },
        degraded: { variant: "warning", icon: AlertTriangle },
        warning: { variant: "warning", icon: AlertTriangle },
        error: { variant: "destructive", icon: XCircle },
        unhealthy: { variant: "destructive", icon: XCircle },
        disabled: { variant: "secondary", icon: Clock },
        active: { variant: "success", icon: CheckCircle2 },
        pending: { variant: "warning", icon: Clock },
        banned: { variant: "destructive", icon: Ban },
    } as const;
    const { variant, icon: Icon } = config[status as keyof typeof config] || config.healthy;
    return (
        <Badge variant={variant} className={cn("rounded-full font-normal", variant === "secondary" && "text-muted-foreground")}>
            <Icon />
            {status}
        </Badge>
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
        <Collapsible asChild open={expanded} onOpenChange={setExpanded}>
            <TableBody className="border-t border-border first-of-type:border-t-0">
                <TableRow className="hover:bg-accent/30 transition-colors">
                    <TableCell className="px-4 py-3 font-mono text-xs">{event.type}</TableCell>
                    <TableCell className="px-4 py-3 text-muted-foreground">{event.actorUsername ?? "system"}</TableCell>
                    <TableCell className="px-4 py-3 text-muted-foreground font-mono text-xs">{event.subjectName ?? "—"}</TableCell>
                    <TableCell className="px-4 py-3 text-muted-foreground font-mono text-xs">{event.ipAddress ?? "—"}</TableCell>
                    <TableCell className="px-4 py-3 text-muted-foreground font-mono text-xs">
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
                    </TableCell>
                    <TableCell className="px-4 py-3 text-muted-foreground">
                        {formatDistanceToNow(uuidToDate(event.id), { addSuffix: true })}
                    </TableCell>
                    <TableCell className="px-4 py-3 text-right">
                        {hasExpandable && (
                            <CollapsibleTrigger asChild>
                                <button
                                    className="p-1 rounded hover:bg-accent transition-colors text-muted-foreground"
                                    title={expanded ? "Hide details" : "Show details"}
                                >
                                    {expanded ? <ChevronUp className="h-3.5 w-3.5" /> : <ChevronDown className="h-3.5 w-3.5" />}
                                </button>
                            </CollapsibleTrigger>
                        )}
                    </TableCell>
                </TableRow>
                <CollapsibleContent asChild>
                    <TableRow className="border-t border-border bg-secondary/20">
                        <TableCell colSpan={7} className="px-4 pt-2 pb-3 space-y-1.5 whitespace-normal">
                            {event.userAgent && <p className="text-xs text-muted-foreground font-mono break-all">{event.userAgent}</p>}
                            {hasPayload && (
                                <pre className="text-xs font-mono bg-secondary/50 rounded p-2 overflow-x-auto whitespace-pre-wrap break-words max-h-48 overflow-y-auto">
                                    {JSON.stringify(event.payload, null, 2)}
                                </pre>
                            )}
                        </TableCell>
                    </TableRow>
                </CollapsibleContent>
            </TableBody>
        </Collapsible>
    );
}

export default function AdminDashboardPage() {
    const [activeSection, setActiveSection] = useState("dashboard");
    const [userToDelete, setUserToDelete] = useState<AdminUser | null>(null);
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
                                    <Progress
                                        value={stats ? (stats.usedSpace / stats.totalSpace) * 100 : 0}
                                        className="mt-2 h-1.5 bg-secondary *:data-[slot=progress-indicator]:bg-info"
                                    />
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
                                                            <UserAvatar userId={user.id} username={user.username} size="lg" />
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
                                            <Skeleton key={index} className="h-12 px-4 py-3 rounded-none" />
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
                                <WipBadge />
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
                                    <InputGroup>
                                        <InputGroupAddon>
                                            <Search />
                                        </InputGroupAddon>
                                        <InputGroupInput placeholder="Search users..." />
                                    </InputGroup>
                                    <Button className="gap-2">
                                        <Plus className="h-4 w-4" />
                                        Add User
                                    </Button>
                                </div>
                            </div>

                            <div className="rounded-lg border border-border overflow-hidden">
                                <Table>
                                    <TableHeader className="bg-card border-b border-border">
                                        <TableRow>
                                            <TableHead className="text-left px-4 py-3 text-xs font-medium uppercase tracking-wider text-muted-foreground">
                                                User
                                            </TableHead>
                                            <TableHead className="text-left px-4 py-3 text-xs font-medium uppercase tracking-wider text-muted-foreground">
                                                Primary email
                                            </TableHead>
                                            <TableHead className="text-left px-4 py-3 text-xs font-medium uppercase tracking-wider text-muted-foreground">
                                                Created
                                            </TableHead>
                                            <TableHead className="text-left px-4 py-3 text-xs font-medium uppercase tracking-wider text-muted-foreground">
                                                Status
                                            </TableHead>
                                            <TableHead className="text-left px-4 py-3 text-xs font-medium uppercase tracking-wider text-muted-foreground">
                                                Admin
                                            </TableHead>
                                            <TableHead className="text-right px-4 py-3 text-xs font-medium uppercase tracking-wider text-muted-foreground">
                                                Actions
                                            </TableHead>
                                        </TableRow>
                                    </TableHeader>
                                    <TableBody>
                                        {areUsersLoading ? (
                                            Array.from({ length: 6 }).map((_, index) => (
                                                <TableRow key={index}>
                                                    <TableCell className="px-4 py-3">
                                                        <div className="flex items-center gap-3">
                                                            <Skeleton className="h-8 w-8 rounded-full" />
                                                            <Skeleton className="h-4 w-24" />
                                                        </div>
                                                    </TableCell>
                                                    <TableCell className="px-4 py-3">
                                                        <Skeleton className="h-4 w-40" />
                                                    </TableCell>
                                                    <TableCell className="px-4 py-3">
                                                        <Skeleton className="h-4 w-24" />
                                                    </TableCell>
                                                    <TableCell className="px-4 py-3">
                                                        <Skeleton className="h-5 w-16 rounded-full" />
                                                    </TableCell>
                                                    <TableCell className="px-4 py-3">
                                                        <Skeleton className="h-4 w-4 rounded" />
                                                    </TableCell>
                                                    <TableCell className="px-4 py-3">
                                                        <div className="flex justify-end">
                                                            <Skeleton className="h-7 w-7 rounded-md" />
                                                        </div>
                                                    </TableCell>
                                                </TableRow>
                                            ))
                                        ) : usersError ? (
                                            <TableRow>
                                                <TableCell colSpan={7} className="px-4 py-3 text-sm text-red-500">
                                                    Unable to load users.
                                                </TableCell>
                                            </TableRow>
                                        ) : adminUsers?.length ? (
                                            adminUsers.map((user) => {
                                                const status = getUserStatus(user);

                                                return (
                                                    <TableRow key={user.id} className="hover:bg-accent/30 transition-colors">
                                                        <TableCell className="px-4 py-3">
                                                            <div className="flex items-center gap-3">
                                                                <UserAvatar userId={user.id} username={user.username} size="lg" />
                                                                <span className="font-medium">{user.username}</span>
                                                            </div>
                                                        </TableCell>
                                                        <TableCell className="px-4 py-3 text-sm text-muted-foreground">
                                                            {user.email}
                                                        </TableCell>
                                                        <TableCell className="px-4 py-3 text-sm text-muted-foreground">
                                                            {formatUserCreatedAt(user)}
                                                        </TableCell>
                                                        <TableCell className="px-4 py-3">
                                                            <StatusBadge status={status} />
                                                        </TableCell>
                                                        <TableCell className="px-4 py-3">
                                                            <Checkbox checked={user.admin} aria-label={`${user.username} admin status`} />
                                                        </TableCell>
                                                        <TableCell className="px-4 py-3 text-right">
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
                                                                    <DropdownMenuItem
                                                                        className="text-red-500"
                                                                        onSelect={() => setUserToDelete(user)}
                                                                    >
                                                                        Delete
                                                                    </DropdownMenuItem>
                                                                </DropdownMenuContent>
                                                            </DropdownMenu>
                                                        </TableCell>
                                                    </TableRow>
                                                );
                                            })
                                        ) : (
                                            <TableRow>
                                                <TableCell colSpan={7} className="px-4 py-3 text-sm text-muted-foreground">
                                                    No users found.
                                                </TableCell>
                                            </TableRow>
                                        )}
                                    </TableBody>
                                </Table>
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
                                <InputGroup className="flex-1 max-w-xs">
                                    <InputGroupAddon>
                                        <Search />
                                    </InputGroupAddon>
                                    <InputGroupInput placeholder="Search events…" />
                                </InputGroup>
                                <Select defaultValue="all">
                                    <SelectTrigger className="bg-card">
                                        <SelectValue />
                                    </SelectTrigger>
                                    <SelectContent>
                                        <SelectItem value="all">All classes</SelectItem>
                                        <SelectItem value="security">Security</SelectItem>
                                        <SelectItem value="activity">Activity</SelectItem>
                                    </SelectContent>
                                </Select>
                                <Button variant="outline" size="sm" className="gap-2 ml-auto">
                                    <Download className="h-4 w-4" />
                                    Export CSV
                                </Button>
                            </div>
                            <div className="border border-border rounded-lg overflow-hidden">
                                <Table>
                                    <TableHeader className="border-b border-border bg-secondary/30">
                                        <TableRow>
                                            {["Action", "Actor", "Target", "IP", "Trace", "Time", ""].map((h) => (
                                                <TableHead
                                                    key={h}
                                                    className="px-4 py-3 text-left text-xs font-medium text-muted-foreground"
                                                >
                                                    {h}
                                                </TableHead>
                                            ))}
                                        </TableRow>
                                    </TableHeader>
                                    {isAuditLoading ? (
                                        <TableBody>
                                            {Array.from({ length: 8 }).map((_, index) => (
                                                <TableRow key={index}>
                                                    <TableCell colSpan={7} className="px-4 py-3">
                                                        <Skeleton className="h-4 rounded" />
                                                    </TableCell>
                                                </TableRow>
                                            ))}
                                        </TableBody>
                                    ) : !auditEvents || auditEvents.length === 0 ? (
                                        <TableBody>
                                            <TableRow>
                                                <TableCell colSpan={7} className="px-4 py-8 text-center text-sm text-muted-foreground">
                                                    No audit events yet.
                                                </TableCell>
                                            </TableRow>
                                        </TableBody>
                                    ) : (
                                        auditEvents.map((event) => <AuditTableRow key={event.id} event={event} />)
                                    )}
                                </Table>
                            </div>
                        </div>
                    )}

                    {activeSection === "access-tokens" && (
                        <div className="max-w-3xl">
                            <TokenManager
                                owner={{ kind: "instance" }}
                                title="Access Tokens"
                                description="Instance-wide tokens and the runner tokens used to register CI runners that are not tied to a single owner."
                            />
                        </div>
                    )}

                    {!["dashboard", "users", "audit-log", "access-tokens"].includes(activeSection) && (
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

            {userToDelete && (
                <DeleteUserDialog
                    open
                    onOpenChange={(open) => !open && setUserToDelete(null)}
                    url={`/api/admin/users/${userToDelete.id}`}
                    username={userToDelete.username}
                    title={`Delete ${userToDelete.username}?`}
                    description="Their profile, repositories, SSH keys and access tokens will be permanently removed. Issues and comments they created on other repositories remain but are no longer attributed to them."
                    onDeleted={() => mutate((key) => typeof key === "string" && key.startsWith("/api/admin/"))}
                />
            )}
        </div>
    );
}

"use client";

import { useState } from "react";
import Link from "next/link";
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
} from "lucide-react";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
    DropdownMenuSeparator,
} from "@/components/ui/dropdown-menu";
import { TopBar } from "@/components/top-bar";

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

const systemStats = {
    users: { total: 1247, active: 892, new: 34 },
    repos: { total: 3456, public: 2890, private: 566 },
    storage: { used: "45.2 GB", total: "100 GB", percentage: 45.2 },
    uptime: "99.98%",
    version: "0.8.2",
    lastBackup: "2 hours ago",
};

const recentUsers = [
    { id: 1, username: "torvalds", email: "torvalds@example.com", createdAt: "2 hours ago", status: "active" },
    { id: 2, username: "gvanrossum", email: "guido@example.com", createdAt: "5 hours ago", status: "active" },
    { id: 3, username: "dhh", email: "dhh@example.com", createdAt: "1 day ago", status: "pending" },
    { id: 4, username: "wycats", email: "wycats@example.com", createdAt: "2 days ago", status: "active" },
];

const systemHealth = [
    { name: "Web Server", status: "healthy", latency: "12ms" },
    { name: "Database", status: "healthy", latency: "3ms" },
    { name: "Redis Cache", status: "healthy", latency: "1ms" },
    { name: "Object Storage", status: "healthy", latency: "45ms" },
    { name: "Background Workers", status: "warning", latency: "—" },
    { name: "Email Service", status: "healthy", latency: "89ms" },
];

const auditLog = [
    { id: 1, action: "user.created", actor: "system", target: "torvalds", time: "2 hours ago" },
    { id: 2, action: "repo.deleted", actor: "mellowagain", target: "old-project", time: "3 hours ago" },
    { id: 3, action: "settings.updated", actor: "mellowagain", target: "email.smtp_host", time: "5 hours ago" },
    { id: 4, action: "user.banned", actor: "mellowagain", target: "spammer123", time: "1 day ago" },
];

function StatusBadge({ status }: { status: string }) {
    const config = {
        healthy: { bg: "bg-green-500/10", text: "text-green-500", icon: CheckCircle2 },
        warning: { bg: "bg-yellow-500/10", text: "text-yellow-500", icon: AlertTriangle },
        error: { bg: "bg-red-500/10", text: "text-red-500", icon: XCircle },
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

export default function AdminDashboardPage() {
    const [activeSection, setActiveSection] = useState("dashboard");

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
                                        <span className="text-xs text-green-500">+{systemStats.users.new} new</span>
                                    </div>
                                    <div className="mt-3">
                                        <div className="text-3xl font-semibold">{systemStats.users.total.toLocaleString()}</div>
                                        <div className="text-sm text-muted-foreground">Total Users</div>
                                    </div>
                                </div>
                                <div className="p-5 rounded-lg bg-card border border-border">
                                    <div className="flex items-center justify-between">
                                        <GitBranch className="h-5 w-5 text-muted-foreground" />
                                    </div>
                                    <div className="mt-3">
                                        <div className="text-3xl font-semibold">{systemStats.repos.total.toLocaleString()}</div>
                                        <div className="text-sm text-muted-foreground">Repositories</div>
                                    </div>
                                </div>
                                <div className="p-5 rounded-lg bg-card border border-border">
                                    <div className="flex items-center justify-between">
                                        <HardDrive className="h-5 w-5 text-muted-foreground" />
                                        <span className="text-xs text-muted-foreground">{systemStats.storage.percentage}%</span>
                                    </div>
                                    <div className="mt-3">
                                        <div className="text-3xl font-semibold">{systemStats.storage.used}</div>
                                        <div className="text-sm text-muted-foreground">of {systemStats.storage.total}</div>
                                    </div>
                                    <div className="mt-2 h-1.5 bg-secondary rounded-full overflow-hidden">
                                        <div
                                            className="h-full bg-blue-500 rounded-full"
                                            style={{ width: `${systemStats.storage.percentage}%` }}
                                        />
                                    </div>
                                </div>
                                <div className="p-5 rounded-lg bg-card border border-border">
                                    <div className="flex items-center justify-between">
                                        <Activity className="h-5 w-5 text-muted-foreground" />
                                        <StatusBadge status="healthy" />
                                    </div>
                                    <div className="mt-3">
                                        <div className="text-3xl font-semibold">{systemStats.uptime}</div>
                                        <div className="text-sm text-muted-foreground">Uptime</div>
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
                                        <Button variant="ghost" size="sm" className="h-7 px-2 gap-1">
                                            <RefreshCw className="h-3.5 w-3.5" />
                                            Refresh
                                        </Button>
                                    </div>
                                    <div className="divide-y divide-border/50">
                                        {systemHealth.map((service) => (
                                            <div key={service.name} className="flex items-center justify-between px-4 py-3">
                                                <div className="flex items-center gap-3">
                                                    <div
                                                        className={`w-2 h-2 rounded-full ${
                                                            service.status === "healthy"
                                                                ? "bg-green-500"
                                                                : service.status === "warning"
                                                                  ? "bg-yellow-500"
                                                                  : "bg-red-500"
                                                        }`}
                                                    />
                                                    <span className="text-sm">{service.name}</span>
                                                </div>
                                                <div className="flex items-center gap-3">
                                                    <span className="text-xs text-muted-foreground font-mono">{service.latency}</span>
                                                    <StatusBadge status={service.status} />
                                                </div>
                                            </div>
                                        ))}
                                    </div>
                                </div>

                                <div className="rounded-lg border border-border overflow-hidden">
                                    <div className="flex items-center justify-between px-4 py-3 bg-card border-b border-border">
                                        <h3 className="text-sm font-medium flex items-center gap-2">
                                            <UserPlus className="h-4 w-4 text-muted-foreground" />
                                            Recent Users
                                        </h3>
                                        <Link href="#" className="text-xs text-muted-foreground hover:text-foreground">
                                            View all
                                        </Link>
                                    </div>
                                    <div className="divide-y divide-border/50">
                                        {recentUsers.map((user) => (
                                            <div
                                                key={user.id}
                                                className="flex items-center justify-between px-4 py-3 hover:bg-accent/30 transition-colors"
                                            >
                                                <div className="flex items-center gap-3">
                                                    <div className="flex h-8 w-8 items-center justify-center rounded-full bg-secondary text-sm font-medium">
                                                        {user.username[0].toUpperCase()}
                                                    </div>
                                                    <div>
                                                        <div className="text-sm font-medium">{user.username}</div>
                                                        <div className="text-xs text-muted-foreground">{user.email}</div>
                                                    </div>
                                                </div>
                                                <div className="flex items-center gap-3">
                                                    <span className="text-xs text-muted-foreground">{user.createdAt}</span>
                                                    <StatusBadge status={user.status} />
                                                    <DropdownMenu>
                                                        <DropdownMenuTrigger asChild>
                                                            <Button variant="ghost" size="sm" className="h-7 w-7 p-0">
                                                                <MoreHorizontal className="h-4 w-4" />
                                                            </Button>
                                                        </DropdownMenuTrigger>
                                                        <DropdownMenuContent align="end">
                                                            <DropdownMenuItem>View profile</DropdownMenuItem>
                                                            <DropdownMenuItem>Edit user</DropdownMenuItem>
                                                            <DropdownMenuSeparator />
                                                            <DropdownMenuItem className="text-yellow-500">Suspend user</DropdownMenuItem>
                                                            <DropdownMenuItem className="text-red-500">Ban user</DropdownMenuItem>
                                                        </DropdownMenuContent>
                                                    </DropdownMenu>
                                                </div>
                                            </div>
                                        ))}
                                    </div>
                                </div>
                            </div>

                            <div className="rounded-lg border border-border overflow-hidden">
                                <div className="flex items-center justify-between px-4 py-3 bg-card border-b border-border">
                                    <h3 className="text-sm font-medium flex items-center gap-2">
                                        <FileText className="h-4 w-4 text-muted-foreground" />
                                        Recent Audit Log
                                    </h3>
                                    <Link href="#" className="text-xs text-muted-foreground hover:text-foreground">
                                        View full log
                                    </Link>
                                </div>
                                <div className="divide-y divide-border/50">
                                    {auditLog.map((entry) => (
                                        <div key={entry.id} className="flex items-center justify-between px-4 py-3">
                                            <div className="flex items-center gap-3">
                                                <code className="px-2 py-0.5 text-xs bg-secondary rounded font-mono">{entry.action}</code>
                                                <span className="text-sm">
                                                    <span className="font-medium">{entry.actor}</span>
                                                    <span className="text-muted-foreground"> → </span>
                                                    <span className="font-medium">{entry.target}</span>
                                                </span>
                                            </div>
                                            <span className="text-xs text-muted-foreground">{entry.time}</span>
                                        </div>
                                    ))}
                                </div>
                            </div>

                            <div className="flex items-center gap-4 p-4 rounded-lg bg-card border border-border">
                                <span className="text-sm text-muted-foreground">Quick actions:</span>
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
                                    GitArena v{systemStats.version} • Last backup: {systemStats.lastBackup}
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
                                                Email
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
                                        {recentUsers.map((user) => (
                                            <tr key={user.id} className="hover:bg-accent/30 transition-colors">
                                                <td className="px-4 py-3">
                                                    <div className="flex items-center gap-3">
                                                        <div className="flex h-8 w-8 items-center justify-center rounded-full bg-secondary text-sm font-medium">
                                                            {user.username[0].toUpperCase()}
                                                        </div>
                                                        <span className="font-medium">{user.username}</span>
                                                    </div>
                                                </td>
                                                <td className="px-4 py-3 text-sm text-muted-foreground">{user.email}</td>
                                                <td className="px-4 py-3 text-sm text-muted-foreground">{user.createdAt}</td>
                                                <td className="px-4 py-3">
                                                    <StatusBadge status={user.status} />
                                                </td>
                                                <td className="px-4 py-3">
                                                    <input type="checkbox" className="rounded border-border" />
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
                                        ))}
                                    </tbody>
                                </table>
                            </div>
                        </div>
                    )}

                    {activeSection === "settings" && (
                        <div className="space-y-6 max-w-2xl">
                            <div>
                                <h1 className="text-2xl font-semibold">General Settings</h1>
                                <p className="text-muted-foreground">Configure your GitArena instance</p>
                            </div>

                            <div className="space-y-6">
                                <div className="rounded-lg border border-border overflow-hidden">
                                    <div className="px-4 py-3 bg-card border-b border-border">
                                        <h3 className="font-medium">Instance Information</h3>
                                    </div>
                                    <div className="p-4 space-y-4">
                                        <div>
                                            <label className="text-sm font-medium">Instance Name</label>
                                            <input
                                                type="text"
                                                defaultValue="GitArena"
                                                className="w-full mt-1.5 h-10 px-3 bg-card border border-border rounded-md"
                                            />
                                        </div>
                                        <div>
                                            <label className="text-sm font-medium">Instance URL</label>
                                            <input
                                                type="text"
                                                defaultValue="https://git.mari.zip"
                                                className="w-full mt-1.5 h-10 px-3 bg-card border border-border rounded-md"
                                            />
                                        </div>
                                        <div>
                                            <label className="text-sm font-medium">Description</label>
                                            <textarea
                                                defaultValue="A lightweight git hosting solution"
                                                className="w-full mt-1.5 px-3 py-2 bg-card border border-border rounded-md resize-none"
                                                rows={3}
                                            />
                                        </div>
                                    </div>
                                </div>

                                <div className="rounded-lg border border-border overflow-hidden">
                                    <div className="px-4 py-3 bg-card border-b border-border">
                                        <h3 className="font-medium">Registration</h3>
                                    </div>
                                    <div className="p-4 space-y-4">
                                        <div className="flex items-center justify-between">
                                            <div>
                                                <div className="font-medium">Allow public registration</div>
                                                <div className="text-sm text-muted-foreground">Anyone can create an account</div>
                                            </div>
                                            <button className="w-12 h-6 bg-blue-500 rounded-full relative">
                                                <span className="absolute right-1 top-1 w-4 h-4 bg-white rounded-full" />
                                            </button>
                                        </div>
                                        <div className="flex items-center justify-between">
                                            <div>
                                                <div className="font-medium">Require email confirmation</div>
                                                <div className="text-sm text-muted-foreground">Users must verify their email</div>
                                            </div>
                                            <button className="w-12 h-6 bg-blue-500 rounded-full relative">
                                                <span className="absolute right-1 top-1 w-4 h-4 bg-white rounded-full" />
                                            </button>
                                        </div>
                                        <div className="flex items-center justify-between">
                                            <div>
                                                <div className="font-medium">Allow SSO registration</div>
                                                <div className="text-sm text-muted-foreground">Users can sign up with OAuth providers</div>
                                            </div>
                                            <button className="w-12 h-6 bg-blue-500 rounded-full relative">
                                                <span className="absolute right-1 top-1 w-4 h-4 bg-white rounded-full" />
                                            </button>
                                        </div>
                                    </div>
                                </div>

                                <div className="rounded-lg border border-border overflow-hidden">
                                    <div className="px-4 py-3 bg-card border-b border-border">
                                        <h3 className="font-medium">Default Repository Settings</h3>
                                    </div>
                                    <div className="p-4 space-y-4">
                                        <div>
                                            <label className="text-sm font-medium">Default visibility</label>
                                            <select className="w-full mt-1.5 h-10 px-3 bg-card border border-border rounded-md">
                                                <option>Public</option>
                                                <option>Internal</option>
                                                <option>Private</option>
                                            </select>
                                        </div>
                                        <div>
                                            <label className="text-sm font-medium">Default branch name</label>
                                            <input
                                                type="text"
                                                defaultValue="main"
                                                className="w-full mt-1.5 h-10 px-3 bg-card border border-border rounded-md"
                                            />
                                        </div>
                                    </div>
                                </div>

                                <div className="flex justify-end">
                                    <Button>Save Settings</Button>
                                </div>
                            </div>
                        </div>
                    )}

                    {!["dashboard", "users", "settings"].includes(activeSection) && (
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

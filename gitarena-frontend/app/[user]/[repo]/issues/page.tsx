"use client";

import { useState, useRef, useEffect } from "react";
import Link from "next/link";
import { TopBar } from "@/components/top-bar";
import { Button } from "@/components/ui/button";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import {
    AlertCircle,
    GitMerge,
    Search,
    ChevronDown,
    Plus,
    Circle,
    CheckCircle2,
    MessageSquare,
    Inbox,
    CircleDot,
    MoreHorizontal,
    Code,
} from "lucide-react";

const repoData = {
    org: "mellowagain",
    name: "test",
    visibility: "public" as const,
};

type Priority = "urgent" | "high" | "medium" | "low" | "none";
type Status = "todo" | "in_progress" | "done" | "cancelled";

type Issue = {
    id: string;
    title: string;
    status: Status;
    priority: Priority;
    author: string;
    createdAt: string;
    updatedAt: string;
    comments: number;
    labels: { name: string; color: string }[];
    assignee?: string;
};

const issues: Issue[] = [
    {
        id: 42,
        title: "Add support for SSH key authentication",
        status: "in_progress",
        priority: "high",
        author: "mellowagain",
        createdAt: "2 days ago",
        updatedAt: "5h",
        comments: 8,
        labels: [
            { name: "enhancement", color: "#a2eeef" },
            { name: "component::auth", color: "#d73a4a" },
        ],
        assignee: "Mari",
    },
    {
        id: 41,
        title: "Repository cloning fails on large repositories",
        status: "todo",
        priority: "urgent",
        author: "contributor1",
        createdAt: "3 days ago",
        updatedAt: "1d",
        comments: 12,
        labels: [{ name: "bug", color: "#d73a4a" }],
    },
    {
        id: 40,
        title: "Implement webhook support for CI/CD integrations",
        status: "todo",
        priority: "medium",
        author: "devops_user",
        createdAt: "5 days ago",
        updatedAt: "2d",
        comments: 5,
        labels: [{ name: "feature", color: "#0e8a16" }],
        assignee: "mellowagain",
    },
    {
        id: 39,
        title: "Documentation for API endpoints",
        status: "todo",
        priority: "low",
        author: "docs_contributor",
        createdAt: "1 week ago",
        updatedAt: "3d",
        comments: 3,
        labels: [{ name: "documentation", color: "#0075ca" }],
    },
    {
        id: 38,
        title: "Fix memory leak in git pack parsing",
        status: "done",
        priority: "high",
        author: "mellowagain",
        createdAt: "2 weeks ago",
        updatedAt: "1w",
        comments: 15,
        labels: [{ name: "bug", color: "#d73a4a" }],
        assignee: "mellowagain",
    },
    {
        id: 37,
        title: "Add dark mode support",
        status: "done",
        priority: "medium",
        author: "ui_designer",
        createdAt: "3 weeks ago",
        updatedAt: "2w",
        comments: 22,
        labels: [{ name: "enhancement", color: "#a2eeef" }],
        assignee: "Mari",
    },
    {
        id: 36,
        title: "Migrate database schema to support multi-tenancy",
        status: "in_progress",
        priority: "high",
        author: "mellowagain",
        createdAt: "4 days ago",
        updatedAt: "2h",
        comments: 6,
        labels: [{ name: "infrastructure", color: "#fbca04" }],
        assignee: "mellowagain",
    },
    {
        id: 35,
        title: "Improve error messages for failed git operations",
        status: "todo",
        priority: "low",
        author: "contributor1",
        createdAt: "1 week ago",
        updatedAt: "5d",
        comments: 2,
        labels: [{ name: "dx", color: "#c5def5" }],
    },
];

const views = [
    { id: "all", label: "All Issues", icon: Inbox, count: issues.length },
    { id: "in_progress", label: "In Progress", icon: CircleDot, count: issues.filter((i) => i.status === "in_progress").length },
    { id: "todo", label: "Todo", icon: Circle, count: issues.filter((i) => i.status === "todo").length },
    { id: "done", label: "Done", icon: CheckCircle2, count: issues.filter((i) => i.status === "done").length },
];

// Status config
const statusConfig: Record<Status, { icon: typeof Circle; label: string; color: string }> = {
    todo: { icon: Circle, label: "Todo", color: "text-muted-foreground" },
    in_progress: { icon: CircleDot, label: "In Progress", color: "text-amber-500" },
    done: { icon: CheckCircle2, label: "Done", color: "text-green-500" },
    cancelled: { icon: Circle, label: "Cancelled", color: "text-muted-foreground/50" },
};

// Priority config
const priorityConfig: Record<Priority, { bars: number; color: string; label: string }> = {
    urgent: { bars: 4, color: "bg-red-500", label: "Urgent" },
    high: { bars: 3, color: "bg-orange-500", label: "High" },
    medium: { bars: 2, color: "bg-yellow-500", label: "Medium" },
    low: { bars: 1, color: "bg-blue-500", label: "Low" },
    none: { bars: 0, color: "bg-muted", label: "No priority" },
};

function PriorityIndicator({ priority }: { priority: Priority }) {
    const config = priorityConfig[priority];
    return (
        <div className="flex items-end gap-0.5 h-4 w-4" title={config.label}>
            {[1, 2, 3, 4].map((bar) => (
                <div
                    key={bar}
                    className={`w-0.5 rounded-full ${bar <= config.bars ? config.color : "bg-muted-foreground/20"}`}
                    style={{ height: `${bar * 25}%` }}
                />
            ))}
        </div>
    );
}

function StatusIcon({ status }: { status: Status }) {
    const config = statusConfig[status];
    const Icon = config.icon;
    return <Icon className={`h-[18px] w-[18px] ${config.color}`} />;
}

function LabelBadge({ label }: { label: { name: string; color: string } }) {
    const scopedIndex = label.name.indexOf("::");
    const isScoped = scopedIndex !== -1;
    const scopeKey = isScoped ? label.name.slice(0, scopedIndex) : null;
    const scopeValue = isScoped ? label.name.slice(scopedIndex + 2) : null;

    if (isScoped) {
        return (
            <span className="inline-flex items-center text-xs rounded overflow-hidden shrink-0">
                <span className="px-2 py-0.5 font-medium" style={{ backgroundColor: `${label.color}35`, color: label.color }}>
                    {scopeKey}
                </span>
                <span className="px-2 py-0.5" style={{ backgroundColor: `${label.color}20`, color: label.color }}>
                    {scopeValue}
                </span>
            </span>
        );
    }
    return (
        <span className="shrink-0 px-2 py-0.5 text-xs rounded" style={{ backgroundColor: `${label.color}20`, color: label.color }}>
            {label.name}
        </span>
    );
}

function IssueRow({ issue, isSelected, onSelect }: { issue: Issue; isSelected: boolean; onSelect: () => void }) {
    return (
        <Link
            href={`/${repoData.org}/${repoData.name}/issues/${issue.id}`}
            onClick={onSelect}
            className={`group flex items-center gap-4 px-5 py-3 border-b border-border cursor-pointer transition-colors ${
                isSelected ? "bg-accent" : "hover:bg-accent/50"
            }`}
        >
            <button className="shrink-0 hover:scale-110 transition-transform" onClick={(e) => e.preventDefault()}>
                <StatusIcon status={issue.status} />
            </button>

            <div className="shrink-0">
                <PriorityIndicator priority={issue.priority} />
            </div>

            <span className="shrink-0 text-sm text-muted-foreground font-mono">#{issue.id}</span>

            <div className="flex-1 min-w-0 flex items-center gap-2">
                <span className="truncate">{issue.title}</span>
                {issue.labels.map((label) => (
                    <LabelBadge key={label.name} label={label} />
                ))}
            </div>

            {issue.comments > 0 && (
                <div className="shrink-0 flex items-center gap-1.5 text-sm text-muted-foreground">
                    <MessageSquare className="h-4 w-4" />
                    <span>{issue.comments}</span>
                </div>
            )}

            <div className="shrink-0 w-7">
                {issue.assignee && (
                    <div
                        className="flex h-7 w-7 items-center justify-center rounded-full bg-secondary text-sm font-medium"
                        title={issue.assignee}
                    >
                        {issue.assignee[0].toUpperCase()}
                    </div>
                )}
            </div>

            <span className="shrink-0 text-sm text-muted-foreground w-10 text-right">{issue.updatedAt}</span>

            <button
                className="shrink-0 opacity-0 group-hover:opacity-100 p-1.5 hover:bg-secondary rounded transition-all"
                onClick={(e) => e.preventDefault()}
            >
                <MoreHorizontal className="h-4 w-4 text-muted-foreground" />
            </button>
        </Link>
    );
}

export default function IssuesPage() {
    const [activeView, setActiveView] = useState("all");
    const [selectedIssue, setSelectedIssue] = useState<number | null>(null);
    const [searchQuery, setSearchQuery] = useState("");
    const [sidebarWidth, setSidebarWidth] = useState(320);
    const [isResizing, setIsResizing] = useState(false);
    const sidebarRef = useRef<HTMLDivElement>(null);

    const filteredIssues = issues.filter((issue) => {
        if (activeView === "in_progress" && issue.status !== "in_progress") {
            return false;
        }
        if (activeView === "todo" && issue.status !== "todo") {
            return false;
        }
        if (activeView === "done" && issue.status !== "done") {
            return false;
        }
        if (searchQuery && !issue.title.toLowerCase().includes(searchQuery.toLowerCase())) {
            return false;
        }
        return true;
    });

    const startResizing = () => setIsResizing(true);
    const stopResizing = () => setIsResizing(false);

    useEffect(() => {
        const handleMouseMove = (e: MouseEvent) => {
            if (!isResizing) {
                return;
            }
            const newWidth = Math.max(240, Math.min(480, e.clientX));
            setSidebarWidth(newWidth);
        };

        if (isResizing) {
            window.addEventListener("mousemove", handleMouseMove);
            window.addEventListener("mouseup", stopResizing);
        }

        return () => {
            window.removeEventListener("mousemove", handleMouseMove);
            window.removeEventListener("mouseup", stopResizing);
        };
    }, [isResizing]);

    return (
        <div className="min-h-screen bg-background flex flex-col">
            <TopBar
                breadcrumb={[
                    { label: repoData.org, href: `/${repoData.org}` },
                    { label: repoData.name, href: `/${repoData.org}/${repoData.name}` },
                    { label: "Issues" },
                ]}
                navLinks={[
                    { label: "Code", href: `/${repoData.org}/${repoData.name}`, icon: <Code className="h-[18px] w-[18px]" /> },
                    {
                        label: "Issues",
                        href: `/${repoData.org}/${repoData.name}/issues`,
                        icon: <AlertCircle className="h-[18px] w-[18px]" />,
                        active: true,
                    },
                    {
                        label: "Merge Requests",
                        href: `/${repoData.org}/${repoData.name}/merge-requests`,
                        icon: <GitMerge className="h-[18px] w-[18px]" />,
                    },
                ]}
                hasNotifications
            />

            <div className="flex-1 flex overflow-hidden">
                <aside
                    ref={sidebarRef}
                    className="border-r border-border shrink-0 flex flex-col bg-card/30 relative"
                    style={{ width: sidebarWidth }}
                >
                    <div className="p-4">
                        <div className="relative">
                            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                            <input
                                type="text"
                                placeholder="Search issues..."
                                value={searchQuery}
                                onChange={(e) => setSearchQuery(e.target.value)}
                                className="w-full h-9 pl-9 pr-3 bg-secondary border-0 rounded-md text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
                            />
                        </div>
                    </div>

                    <div className="flex-1 overflow-y-auto px-3 pb-4">
                        <div className="space-y-1">
                            {views.map((view) => {
                                const Icon = view.icon;
                                const isActive = activeView === view.id;
                                return (
                                    <button
                                        key={view.id}
                                        onClick={() => setActiveView(view.id)}
                                        className={`w-full flex items-center gap-3 px-3 py-2 rounded-md transition-colors ${
                                            isActive
                                                ? "bg-accent text-foreground"
                                                : "text-muted-foreground hover:text-foreground hover:bg-accent/50"
                                        }`}
                                    >
                                        <Icon className="h-[18px] w-[18px] shrink-0" />
                                        <span className="flex-1 text-left">{view.label}</span>
                                        <span className="text-sm text-muted-foreground">{view.count}</span>
                                    </button>
                                );
                            })}
                        </div>
                    </div>

                    <div className="p-4 border-t border-border">
                        <div className="flex items-center gap-2 text-sm text-muted-foreground">
                            <kbd className="px-2 py-1 bg-secondary rounded text-xs">C</kbd>
                            <span>New issue</span>
                        </div>
                    </div>

                    <div
                        className="absolute right-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-ring/50 transition-colors"
                        onMouseDown={startResizing}
                    />
                </aside>

                <main className="flex-1 flex flex-col overflow-hidden">
                    <div className="flex items-center justify-between px-5 py-3 border-b border-border shrink-0">
                        <div className="flex items-center gap-3">
                            <h2 className="font-medium">{views.find((v) => v.id === activeView)?.label}</h2>
                            <span className="text-sm text-muted-foreground">{filteredIssues.length} issues</span>
                        </div>
                        <div className="flex items-center gap-2">
                            <DropdownMenu>
                                <DropdownMenuTrigger asChild>
                                    <Button variant="ghost" size="sm" className="h-9 gap-2 text-muted-foreground">
                                        Filter
                                        <ChevronDown className="h-4 w-4" />
                                    </Button>
                                </DropdownMenuTrigger>
                                <DropdownMenuContent align="end" className="w-48">
                                    <DropdownMenuItem>Status</DropdownMenuItem>
                                    <DropdownMenuItem>Priority</DropdownMenuItem>
                                    <DropdownMenuItem>Assignee</DropdownMenuItem>
                                    <DropdownMenuItem>Label</DropdownMenuItem>
                                </DropdownMenuContent>
                            </DropdownMenu>
                            <DropdownMenu>
                                <DropdownMenuTrigger asChild>
                                    <Button variant="ghost" size="sm" className="h-9 gap-2 text-muted-foreground">
                                        Sort
                                        <ChevronDown className="h-4 w-4" />
                                    </Button>
                                </DropdownMenuTrigger>
                                <DropdownMenuContent align="end" className="w-48">
                                    <DropdownMenuItem>Newest</DropdownMenuItem>
                                    <DropdownMenuItem>Oldest</DropdownMenuItem>
                                    <DropdownMenuItem>Recently updated</DropdownMenuItem>
                                    <DropdownMenuItem>Priority</DropdownMenuItem>
                                </DropdownMenuContent>
                            </DropdownMenu>
                            <Button size="sm" className="h-9 gap-2">
                                <Plus className="h-4 w-4" />
                                New Issue
                            </Button>
                        </div>
                    </div>

                    <div className="flex-1 overflow-y-auto">
                        {filteredIssues.length > 0 ? (
                            <div>
                                {filteredIssues.map((issue) => (
                                    <IssueRow
                                        key={issue.id}
                                        issue={issue}
                                        isSelected={selectedIssue === issue.id}
                                        onSelect={() => setSelectedIssue(issue.id)}
                                    />
                                ))}
                            </div>
                        ) : (
                            <div className="flex flex-col items-center justify-center h-full text-muted-foreground">
                                <Inbox className="h-16 w-16 mb-4 opacity-30" />
                                <p className="text-lg font-medium">No issues</p>
                                <p className="mt-1">Create your first issue to get started</p>
                                <Button size="sm" className="mt-6 gap-2">
                                    <Plus className="h-4 w-4" />
                                    New Issue
                                </Button>
                            </div>
                        )}
                    </div>
                </main>
            </div>
        </div>
    );
}

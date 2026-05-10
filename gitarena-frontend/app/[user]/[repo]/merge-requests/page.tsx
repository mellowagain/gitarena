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
    MessageSquare,
    Inbox,
    MoreHorizontal,
    Code,
    GitPullRequest,
    GitMergeIcon,
    XCircle,
    GitBranch,
} from "lucide-react";

const repoData = {
    org: "mellowagain",
    name: "test",
    visibility: "public" as const,
};

type MRStatus = "open" | "merged" | "closed" | "draft";
type ReviewStatus = "pending" | "approved" | "changes_requested" | "none";

type MergeRequest = {
    id: string;
    title: string;
    status: MRStatus;
    reviewStatus: ReviewStatus;
    author: string;
    createdAt: string;
    updatedAt: string;
    comments: number;
    sourceBranch: string;
    targetBranch: string;
    additions: number;
    deletions: number;
    labels: { name: string; color: string }[];
    reviewers: string[];
    ciStatus: "pending" | "passed" | "failed" | "none";
};

const mergeRequests: MergeRequest[] = [
    {
        id: 15,
        title: "feat: Add SSH key authentication support",
        status: "open",
        reviewStatus: "approved",
        author: "mellowagain",
        createdAt: "1 day ago",
        updatedAt: "2h",
        comments: 12,
        sourceBranch: "feature/ssh-auth",
        targetBranch: "main",
        additions: 342,
        deletions: 28,
        labels: [
            { name: "enhancement", color: "#a2eeef" },
            { name: "component::auth", color: "#d73a4a" },
        ],
        reviewers: ["Mari", "contributor1"],
        ciStatus: "passed",
    },
    {
        id: 14,
        title: "fix: Resolve memory leak in git pack parsing",
        status: "open",
        reviewStatus: "changes_requested",
        author: "contributor1",
        createdAt: "2 days ago",
        updatedAt: "5h",
        comments: 8,
        sourceBranch: "fix/memory-leak",
        targetBranch: "main",
        additions: 56,
        deletions: 124,
        labels: [{ name: "bug", color: "#d73a4a" }],
        reviewers: ["mellowagain"],
        ciStatus: "passed",
    },
    {
        id: 13,
        title: "WIP: Implement webhook support for CI/CD",
        status: "draft",
        reviewStatus: "none",
        author: "devops_user",
        createdAt: "3 days ago",
        updatedAt: "1d",
        comments: 3,
        sourceBranch: "feature/webhooks",
        targetBranch: "main",
        additions: 789,
        deletions: 12,
        labels: [
            { name: "feature", color: "#0e8a16" },
            { name: "wip", color: "#fbca04" },
        ],
        reviewers: [],
        ciStatus: "pending",
    },
    {
        id: 12,
        title: "docs: Update API endpoint documentation",
        status: "open",
        reviewStatus: "pending",
        author: "docs_contributor",
        createdAt: "4 days ago",
        updatedAt: "3d",
        comments: 2,
        sourceBranch: "docs/api-update",
        targetBranch: "main",
        additions: 156,
        deletions: 34,
        labels: [{ name: "documentation", color: "#0075ca" }],
        reviewers: ["mellowagain"],
        ciStatus: "passed",
    },
    {
        id: 11,
        title: "feat: Add dark mode theme support",
        status: "merged",
        reviewStatus: "approved",
        author: "ui_designer",
        createdAt: "1 week ago",
        updatedAt: "5d",
        comments: 18,
        sourceBranch: "feature/dark-mode",
        targetBranch: "main",
        additions: 423,
        deletions: 89,
        labels: [{ name: "enhancement", color: "#a2eeef" }],
        reviewers: ["mellowagain", "Mari"],
        ciStatus: "passed",
    },
    {
        id: 10,
        title: "refactor: Migrate to async database connections",
        status: "closed",
        reviewStatus: "none",
        author: "mellowagain",
        createdAt: "2 weeks ago",
        updatedAt: "1w",
        comments: 5,
        sourceBranch: "refactor/async-db",
        targetBranch: "main",
        additions: 234,
        deletions: 567,
        labels: [{ name: "infrastructure", color: "#fbca04" }],
        reviewers: [],
        ciStatus: "failed",
    },
];

const views = [
    { id: "all", label: "All", icon: Inbox, count: mergeRequests.length },
    {
        id: "open",
        label: "Open",
        icon: GitPullRequest,
        count: mergeRequests.filter((m) => m.status === "open" || m.status === "draft").length,
    },
    { id: "merged", label: "Merged", icon: GitMergeIcon, count: mergeRequests.filter((m) => m.status === "merged").length },
    { id: "closed", label: "Closed", icon: XCircle, count: mergeRequests.filter((m) => m.status === "closed").length },
];

const statusConfig: Record<MRStatus, { icon: typeof GitPullRequest; label: string; color: string }> = {
    open: { icon: GitPullRequest, label: "Open", color: "text-green-500" },
    draft: { icon: GitPullRequest, label: "Draft", color: "text-muted-foreground" },
    merged: { icon: GitMergeIcon, label: "Merged", color: "text-purple-500" },
    closed: { icon: XCircle, label: "Closed", color: "text-red-500" },
};

const ciConfig: Record<string, { color: string; label: string }> = {
    pending: { color: "bg-amber-500", label: "CI pending" },
    passed: { color: "bg-green-500", label: "CI passed" },
    failed: { color: "bg-red-500", label: "CI failed" },
    none: { color: "bg-muted", label: "No CI" },
};

function StatusIcon({ status }: { status: MRStatus }) {
    const config = statusConfig[status];
    const Icon = config.icon;
    return <Icon className={`h-[18px] w-[18px] ${config.color}`} />;
}

function CIIndicator({ status }: { status: string }) {
    const config = ciConfig[status];
    return <div className={`w-2.5 h-2.5 rounded-full ${config.color}`} title={config.label} />;
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

function MRRow({ mr, isSelected, onSelect }: { mr: MergeRequest; isSelected: boolean; onSelect: () => void }) {
    return (
        <Link
            href={`/${repoData.org}/${repoData.name}/merge-requests/${mr.id}`}
            onClick={onSelect}
            className={`group flex items-center gap-4 px-5 py-3 border-b border-border cursor-pointer transition-colors ${
                isSelected ? "bg-accent" : "hover:bg-accent/50"
            }`}
        >
            <button className="shrink-0 hover:scale-110 transition-transform" onClick={(e) => e.preventDefault()}>
                <StatusIcon status={mr.status} />
            </button>

            <div className="shrink-0">
                <CIIndicator status={mr.ciStatus} />
            </div>

            <span className="shrink-0 text-sm text-muted-foreground font-mono">!{mr.id}</span>

            <div className="flex-1 min-w-0 flex items-center gap-2">
                <span className="truncate">{mr.title}</span>
                {mr.labels.map((label) => (
                    <LabelBadge key={label.name} label={label} />
                ))}
            </div>

            <div className="shrink-0 flex items-center gap-1.5 text-sm text-muted-foreground">
                <GitBranch className="h-3.5 w-3.5" />
                <span className="font-mono text-xs max-w-[100px] truncate">{mr.sourceBranch}</span>
            </div>

            <div className="shrink-0 flex items-center gap-2 text-sm font-mono">
                <span className="text-green-500">+{mr.additions}</span>
                <span className="text-red-500">-{mr.deletions}</span>
            </div>

            {mr.comments > 0 && (
                <div className="shrink-0 flex items-center gap-1.5 text-sm text-muted-foreground">
                    <MessageSquare className="h-4 w-4" />
                    <span>{mr.comments}</span>
                </div>
            )}

            <div className="shrink-0 flex -space-x-1.5">
                {mr.reviewers.slice(0, 2).map((reviewer) => (
                    <div
                        key={reviewer}
                        className="flex h-6 w-6 items-center justify-center rounded-full bg-secondary text-xs font-medium border-2 border-background"
                        title={reviewer}
                    >
                        {reviewer[0].toUpperCase()}
                    </div>
                ))}
            </div>

            <span className="shrink-0 text-sm text-muted-foreground w-10 text-right">{mr.updatedAt}</span>

            <button
                className="shrink-0 opacity-0 group-hover:opacity-100 p-1.5 hover:bg-secondary rounded transition-all"
                onClick={(e) => e.preventDefault()}
            >
                <MoreHorizontal className="h-4 w-4 text-muted-foreground" />
            </button>
        </Link>
    );
}

export default function MergeRequestsPage() {
    const [activeView, setActiveView] = useState("all");
    const [selectedMR, setSelectedMR] = useState<number | null>(null);
    const [searchQuery, setSearchQuery] = useState("");
    const [sidebarWidth, setSidebarWidth] = useState(320);
    const [isResizing, setIsResizing] = useState(false);
    const sidebarRef = useRef<HTMLDivElement>(null);

    const filteredMRs = mergeRequests.filter((mr) => {
        if (activeView === "open" && mr.status !== "open" && mr.status !== "draft") {
            return false;
        }
        if (activeView === "merged" && mr.status !== "merged") {
            return false;
        }
        if (activeView === "closed" && mr.status !== "closed") {
            return false;
        }
        if (searchQuery && !mr.title.toLowerCase().includes(searchQuery.toLowerCase())) {
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
                    { label: "Merge Requests" },
                ]}
                navLinks={[
                    { label: "Code", href: `/${repoData.org}/${repoData.name}`, icon: <Code className="h-[18px] w-[18px]" /> },
                    {
                        label: "Issues",
                        href: `/${repoData.org}/${repoData.name}/issues`,
                        icon: <AlertCircle className="h-[18px] w-[18px]" />,
                    },
                    {
                        label: "Merge Requests",
                        href: `/${repoData.org}/${repoData.name}/merge-requests`,
                        icon: <GitMerge className="h-[18px] w-[18px]" />,
                        active: true,
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
                                placeholder="Search merge requests..."
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
                            <kbd className="px-2 py-1 bg-secondary rounded text-xs">N</kbd>
                            <span>New merge request</span>
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
                            <span className="text-sm text-muted-foreground">{filteredMRs.length} merge requests</span>
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
                                    <DropdownMenuItem>Author</DropdownMenuItem>
                                    <DropdownMenuItem>Reviewer</DropdownMenuItem>
                                    <DropdownMenuItem>Label</DropdownMenuItem>
                                    <DropdownMenuItem>Branch</DropdownMenuItem>
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
                                </DropdownMenuContent>
                            </DropdownMenu>
                            <Button size="sm" className="h-9 gap-2">
                                <Plus className="h-4 w-4" />
                                New Merge Request
                            </Button>
                        </div>
                    </div>

                    <div className="flex-1 overflow-y-auto">
                        {filteredMRs.length > 0 ? (
                            <div>
                                {filteredMRs.map((mr) => (
                                    <MRRow key={mr.id} mr={mr} isSelected={selectedMR === mr.id} onSelect={() => setSelectedMR(mr.id)} />
                                ))}
                            </div>
                        ) : (
                            <div className="flex flex-col items-center justify-center h-full text-muted-foreground">
                                <Inbox className="h-16 w-16 mb-4 opacity-30" />
                                <p className="text-lg font-medium">No merge requests</p>
                                <p className="mt-1">Create a merge request to start collaborating</p>
                                <Button size="sm" className="mt-6 gap-2">
                                    <Plus className="h-4 w-4" />
                                    New Merge Request
                                </Button>
                            </div>
                        )}
                    </div>
                </main>
            </div>
        </div>
    );
}

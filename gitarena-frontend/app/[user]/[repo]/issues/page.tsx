"use client";

import { useState, useRef, useEffect, type CSSProperties } from "react";
import Link from "next/link";
import useSWR from "swr";
import { useParams, useRouter } from "next/navigation";
import { TopBar } from "@/components/top-bar";
import { Button } from "@/components/ui/button";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuSeparator,
    DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
    AlertCircle,
    Search,
    ChevronDown,
    Plus,
    Circle,
    CheckCircle2,
    XCircle,
    MessageSquare,
    Inbox,
    CircleDot,
    MoreHorizontal,
    Code,
    Tag,
    Trash2,
    Milestone,
    Kanban,
} from "lucide-react";
import { jsonFetcher, patchJsonFetcher, deleteFetcher } from "@/lib/fetchers";
import { formatDistanceToNow } from "date-fns";
import { shortLocale } from "@/lib/utils";
import { Skeleton } from "@/components/ui/skeleton";
import { PriorityIndicator, type Priority } from "@/components/priority-indicator";
import type { RepoMetadata } from "@/app/[user]/[repo]/page";
import { ArchivedBanner } from "@/components/archived-banner";
import { UserAvatar } from "@/components/ui/user-avatar";

interface IssueLabel {
    name: string;
    color: string;
}

interface IssueListItem {
    index: number;
    title: string;
    status: string;
    priority: Priority;
    labels: string[];
    commentCount: number;
    authorId: string;
    authorUsername: string;
    assigneeIds: string[];
    assignees: string[];
    updatedAt: string;
}

interface IssuesResponse {
    issues: IssueListItem[];
    total: number;
}

interface LabelsResponse {
    labels: IssueLabel[];
}

interface PermissionsResponse {
    permissions: { manageIssues: boolean };
}

type ViewId = "all" | "open" | "in_progress" | "closed";

const views: { id: ViewId; label: string; icon: typeof Circle }[] = [
    { id: "all", label: "All Issues", icon: Inbox },
    { id: "open", label: "Open", icon: Circle },
    { id: "in_progress", label: "In Progress", icon: CircleDot },
    { id: "closed", label: "Closed", icon: CheckCircle2 },
];

function getStatusDisplay(status: string): { Icon: typeof Circle; color: string } {
    switch (status) {
        case "open":
            return { Icon: Circle, color: "text-green-500" };
        case "in_progress":
            return { Icon: CircleDot, color: "text-yellow-500" };
        case "completed":
            return { Icon: CheckCircle2, color: "text-muted-foreground" };
        case "not_planned":
            return { Icon: XCircle, color: "text-muted-foreground" };
        default:
            return { Icon: Circle, color: "text-green-500" };
    }
}

function LabelBadge({ name, color }: { name: string; color: string }) {
    const scopedIndex = name.indexOf("::");
    const isScoped = scopedIndex !== -1;
    const scopeKey = isScoped ? name.slice(0, scopedIndex) : null;
    const scopeValue = isScoped ? name.slice(scopedIndex + 2) : null;

    if (isScoped) {
        return (
            <span className="inline-flex items-center text-xs rounded overflow-hidden shrink-0">
                <span className="px-2 py-0.5 font-medium" style={{ backgroundColor: `${color}35`, color }}>
                    {scopeKey}
                </span>
                <span className="px-2 py-0.5" style={{ backgroundColor: `${color}20`, color }}>
                    {scopeValue}
                </span>
            </span>
        );
    }
    return (
        <span className="shrink-0 px-2 py-0.5 text-xs rounded" style={{ backgroundColor: `${color}20`, color }}>
            {name}
        </span>
    );
}

function IssueRow({
    issue,
    labelMap,
    isSelected,
    onSelect,
    user,
    repo,
    canManage,
    onSetStatus,
    onDelete,
}: {
    issue: IssueListItem;
    labelMap: Map<string, string>;
    isSelected: boolean;
    onSelect: () => void;
    user: string;
    repo: string;
    canManage: boolean;
    onSetStatus: (index: number, status: string) => void;
    onDelete: (index: number) => void;
}) {
    const { Icon: StatusIcon, color: statusColor } = getStatusDisplay(issue.status);
    const isClosed = issue.status === "completed" || issue.status === "not_planned";

    return (
        <Link
            href={`/${user}/${repo}/issues/${issue.index}`}
            onClick={onSelect}
            className={`group flex items-center gap-2 border-b border-border px-3 py-3 cursor-pointer transition-colors sm:gap-4 sm:px-5 ${
                isSelected ? "bg-accent" : "hover:bg-accent/50"
            }`}
        >
            <button className="shrink-0 hover:scale-110 transition-transform" onClick={(e) => e.preventDefault()}>
                <StatusIcon className={`h-[18px] w-[18px] ${statusColor}`} />
            </button>

            <PriorityIndicator priority={issue.priority} />

            <span className="shrink-0 text-sm text-muted-foreground font-mono">#{issue.index}</span>

            <div className="flex-1 min-w-0 flex items-center gap-2">
                <span className="truncate">{issue.title}</span>
                <div className="hidden lg:contents">
                    {issue.labels.map((name) => {
                        const color = labelMap.get(name) ?? "#888888";
                        return <LabelBadge key={name} name={name} color={color} />;
                    })}
                </div>
            </div>

            {issue.commentCount > 0 && (
                <div className="shrink-0 flex items-center gap-1.5 text-sm text-muted-foreground">
                    <MessageSquare className="h-4 w-4" />
                    <span>{issue.commentCount}</span>
                </div>
            )}

            <div className="hidden shrink-0 w-7 lg:block">
                {issue.assignees.length > 0 && (
                    <UserAvatar userId={issue.assigneeIds[0]} username={issue.assignees[0]} size="md" className="size-7" />
                )}
            </div>

            <span className="hidden shrink-0 text-sm text-muted-foreground w-10 text-right lg:block">
                {formatDistanceToNow(new Date(issue.updatedAt), { locale: shortLocale })}
            </span>

            {canManage && (
                <DropdownMenu modal={false}>
                    <DropdownMenuTrigger asChild>
                        <button
                            className="shrink-0 rounded p-1.5 opacity-100 transition-all hover:bg-secondary lg:opacity-0 lg:group-hover:opacity-100"
                            onClick={(e) => e.preventDefault()}
                            aria-label="Issue actions"
                        >
                            <MoreHorizontal className="h-4 w-4 text-muted-foreground" />
                        </button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end" className="w-48">
                        {isClosed ? (
                            <DropdownMenuItem
                                onClick={(e) => {
                                    e.preventDefault();
                                    onSetStatus(issue.index, "open");
                                }}
                            >
                                <Circle className="h-4 w-4 mr-2 text-green-500" />
                                Reopen
                            </DropdownMenuItem>
                        ) : (
                            <>
                                {issue.status === "open" && (
                                    <DropdownMenuItem
                                        onClick={(e) => {
                                            e.preventDefault();
                                            onSetStatus(issue.index, "in_progress");
                                        }}
                                    >
                                        <CircleDot className="h-4 w-4 mr-2 text-yellow-500" />
                                        Mark as In Progress
                                    </DropdownMenuItem>
                                )}
                                <DropdownMenuItem
                                    onClick={(e) => {
                                        e.preventDefault();
                                        onSetStatus(issue.index, "completed");
                                    }}
                                >
                                    <CheckCircle2 className="h-4 w-4 mr-2 text-muted-foreground" />
                                    Close as Completed
                                </DropdownMenuItem>
                                <DropdownMenuItem
                                    onClick={(e) => {
                                        e.preventDefault();
                                        onSetStatus(issue.index, "not_planned");
                                    }}
                                >
                                    <XCircle className="h-4 w-4 mr-2 text-muted-foreground" />
                                    Close as Not Planned
                                </DropdownMenuItem>
                            </>
                        )}
                        <DropdownMenuSeparator />
                        <DropdownMenuItem
                            className="text-destructive focus:text-destructive"
                            onClick={(e) => {
                                e.preventDefault();
                                onDelete(issue.index);
                            }}
                        >
                            <Trash2 className="h-4 w-4 mr-2" />
                            Delete issue
                        </DropdownMenuItem>
                    </DropdownMenuContent>
                </DropdownMenu>
            )}
        </Link>
    );
}

function IssueRowSkeleton() {
    return (
        <div className="flex items-center gap-2 border-b border-border px-3 py-3 sm:gap-4 sm:px-5">
            <Skeleton className="h-[18px] w-[18px] rounded-full shrink-0" />
            <Skeleton className="h-4 w-8 shrink-0" />
            <Skeleton className="h-4 flex-1" />
            <Skeleton className="h-4 w-10 shrink-0" />
        </div>
    );
}

export default function IssuesPage() {
    const params = useParams();
    const router = useRouter();
    const user = (params.user ?? params.org) as string;
    const repo = params.repo as string;

    const [activeView, setActiveView] = useState<ViewId>("all");
    const [selectedIssue, setSelectedIssue] = useState<number | null>(null);
    const [searchQuery, setSearchQuery] = useState("");
    const [sortBy, setSortBy] = useState<"newest" | "oldest" | "updated" | "priority">("newest");
    const [sidebarWidth, setSidebarWidth] = useState(320);
    const [isResizing, setIsResizing] = useState(false);
    const sidebarRef = useRef<HTMLDivElement>(null);

    const apiUrl = `/api/repos/${user}/${repo}/issues`;

    const { data, isLoading, mutate } = useSWR<IssuesResponse>(user && repo ? apiUrl : null, jsonFetcher);
    const { data: labelsData } = useSWR<LabelsResponse>(user && repo ? `/api/repos/${user}/${repo}/labels` : null, jsonFetcher);
    const { data: permsData } = useSWR<PermissionsResponse>(user && repo ? `/api/repos/${user}/${repo}/permissions` : null, jsonFetcher);
    const { data: repoMeta } = useSWR<RepoMetadata>(user && repo ? `/api/repos/${user}/${repo}` : null, jsonFetcher);

    const isArchived = repoMeta?.archivedAt != null;
    const canManage = (permsData?.permissions.manageIssues ?? false) && !isArchived;

    const labelMap = new Map<string, string>((labelsData?.labels ?? []).map((l) => [l.name, l.color]));

    const allIssues = data?.issues ?? [];

    const viewFilteredIssues = allIssues.filter((issue) => {
        if (activeView === "open") return issue.status === "open";
        if (activeView === "in_progress") return issue.status === "in_progress";
        if (activeView === "closed") return issue.status === "completed" || issue.status === "not_planned";
        return true;
    });

    const filteredIssues = viewFilteredIssues.filter((issue) => {
        if (searchQuery && !issue.title.toLowerCase().includes(searchQuery.toLowerCase())) {
            return false;
        }
        return true;
    });

    const PRIORITY_ORDER: Record<string, number> = { urgent: 0, high: 1, medium: 2, low: 3, none: 4 };

    const sortedIssues = [...filteredIssues].sort((a, b) => {
        if (sortBy === "oldest") return a.index - b.index;
        if (sortBy === "updated") return new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime();
        if (sortBy === "priority") return (PRIORITY_ORDER[a.priority] ?? 4) - (PRIORITY_ORDER[b.priority] ?? 4);
        return b.index - a.index;
    });

    const counts = {
        all: allIssues.length,
        open: allIssues.filter((i) => i.status === "open").length,
        in_progress: allIssues.filter((i) => i.status === "in_progress").length,
        closed: allIssues.filter((i) => i.status === "completed" || i.status === "not_planned").length,
    };

    const startResizing = () => setIsResizing(true);
    const stopResizing = () => setIsResizing(false);

    const handleSetStatus = async (index: number, status: string) => {
        await patchJsonFetcher(`/api/repos/${user}/${repo}/issues/${index}`, { arg: { status } });
        await mutate();
    };

    const handleDelete = async (index: number) => {
        await deleteFetcher(`/api/repos/${user}/${repo}/issues/${index}`);
        await mutate();
    };

    useEffect(() => {
        const handleKeyDown = (e: KeyboardEvent) => {
            if (
                e.key === "c" &&
                !e.metaKey &&
                !e.ctrlKey &&
                !(e.target instanceof HTMLInputElement) &&
                !(e.target instanceof HTMLTextAreaElement)
            ) {
                router.push(`/${user}/${repo}/issues/new`);
            }
        };
        window.addEventListener("keydown", handleKeyDown);
        return () => window.removeEventListener("keydown", handleKeyDown);
    }, [router, user, repo]);

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
        <div className="min-h-screen bg-background flex flex-col lg:h-screen lg:overflow-hidden">
            <TopBar
                breadcrumb={[{ label: user, href: `/${user}` }, { label: repo, href: `/${user}/${repo}` }, { label: "Issues" }]}
                navLinks={[
                    { label: "Code", href: `/${user}/${repo}`, icon: <Code className="h-[18px] w-[18px]" /> },
                    {
                        label: "Issues",
                        href: `/${user}/${repo}/issues`,
                        icon: <AlertCircle className="h-[18px] w-[18px]" />,
                        active: true,
                    },
                    //{
                    //    label: "Merge Requests",
                    //    href: `/${user}/${repo}/merge-requests`,
                    //    icon: <GitMerge className="h-[18px] w-[18px]" />,
                    //},
                ]}
                hasNotifications
            />
            {repoMeta?.archivedAt && <ArchivedBanner archivedAt={repoMeta.archivedAt} />}

            <div className="flex flex-1 flex-col min-h-0 lg:flex-row lg:overflow-hidden">
                <aside
                    ref={sidebarRef}
                    className="relative flex w-full shrink-0 flex-col border-b border-border bg-card/30 lg:w-[var(--issues-sidebar-width)] lg:border-r lg:border-b-0"
                    style={{ "--issues-sidebar-width": `${sidebarWidth}px` } as CSSProperties}
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

                    <div className="px-3 pb-4 lg:flex-1 lg:overflow-y-auto">
                        <div className="grid grid-cols-2 gap-1 lg:block lg:space-y-1">
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
                                        <span className="min-w-0 flex-1 truncate text-left">{view.label}</span>
                                        <span className="text-sm text-muted-foreground">{counts[view.id]}</span>
                                    </button>
                                );
                            })}
                        </div>
                    </div>

                    <div className="hidden px-4 py-4 border-t border-border lg:block">
                        <p className="text-xs text-muted-foreground leading-relaxed">
                            Issues are stored natively in git using the{" "}
                            <a
                                href="https://github.com/git-bug/git-bug"
                                target="_blank"
                                rel="noopener noreferrer"
                                className="underline hover:text-foreground transition-colors"
                            >
                                git-bug
                            </a>{" "}
                            format and can be accessed offline with the git-bug CLI.
                        </p>
                    </div>

                    <div className="hidden px-4 pb-4 border-t border-border pt-4 lg:block">
                        <div className="flex items-center gap-2 text-sm text-muted-foreground">
                            <kbd className="px-2 py-1 bg-secondary rounded text-xs">C</kbd>
                            <span>New issue</span>
                        </div>
                    </div>

                    <div
                        className="absolute right-0 top-0 bottom-0 hidden w-1 cursor-col-resize hover:bg-ring/50 transition-colors lg:block"
                        onMouseDown={startResizing}
                    />
                </aside>

                <main className="flex w-full min-w-0 flex-col lg:flex-1 lg:overflow-hidden">
                    <div className="flex items-center justify-between gap-2 border-b border-border px-3 py-3 shrink-0 sm:px-5">
                        <div className="flex min-w-0 items-center gap-2 sm:gap-3">
                            <h2 className="truncate font-medium">{views.find((v) => v.id === activeView)?.label}</h2>
                            <span className="shrink-0 text-sm text-muted-foreground">
                                {sortedIssues.length}
                                <span className="hidden sm:inline"> issues</span>
                            </span>
                        </div>
                        <div className="flex shrink-0 items-center gap-1 sm:gap-2">
                            <DropdownMenu modal={false}>
                                <DropdownMenuTrigger asChild>
                                    <Button variant="ghost" size="sm" className="h-9 gap-2 text-muted-foreground">
                                        Sort
                                        <ChevronDown className="h-4 w-4" />
                                    </Button>
                                </DropdownMenuTrigger>
                                <DropdownMenuContent align="end" className="w-48">
                                    <DropdownMenuItem onClick={() => setSortBy("newest")} className="flex items-center gap-2">
                                        <span className="flex-1">Newest</span>
                                        {sortBy === "newest" && <CheckCircle2 className="h-3.5 w-3.5 text-green-500" />}
                                    </DropdownMenuItem>
                                    <DropdownMenuItem onClick={() => setSortBy("oldest")} className="flex items-center gap-2">
                                        <span className="flex-1">Oldest</span>
                                        {sortBy === "oldest" && <CheckCircle2 className="h-3.5 w-3.5 text-green-500" />}
                                    </DropdownMenuItem>
                                    <DropdownMenuItem onClick={() => setSortBy("updated")} className="flex items-center gap-2">
                                        <span className="flex-1">Recently updated</span>
                                        {sortBy === "updated" && <CheckCircle2 className="h-3.5 w-3.5 text-green-500" />}
                                    </DropdownMenuItem>
                                    <DropdownMenuItem onClick={() => setSortBy("priority")} className="flex items-center gap-2">
                                        <span className="flex-1">Priority</span>
                                        {sortBy === "priority" && <CheckCircle2 className="h-3.5 w-3.5 text-green-500" />}
                                    </DropdownMenuItem>
                                </DropdownMenuContent>
                            </DropdownMenu>

                            <Link
                                href={`/${user}/${repo}/labels`}
                                className="hidden items-center gap-2 h-9 px-3 text-sm text-muted-foreground border border-border rounded-md hover:text-foreground hover:bg-accent/50 transition-colors lg:flex"
                            >
                                <Tag className="h-4 w-4" />
                                Labels
                            </Link>

                            <Link
                                href={`/${user}/${repo}/milestones`}
                                className="hidden items-center gap-2 h-9 px-3 text-sm text-muted-foreground border border-border rounded-md hover:text-foreground hover:bg-accent/50 transition-colors lg:flex"
                            >
                                <Milestone className="h-4 w-4" />
                                Milestones
                            </Link>

                            <Link
                                href={`/${user}/${repo}/issues/board`}
                                className="hidden items-center gap-2 h-9 px-3 text-sm text-muted-foreground border border-border rounded-md hover:text-foreground hover:bg-accent/50 transition-colors lg:flex"
                                aria-label="Kanban board"
                            >
                                <Kanban className="h-4 w-4" />
                            </Link>

                            <DropdownMenu modal={false}>
                                <DropdownMenuTrigger asChild>
                                    <Button
                                        variant="ghost"
                                        size="icon-sm"
                                        className="text-muted-foreground lg:hidden"
                                        aria-label="More issue links"
                                    >
                                        <MoreHorizontal className="h-4 w-4" />
                                    </Button>
                                </DropdownMenuTrigger>
                                <DropdownMenuContent align="end" className="w-44">
                                    <DropdownMenuItem asChild>
                                        <Link href={`/${user}/${repo}/labels`}>
                                            <Tag className="h-4 w-4" />
                                            Labels
                                        </Link>
                                    </DropdownMenuItem>
                                    <DropdownMenuItem asChild>
                                        <Link href={`/${user}/${repo}/milestones`}>
                                            <Milestone className="h-4 w-4" />
                                            Milestones
                                        </Link>
                                    </DropdownMenuItem>
                                    <DropdownMenuItem asChild>
                                        <Link href={`/${user}/${repo}/issues/board`}>
                                            <Kanban className="h-4 w-4" />
                                            Board
                                        </Link>
                                    </DropdownMenuItem>
                                </DropdownMenuContent>
                            </DropdownMenu>

                            {!isArchived && (
                                <Link href={`/${user}/${repo}/issues/new`}>
                                    <Button size="sm" className="h-8 w-8 gap-2 px-0 lg:h-9 lg:w-auto lg:px-3">
                                        <Plus className="h-4 w-4" />
                                        <span className="sr-only lg:not-sr-only">New Issue</span>
                                    </Button>
                                </Link>
                            )}
                        </div>
                    </div>

                    <div className="lg:flex-1 lg:overflow-y-auto">
                        {isLoading ? (
                            <div>
                                {Array.from({ length: 5 }).map((_, i) => (
                                    <IssueRowSkeleton key={i} />
                                ))}
                            </div>
                        ) : sortedIssues.length > 0 ? (
                            <div>
                                {sortedIssues.map((issue) => (
                                    <IssueRow
                                        key={issue.index}
                                        issue={issue}
                                        labelMap={labelMap}
                                        isSelected={selectedIssue === issue.index}
                                        onSelect={() => setSelectedIssue(issue.index)}
                                        user={user}
                                        repo={repo}
                                        canManage={canManage}
                                        onSetStatus={handleSetStatus}
                                        onDelete={handleDelete}
                                    />
                                ))}
                            </div>
                        ) : (
                            <div className="flex flex-col items-center justify-center py-16 text-muted-foreground lg:h-full lg:py-0">
                                <Inbox className="h-16 w-16 mb-4 opacity-30" />
                                <p className="text-lg font-medium">No issues</p>
                                <p className="mt-1">Create your first issue to get started</p>
                                {!isArchived && (
                                    <Link href={`/${user}/${repo}/issues/new`}>
                                        <Button size="sm" className="mt-6 gap-2">
                                            <Plus className="h-4 w-4" />
                                            New Issue
                                        </Button>
                                    </Link>
                                )}
                            </div>
                        )}
                    </div>
                </main>
            </div>
        </div>
    );
}

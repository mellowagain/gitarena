"use client";

import { useState, useMemo } from "react";
import Link from "next/link";
import { useParams } from "next/navigation";
import useSWR from "swr";
import { TopBar } from "@/components/top-bar";
import { Skeleton } from "@/components/ui/skeleton";
import {
    DndContext,
    DragEndEvent,
    DragOverlay,
    DragStartEvent,
    PointerSensor,
    useDraggable,
    useDroppable,
    useSensor,
    useSensors,
} from "@dnd-kit/core";
import { AlertCircle, Code, Circle, CircleDot, CheckCircle2, XCircle, List, Search } from "lucide-react";
import { jsonFetcher, patchJsonFetcher } from "@/lib/fetchers";
import { PriorityIndicator, type Priority } from "@/components/priority-indicator";
import { useIsMobile } from "@/hooks/use-mobile";

interface IssueListItem {
    index: number;
    title: string;
    status: string;
    priority: Priority;
    labels: string[];
    commentCount: number;
    authorUsername: string;
    assignees: string[];
    updatedAt: string;
}

interface IssueLabel {
    name: string;
    color: string;
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

const COLUMNS: { status: string; label: string; Icon: typeof Circle; color: string }[] = [
    { status: "open", label: "Open", Icon: Circle, color: "text-green-500" },
    { status: "in_progress", label: "In Progress", Icon: CircleDot, color: "text-yellow-500" },
    { status: "completed", label: "Completed", Icon: CheckCircle2, color: "text-muted-foreground" },
    { status: "not_planned", label: "Not Planned", Icon: XCircle, color: "text-muted-foreground" },
];

function LabelBadge({ name, color }: { name: string; color: string }) {
    const scopedIndex = name.indexOf("::");
    const isScoped = scopedIndex !== -1;
    const scopeKey = isScoped ? name.slice(0, scopedIndex) : null;
    const scopeValue = isScoped ? name.slice(scopedIndex + 2) : null;

    if (isScoped) {
        return (
            <span className="inline-flex items-center text-xs rounded overflow-hidden shrink-0">
                <span className="px-1.5 py-0.5 font-medium" style={{ backgroundColor: `${color}35`, color }}>
                    {scopeKey}
                </span>
                <span className="px-1.5 py-0.5" style={{ backgroundColor: `${color}20`, color }}>
                    {scopeValue}
                </span>
            </span>
        );
    }
    return (
        <span className="shrink-0 px-1.5 py-0.5 text-xs rounded" style={{ backgroundColor: `${color}20`, color }}>
            {name}
        </span>
    );
}

function IssueCard({ issue, labelMap, isDragOverlay }: { issue: IssueListItem; labelMap: Map<string, string>; isDragOverlay?: boolean }) {
    return (
        <div
            className={`bg-card border border-border rounded-md p-3 space-y-2 ${
                isDragOverlay ? "shadow-lg rotate-1 opacity-90" : "hover:border-ring/50 transition-colors"
            }`}
        >
            <div className="flex items-start gap-2">
                <PriorityIndicator priority={issue.priority} />
                <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium leading-snug line-clamp-2">{issue.title}</p>
                    <p className="text-xs text-muted-foreground mt-0.5">#{issue.index}</p>
                </div>
                {issue.assignees.length > 0 && (
                    <div
                        className="shrink-0 h-6 w-6 rounded-full bg-secondary flex items-center justify-center text-xs font-medium"
                        title={issue.assignees[0]}
                    >
                        {issue.assignees[0][0].toUpperCase()}
                    </div>
                )}
            </div>
            {issue.labels.length > 0 && (
                <div className="flex flex-wrap gap-1">
                    {issue.labels.map((name) => {
                        const color = labelMap.get(name) ?? "#888888";
                        return <LabelBadge key={name} name={name} color={color} />;
                    })}
                </div>
            )}
        </div>
    );
}

function DraggableCard({
    issue,
    labelMap,
    user,
    repo,
}: {
    issue: IssueListItem;
    labelMap: Map<string, string>;
    user: string;
    repo: string;
}) {
    const { attributes, listeners, setNodeRef, isDragging } = useDraggable({ id: String(issue.index) });

    return (
        <div ref={setNodeRef} {...listeners} {...attributes} className={`touch-none ${isDragging ? "opacity-0" : ""}`}>
            <Link href={`/${user}/${repo}/issues/${issue.index}`} onClick={(e) => isDragging && e.preventDefault()}>
                <IssueCard issue={issue} labelMap={labelMap} />
            </Link>
        </div>
    );
}

function DroppableColumn({
    status,
    label,
    Icon,
    color,
    issues,
    labelMap,
    user,
    repo,
    isLoading,
    canDrag,
}: {
    status: string;
    label: string;
    Icon: typeof Circle;
    color: string;
    issues: IssueListItem[];
    labelMap: Map<string, string>;
    user: string;
    repo: string;
    isLoading: boolean;
    canDrag: boolean;
}) {
    const { setNodeRef, isOver } = useDroppable({ id: status, disabled: !canDrag });

    return (
        <div className="flex w-full flex-col md:min-w-52 md:flex-1">
            <div className="flex items-center gap-2 mb-3 px-1 shrink-0">
                <Icon className={`h-4 w-4 shrink-0 ${color}`} />
                <span className="font-medium text-sm">{label}</span>
                <span className="ml-auto text-xs text-muted-foreground bg-secondary px-1.5 py-0.5 rounded-full">
                    {isLoading ? "-" : issues.length}
                </span>
            </div>
            <div
                ref={setNodeRef}
                className={`scrollbar-dark space-y-2 rounded-lg p-2 transition-colors md:min-h-0 md:flex-1 md:overflow-y-auto ${
                    isOver && canDrag ? "bg-accent/50 ring-2 ring-ring/40" : "bg-secondary/30"
                }`}
            >
                {isLoading ? (
                    <>
                        <Skeleton className="h-20 w-full rounded-md" />
                        <Skeleton className="h-16 w-full rounded-md" />
                    </>
                ) : issues.length === 0 ? (
                    <div className="flex items-center justify-center h-24 text-xs text-muted-foreground">No issues</div>
                ) : canDrag ? (
                    issues.map((issue) => <DraggableCard key={issue.index} issue={issue} labelMap={labelMap} user={user} repo={repo} />)
                ) : (
                    issues.map((issue) => (
                        <Link key={issue.index} href={`/${user}/${repo}/issues/${issue.index}`} className="block">
                            <IssueCard issue={issue} labelMap={labelMap} />
                        </Link>
                    ))
                )}
            </div>
        </div>
    );
}

export default function BoardPage() {
    const params = useParams();
    const user = (params.user ?? params.org) as string;
    const repo = params.repo as string;

    const apiUrl = user && repo ? `/api/repos/${user}/${repo}/issues` : null;
    const { data, isLoading, mutate } = useSWR<IssuesResponse>(apiUrl, jsonFetcher);
    const { data: labelsData } = useSWR<LabelsResponse>(user && repo ? `/api/repos/${user}/${repo}/labels` : null, jsonFetcher);
    const { data: permsData } = useSWR<PermissionsResponse>(user && repo ? `/api/repos/${user}/${repo}/permissions` : null, jsonFetcher);

    const canManage = permsData?.permissions.manageIssues ?? false;
    const isMobile = useIsMobile();
    const canDrag = canManage && !isMobile;

    const [pendingStatus, setPendingStatus] = useState<Record<number, string>>({});
    const [draggedIndex, setDraggedIndex] = useState<number | null>(null);
    const [draggedWidth, setDraggedWidth] = useState<number | null>(null);
    const [search, setSearch] = useState("");

    const localIssues = useMemo(() => {
        const issues = data?.issues ?? [];
        if (Object.keys(pendingStatus).length === 0) {
            return issues;
        }
        return issues.map((i) => (pendingStatus[i.index] !== undefined ? { ...i, status: pendingStatus[i.index] } : i));
    }, [data, pendingStatus]);

    const labelMap = new Map<string, string>((labelsData?.labels ?? []).map((l) => [l.name, l.color]));

    const PRIORITY_ORDER: Record<string, number> = { urgent: 0, high: 1, medium: 2, low: 3, none: 4 };

    const filtered = (search ? localIssues.filter((i) => i.title.toLowerCase().includes(search.toLowerCase())) : localIssues).sort(
        (a, b) => (PRIORITY_ORDER[a.priority] ?? 4) - (PRIORITY_ORDER[b.priority] ?? 4)
    );

    const sensors = useSensors(
        useSensor(PointerSensor, {
            activationConstraint: { distance: 8 },
        })
    );

    const draggedIssue = draggedIndex !== null ? (localIssues.find((i) => i.index === draggedIndex) ?? null) : null;

    function handleDragStart(event: DragStartEvent) {
        setDraggedIndex(parseInt(event.active.id as string));
        const rect = event.active.rect.current.initial;
        setDraggedWidth(rect ? rect.width : null);
    }

    async function handleDragEnd(event: DragEndEvent) {
        if (!canDrag) {
            setDraggedIndex(null);
            return;
        }

        const { active, over } = event;
        setDraggedIndex(null);

        if (!over) {
            return;
        }

        const issueIndex = parseInt(active.id as string);
        const newStatus = over.id as string;
        const issue = localIssues.find((i) => i.index === issueIndex);

        if (!issue || issue.status === newStatus) {
            return;
        }

        const clearPending = () =>
            setPendingStatus((prev) => {
                const next = { ...prev };
                delete next[issueIndex];
                return next;
            });

        // Optimistic update
        setPendingStatus((prev) => ({ ...prev, [issueIndex]: newStatus }));

        try {
            await patchJsonFetcher(`/api/repos/${user}/${repo}/issues/${issueIndex}`, { arg: { status: newStatus } });
            await mutate();
        } finally {
            clearPending();
        }
    }

    return (
        <div className="flex min-h-screen flex-col bg-background text-foreground md:h-screen md:overflow-hidden">
            <TopBar
                breadcrumb={[
                    { label: user, href: `/${user}` },
                    { label: repo, href: `/${user}/${repo}` },
                    { label: "Issues", href: `/${user}/${repo}/issues` },
                    { label: "Board" },
                ]}
                navLinks={[
                    { label: "Code", href: `/${user}/${repo}`, icon: <Code className="h-[18px] w-[18px]" /> },
                    {
                        label: "Issues",
                        href: `/${user}/${repo}/issues`,
                        icon: <AlertCircle className="h-[18px] w-[18px]" />,
                        active: true,
                    },
                ]}
            />

            <div className="flex shrink-0 items-center gap-3 border-b border-border px-4 py-3 sm:px-5">
                <div className="relative min-w-0 flex-1 sm:max-w-xs">
                    <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground pointer-events-none" />
                    <input
                        type="text"
                        value={search}
                        onChange={(e) => setSearch(e.target.value)}
                        placeholder="Filter issues…"
                        className="h-8 w-full rounded-md border-0 bg-secondary pr-3 pl-9 text-sm placeholder:text-muted-foreground focus:ring-1 focus:ring-ring focus:outline-none"
                    />
                </div>
                <Link
                    href={`/${user}/${repo}/issues`}
                    className="ml-auto flex h-8 shrink-0 items-center gap-2 whitespace-nowrap rounded-md border border-border px-3 text-sm text-muted-foreground transition-colors hover:bg-accent/50 hover:text-foreground"
                >
                    <List className="h-4 w-4" />
                    List view
                </Link>
            </div>

            <DndContext
                sensors={sensors}
                onDragStart={canDrag ? handleDragStart : undefined}
                onDragEnd={canDrag ? handleDragEnd : undefined}
            >
                <div className="flex flex-1 flex-col gap-6 px-4 py-5 md:min-h-0 md:flex-row md:gap-4 md:overflow-x-auto md:px-5">
                    {COLUMNS.map(({ status, label, Icon, color }) => (
                        <DroppableColumn
                            key={status}
                            status={status}
                            label={label}
                            Icon={Icon}
                            color={color}
                            issues={filtered.filter((i) => i.status === status)}
                            labelMap={labelMap}
                            user={user}
                            repo={repo}
                            isLoading={isLoading}
                            canDrag={canDrag}
                        />
                    ))}
                </div>

                <DragOverlay>
                    {canDrag && draggedIssue && (
                        <div style={{ width: draggedWidth ?? undefined }} className="overflow-hidden">
                            <IssueCard issue={draggedIssue} labelMap={labelMap} isDragOverlay />
                        </div>
                    )}
                </DragOverlay>
            </DndContext>
        </div>
    );
}

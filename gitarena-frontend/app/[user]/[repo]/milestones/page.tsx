"use client";

import { useState } from "react";
import Link from "next/link";
import { useParams } from "next/navigation";
import useSWR from "swr";
import useSWRMutation from "swr/mutation";
import { TopBar } from "@/components/top-bar";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { Skeleton } from "@/components/ui/skeleton";
import { jsonFetcher, postJsonFetcher, patchJsonFetcher, deleteFetcher } from "@/lib/fetchers";
import { AlertCircle, Code, Milestone, Plus, Pencil, Trash2, Check, X, RefreshCw, Calendar, CheckCircle2, Circle } from "lucide-react";
import { format, parseISO } from "date-fns";

interface MilestoneEntry {
    id: string;
    title: string;
    description: string | null;
    dueDate: string | null;
    closed: boolean;
    openIssues: number;
    closedIssues: number;
}

interface MilestonesResponse {
    milestones: MilestoneEntry[];
}

interface PermissionsResponse {
    permissions: { manageIssues: boolean };
}

interface CreateMilestoneRequest {
    title: string;
    description?: string;
    dueDate?: string;
}

interface UpdateMilestoneRequest {
    title?: string;
    description?: string;
    dueDate?: string | null;
    closed?: boolean;
}

type TabId = "open" | "closed";

type EditState = {
    id: string | null;
    title: string;
    description: string;
    dueDate: string;
};

function MilestoneCardSkeleton() {
    return (
        <div className="border border-border rounded-lg p-5 space-y-3">
            <div className="flex items-start justify-between gap-4">
                <Skeleton className="h-5 w-48" />
                <Skeleton className="h-7 w-24" />
            </div>
            <Skeleton className="h-4 w-full max-w-sm" />
            <Skeleton className="h-2 w-full rounded-full" />
            <div className="flex gap-4">
                <Skeleton className="h-4 w-24" />
                <Skeleton className="h-4 w-24" />
            </div>
        </div>
    );
}

function MilestoneForm({
    state,
    onChange,
    onSave,
    onCancel,
    isMutating,
    saveLabel,
}: {
    state: EditState;
    onChange: (s: EditState) => void;
    onSave: () => void;
    onCancel: () => void;
    isMutating: boolean;
    saveLabel: string;
}) {
    const canSave = state.title.trim().length > 0;

    return (
        <div className="px-5 py-4 space-y-4">
            <div className="grid grid-cols-1 gap-4 sm:grid-cols-[1fr_1fr_auto]">
                <div className="space-y-1.5">
                    <label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Title</label>
                    <input
                        type="text"
                        value={state.title}
                        onChange={(e) => onChange({ ...state, title: e.target.value })}
                        placeholder="e.g. v1.0 release"
                        className="w-full h-9 px-3 text-sm bg-background border border-border rounded-md focus:outline-none focus:ring-1 focus:ring-ring"
                        autoFocus
                    />
                </div>
                <div className="space-y-1.5">
                    <label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Description</label>
                    <input
                        type="text"
                        value={state.description}
                        onChange={(e) => onChange({ ...state, description: e.target.value })}
                        placeholder="Optional description"
                        className="w-full h-9 px-3 text-sm bg-background border border-border rounded-md focus:outline-none focus:ring-1 focus:ring-ring"
                    />
                </div>
                <div className="space-y-1.5">
                    <label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Due date</label>
                    <input
                        type="date"
                        value={state.dueDate}
                        onChange={(e) => onChange({ ...state, dueDate: e.target.value })}
                        className="h-9 px-3 text-sm bg-background border border-border rounded-md focus:outline-none focus:ring-1 focus:ring-ring"
                    />
                </div>
            </div>
            <div className="flex items-center gap-2">
                <Button size="sm" onClick={onSave} disabled={!canSave || isMutating} className="gap-2">
                    {isMutating ? <RefreshCw className="h-4 w-4 animate-spin" /> : <Check className="h-4 w-4" />}
                    {saveLabel}
                </Button>
                <Button size="sm" variant="ghost" onClick={onCancel} disabled={isMutating} className="gap-2 text-muted-foreground">
                    <X className="h-4 w-4" />
                    Cancel
                </Button>
            </div>
        </div>
    );
}

export default function MilestonesPage() {
    const params = useParams();
    const user = params.user as string;
    const repo = params.repo as string;

    const milestonesKey = user && repo ? `/api/repos/${user}/${repo}/milestones` : null;
    const permissionsKey = user && repo ? `/api/repos/${user}/${repo}/permissions` : null;

    const { data, isLoading, error } = useSWR<MilestonesResponse>(milestonesKey, jsonFetcher);
    const { data: permissionsData } = useSWR<PermissionsResponse>(permissionsKey, jsonFetcher);

    const canManage = permissionsData?.permissions.manageIssues ?? false;

    const { trigger: triggerCreate, isMutating: isCreating } = useSWRMutation(
        milestonesKey,
        (url: string, { arg }: { arg: CreateMilestoneRequest }) => postJsonFetcher<CreateMilestoneRequest, MilestoneEntry>(url, { arg })
    );

    const { trigger: triggerUpdate, isMutating: isUpdating } = useSWRMutation(
        milestonesKey,
        (url: string, { arg }: { arg: { id: string } & UpdateMilestoneRequest }) => {
            const { id, ...body } = arg;
            return patchJsonFetcher<UpdateMilestoneRequest, MilestoneEntry>(`${url}/${id}`, { arg: body });
        }
    );

    const { trigger: triggerDelete } = useSWRMutation(milestonesKey, (url: string, { arg }: { arg: string }) =>
        deleteFetcher(`${url}/${arg}`)
    );

    const [activeTab, setActiveTab] = useState<TabId>("open");
    const [editState, setEditState] = useState<EditState | null>(null);
    const [deleteConfirm, setDeleteConfirm] = useState<string | null>(null);

    const allMilestones = data?.milestones ?? [];
    const displayed = allMilestones.filter((m) => (activeTab === "open" ? !m.closed : m.closed));

    const openCount = allMilestones.filter((m) => !m.closed).length;
    const closedCount = allMilestones.filter((m) => m.closed).length;

    function startNew() {
        setEditState({ id: null, title: "", description: "", dueDate: "" });
        setDeleteConfirm(null);
    }

    function startEdit(m: MilestoneEntry) {
        setEditState({
            id: m.id,
            title: m.title,
            description: m.description ?? "",
            dueDate: m.dueDate ? m.dueDate.split("T")[0] : "",
        });
        setDeleteConfirm(null);
    }

    function cancelEdit() {
        setEditState(null);
    }

    async function saveEdit() {
        if (!editState || !editState.title.trim()) {
            return;
        }
        const dueDateIso = editState.dueDate ? new Date(editState.dueDate).toISOString() : undefined;
        if (editState.id === null) {
            await triggerCreate({
                title: editState.title.trim(),
                description: editState.description.trim() || undefined,
                dueDate: dueDateIso,
            });
        } else {
            await triggerUpdate({
                id: editState.id,
                title: editState.title.trim(),
                description: editState.description.trim(),
                dueDate: dueDateIso ?? null,
            });
        }
        setEditState(null);
    }

    async function toggleClosed(m: MilestoneEntry) {
        await triggerUpdate({ id: m.id, closed: !m.closed });
    }

    async function deleteMilestone(id: string) {
        await triggerDelete(id);
        setDeleteConfirm(null);
    }

    const isMutating = isCreating || isUpdating;

    return (
        <div className="flex flex-col h-screen bg-background text-foreground">
            <TopBar
                breadcrumb={[
                    { label: user, href: `/${user}` },
                    { label: repo, href: `/${user}/${repo}` },
                    { label: "Issues", href: `/${user}/${repo}/issues` },
                    { label: "Milestones" },
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

            <main className="flex-1 overflow-y-auto">
                <div className="max-w-4xl mx-auto px-6 py-8 space-y-6">
                    {/* Header */}
                    <div className="flex items-center justify-between">
                        <div>
                            <h1 className="text-2xl font-semibold flex items-center gap-2">
                                <Milestone className="h-5 w-5 text-muted-foreground" />
                                Milestones
                            </h1>
                            <p className="text-sm text-muted-foreground mt-1">
                                Track progress toward goals by grouping issues into milestones
                            </p>
                        </div>
                        {canManage && (
                            <Button size="sm" className="gap-2" onClick={startNew} disabled={editState?.id === null}>
                                <Plus className="h-4 w-4" />
                                New milestone
                            </Button>
                        )}
                    </div>

                    {/* New milestone form */}
                    {canManage && editState?.id === null && (
                        <div className="border border-border rounded-lg">
                            <div className="px-5 py-3 border-b border-border bg-card rounded-t-lg">
                                <span className="font-medium text-sm">New milestone</span>
                            </div>
                            <MilestoneForm
                                state={editState}
                                onChange={setEditState}
                                onSave={saveEdit}
                                onCancel={cancelEdit}
                                isMutating={isMutating}
                                saveLabel="Create milestone"
                            />
                        </div>
                    )}

                    {/* Tabs */}
                    <div className="flex items-center gap-1 border-b border-border">
                        <button
                            onClick={() => setActiveTab("open")}
                            className={`flex items-center gap-1.5 px-4 py-2 text-sm font-medium border-b-2 -mb-px transition-colors ${
                                activeTab === "open"
                                    ? "border-foreground text-foreground"
                                    : "border-transparent text-muted-foreground hover:text-foreground"
                            }`}
                        >
                            <Circle className="h-4 w-4" />
                            Open
                            <span className="ml-1 bg-secondary px-1.5 py-0.5 rounded-full text-xs">{openCount}</span>
                        </button>
                        <button
                            onClick={() => setActiveTab("closed")}
                            className={`flex items-center gap-1.5 px-4 py-2 text-sm font-medium border-b-2 -mb-px transition-colors ${
                                activeTab === "closed"
                                    ? "border-foreground text-foreground"
                                    : "border-transparent text-muted-foreground hover:text-foreground"
                            }`}
                        >
                            <CheckCircle2 className="h-4 w-4" />
                            Closed
                            <span className="ml-1 bg-secondary px-1.5 py-0.5 rounded-full text-xs">{closedCount}</span>
                        </button>
                    </div>

                    {/* Loading */}
                    {isLoading && (
                        <div className="space-y-3">
                            <MilestoneCardSkeleton />
                            <MilestoneCardSkeleton />
                            <MilestoneCardSkeleton />
                        </div>
                    )}

                    {/* Error */}
                    {error && !isLoading && (
                        <div className="border border-border rounded-lg py-8 flex flex-col items-center gap-2 text-center">
                            <p className="text-sm text-destructive">Failed to load milestones</p>
                        </div>
                    )}

                    {/* Empty */}
                    {!isLoading && !error && displayed.length === 0 && (
                        <div className="border border-border rounded-lg py-16 flex flex-col items-center gap-2 text-center">
                            <Milestone className="h-8 w-8 text-muted-foreground" />
                            <p className="text-sm font-medium">No {activeTab} milestones</p>
                            {activeTab === "open" && canManage && (
                                <p className="text-xs text-muted-foreground">Create a milestone to track progress toward a goal.</p>
                            )}
                        </div>
                    )}

                    {/* Milestone cards */}
                    {!isLoading && !error && displayed.length > 0 && (
                        <div className="space-y-3">
                            {displayed.map((m) => {
                                const total = m.openIssues + m.closedIssues;
                                const progress = total > 0 ? Math.round((m.closedIssues / total) * 100) : 0;
                                const isEditing = editState?.id === m.id;

                                return (
                                    <div key={m.id} className="border border-border rounded-lg overflow-hidden">
                                        {isEditing ? (
                                            <>
                                                <div className="px-5 py-3 border-b border-border bg-card">
                                                    <span className="font-medium text-sm">Edit milestone</span>
                                                </div>
                                                <MilestoneForm
                                                    state={editState!}
                                                    onChange={setEditState}
                                                    onSave={saveEdit}
                                                    onCancel={cancelEdit}
                                                    isMutating={isMutating}
                                                    saveLabel="Save changes"
                                                />
                                            </>
                                        ) : (
                                            <div className="p-5 space-y-3">
                                                <div className="flex items-start justify-between gap-4">
                                                    <div className="flex items-center gap-2 min-w-0">
                                                        <Link
                                                            href={`/${user}/${repo}/issues?milestone=${m.id}`}
                                                            className="font-medium hover:underline truncate"
                                                        >
                                                            {m.title}
                                                        </Link>
                                                        {m.closed && (
                                                            <span className="shrink-0 px-2 py-0.5 text-xs rounded-full bg-secondary text-muted-foreground">
                                                                closed
                                                            </span>
                                                        )}
                                                    </div>
                                                    {canManage && (
                                                        <div className="flex items-center gap-1 shrink-0">
                                                            {deleteConfirm === m.id ? (
                                                                <div className="flex items-center gap-2">
                                                                    <span className="text-xs text-muted-foreground">Delete?</span>
                                                                    <button
                                                                        onClick={() => deleteMilestone(m.id)}
                                                                        className="flex items-center gap-1 px-2 py-1 text-xs text-red-500 hover:text-red-400 border border-red-500/30 rounded-md transition-colors"
                                                                    >
                                                                        <Check className="h-3 w-3" />
                                                                        Yes
                                                                    </button>
                                                                    <button
                                                                        onClick={() => setDeleteConfirm(null)}
                                                                        className="flex items-center gap-1 px-2 py-1 text-xs text-muted-foreground hover:text-foreground border border-border rounded-md transition-colors"
                                                                    >
                                                                        <X className="h-3 w-3" />
                                                                        No
                                                                    </button>
                                                                </div>
                                                            ) : (
                                                                <>
                                                                    <button
                                                                        onClick={() => toggleClosed(m)}
                                                                        className="flex items-center gap-1.5 px-2.5 py-1.5 text-xs text-muted-foreground hover:text-foreground border border-transparent hover:border-border rounded-md transition-colors"
                                                                    >
                                                                        {m.closed ? (
                                                                            <>
                                                                                <Circle className="h-3.5 w-3.5" />
                                                                                Reopen
                                                                            </>
                                                                        ) : (
                                                                            <>
                                                                                <CheckCircle2 className="h-3.5 w-3.5" />
                                                                                Close
                                                                            </>
                                                                        )}
                                                                    </button>
                                                                    <button
                                                                        onClick={() => startEdit(m)}
                                                                        className="flex items-center gap-1.5 px-2.5 py-1.5 text-xs text-muted-foreground hover:text-foreground border border-transparent hover:border-border rounded-md transition-colors"
                                                                    >
                                                                        <Pencil className="h-3.5 w-3.5" />
                                                                        Edit
                                                                    </button>
                                                                    <button
                                                                        onClick={() => setDeleteConfirm(m.id)}
                                                                        className="flex items-center gap-1.5 px-2.5 py-1.5 text-xs text-red-500/70 hover:text-red-500 border border-transparent hover:border-red-500/30 rounded-md transition-colors"
                                                                    >
                                                                        <Trash2 className="h-3.5 w-3.5" />
                                                                        Delete
                                                                    </button>
                                                                </>
                                                            )}
                                                        </div>
                                                    )}
                                                </div>

                                                {m.description && <p className="text-sm text-muted-foreground">{m.description}</p>}

                                                <div className="space-y-1.5">
                                                    <div className="flex items-center justify-between text-xs text-muted-foreground">
                                                        <span>{progress}% complete</span>
                                                        <span>
                                                            {m.closedIssues} / {total} issues closed
                                                        </span>
                                                    </div>
                                                    <Progress value={progress} className="h-2" />
                                                </div>

                                                <div className="flex items-center gap-4 text-xs text-muted-foreground">
                                                    <span>
                                                        <span className="font-medium text-foreground">{m.openIssues}</span> open
                                                    </span>
                                                    <span>
                                                        <span className="font-medium text-foreground">{m.closedIssues}</span> closed
                                                    </span>
                                                    {m.dueDate && (
                                                        <span className="flex items-center gap-1">
                                                            <Calendar className="h-3.5 w-3.5" />
                                                            Due {format(parseISO(m.dueDate), "MMM d, yyyy")}
                                                        </span>
                                                    )}
                                                </div>
                                            </div>
                                        )}
                                    </div>
                                );
                            })}
                        </div>
                    )}
                </div>
            </main>
        </div>
    );
}

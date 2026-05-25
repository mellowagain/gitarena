"use client";

import { useState } from "react";
import Link from "next/link";
import { useParams, useRouter } from "next/navigation";
import useSWR from "swr";
import useSWRMutation from "swr/mutation";
import { TopBar } from "@/components/top-bar";
import { Button } from "@/components/ui/button";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { AlertCircle, GitMerge, Code, ArrowLeft, ChevronDown, Tag, User, CheckCircle2, X, Eye, Loader2, Milestone } from "lucide-react";
import { jsonFetcher, postJsonFetcher } from "@/lib/fetchers";
import { toast } from "sonner";
import { MarkdownRenderer } from "@/components/markdown-renderer";
import { PriorityIndicator, priorityConfig, type Priority } from "@/components/priority-indicator";

interface LabelsResponse {
    labels: { name: string; color: string }[];
}

interface MilestonesResponse {
    milestones: { id: string; title: string; closed: boolean; dueDate: string | null }[];
}

interface CollaboratorResponse {
    userId: string;
    username: string;
    accessLevel: string;
}

interface CreateIssueRequest {
    title: string;
    body: string;
    priority?: string;
    labels?: string[];
    assignees?: string[];
    milestoneId?: string;
}

interface IssueResponse {
    index: number;
    title: string;
}

export default function NewIssuePage() {
    const params = useParams();
    const router = useRouter();
    const user = params.user as string;
    const repo = params.repo as string;

    const [title, setTitle] = useState("");
    const [body, setBody] = useState("");
    const [preview, setPreview] = useState(false);
    const [labels, setLabels] = useState<string[]>([]);
    const [assignees, setAssignees] = useState<string[]>([]);
    const [priority, setPriority] = useState<string>("none");
    const [milestoneId, setMilestoneId] = useState<string | null>(null);

    const { data: labelsData } = useSWR<LabelsResponse>(user && repo ? `/api/repos/${user}/${repo}/labels` : null, jsonFetcher);

    const { data: collaboratorsData } = useSWR<CollaboratorResponse[]>(
        user && repo ? `/api/repos/${user}/${repo}/collaborators` : null,
        jsonFetcher
    );

    const { data: milestonesData } = useSWR<MilestonesResponse>(user && repo ? `/api/repos/${user}/${repo}/milestones` : null, jsonFetcher);

    const { trigger, isMutating } = useSWRMutation<IssueResponse, Error, string, CreateIssueRequest>(
        `/api/repos/${user}/${repo}/issues`,
        postJsonFetcher
    );

    const availableLabels = labelsData?.labels ?? [];
    const availableCollaborators = collaboratorsData ?? [];
    const availableMilestones = milestonesData?.milestones ?? [];

    function toggleLabel(name: string) {
        if (labels.includes(name)) {
            setLabels((prev) => prev.filter((l) => l !== name));
        } else {
            const sepIdx = name.indexOf("::");
            const scope = sepIdx !== -1 ? name.slice(0, sepIdx) : null;
            setLabels((prev) => {
                const without = scope !== null ? prev.filter((l) => !l.startsWith(scope + "::")) : prev;
                return [...without, name];
            });
        }
    }

    function toggleAssignee(userId: string) {
        setAssignees((prev) => (prev.includes(userId) ? prev.filter((a) => a !== userId) : [...prev, userId]));
    }

    async function handleSubmit() {
        if (!title.trim()) {
            return;
        }

        try {
            const result = await trigger({
                title: title.trim(),
                body,
                ...(priority !== "none" ? { priority } : {}),
                ...(labels.length > 0 ? { labels } : {}),
                ...(assignees.length > 0 ? { assignees } : {}),
                ...(milestoneId ? { milestoneId } : {}),
            });
            router.push(`/${user}/${repo}/issues/${result.index}`);
        } catch (err) {
            toast.error(err instanceof Error ? err.message : "Failed to create issue");
        }
    }

    return (
        <div className="flex flex-col h-screen overflow-hidden bg-background text-foreground font-sans">
            <TopBar
                breadcrumb={[
                    { label: user, href: `/${user}` },
                    { label: repo, href: `/${user}/${repo}` },
                    { label: "Issues", href: `/${user}/${repo}/issues` },
                    { label: "New Issue" },
                ]}
                navLinks={[
                    { label: "Code", href: `/${user}/${repo}`, icon: <Code className="h-[18px] w-[18px]" /> },
                    { label: "Issues", href: `/${user}/${repo}/issues`, icon: <AlertCircle className="h-[18px] w-[18px]" />, active: true },
                    { label: "Merge Requests", href: `/${user}/${repo}/merge-requests`, icon: <GitMerge className="h-[18px] w-[18px]" /> },
                ]}
                hasNotifications
            />

            <div className="flex-1 flex overflow-hidden">
                <main className="flex-1 overflow-y-auto">
                    <div className="max-w-4xl mx-auto px-6 py-8">
                        <Link
                            href={`/${user}/${repo}/issues`}
                            className="inline-flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors mb-6"
                        >
                            <ArrowLeft className="h-4 w-4" />
                            Back to issues
                        </Link>

                        <h1 className="text-2xl font-semibold mb-8">New Issue</h1>

                        <div className="mb-5">
                            <label className="text-sm font-medium block mb-1.5">
                                Title <span className="text-red-500">*</span>
                            </label>
                            <input
                                type="text"
                                value={title}
                                onChange={(e) => setTitle(e.target.value)}
                                placeholder="Short, descriptive title…"
                                className="w-full h-10 px-3 bg-card border border-border rounded-md text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
                            />
                        </div>

                        <div className="mb-8">
                            <label className="text-sm font-medium block mb-1.5">Description</label>
                            <div className="border border-border rounded-md overflow-hidden focus-within:ring-1 focus-within:ring-ring/50">
                                <div className="flex items-center border-b border-border">
                                    <button
                                        type="button"
                                        onClick={() => setPreview(false)}
                                        className={`px-3 py-1.5 text-xs font-medium transition-colors ${!preview ? "text-foreground border-b-2 border-foreground -mb-px" : "text-muted-foreground hover:text-foreground"}`}
                                    >
                                        Write
                                    </button>
                                    <button
                                        type="button"
                                        onClick={() => setPreview(true)}
                                        className={`px-3 py-1.5 text-xs font-medium transition-colors flex items-center gap-1.5 ${preview ? "text-foreground border-b-2 border-foreground -mb-px" : "text-muted-foreground hover:text-foreground"}`}
                                    >
                                        <Eye className="h-3 w-3" />
                                        Preview
                                    </button>
                                </div>
                                {preview ? (
                                    <div className="px-3 py-2 min-h-[240px]">
                                        {body.trim() ? (
                                            <MarkdownRenderer
                                                content={body}
                                                user={user}
                                                repo={repo}
                                                className="space-y-4 text-sm leading-relaxed"
                                            />
                                        ) : (
                                            <span className="text-muted-foreground italic">Nothing to preview.</span>
                                        )}
                                    </div>
                                ) : (
                                    <textarea
                                        value={body}
                                        onChange={(e) => setBody(e.target.value)}
                                        placeholder="Add a description… Markdown is supported."
                                        rows={10}
                                        className="w-full px-3 py-2 bg-transparent text-base resize-none focus:outline-none"
                                    />
                                )}
                                <div className="flex items-center px-3 py-2 border-t border-border bg-card/50">
                                    <span className="text-xs text-muted-foreground font-mono">Markdown</span>
                                </div>
                            </div>
                        </div>

                        <div className="flex items-center justify-between">
                            <Link href={`/${user}/${repo}/issues`}>
                                <Button variant="outline">Cancel</Button>
                            </Link>
                            <Button onClick={handleSubmit} disabled={!title.trim() || isMutating}>
                                {isMutating && <Loader2 className="h-3.5 w-3.5 mr-1.5 animate-spin" />}
                                Submit Issue
                            </Button>
                        </div>
                    </div>
                </main>

                <aside className="w-72 border-l border-border shrink-0 overflow-y-auto p-5 space-y-6">
                    <div>
                        <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">Assignees</h3>
                        <div className="space-y-1.5">
                            {assignees.map((uid) => {
                                const c = availableCollaborators.find((col) => col.userId === uid);
                                const name = c?.username ?? uid.slice(0, 8);
                                return (
                                    <div
                                        key={uid}
                                        className="group/assignee flex items-center gap-2 px-3 py-2 rounded-md border border-border text-sm"
                                    >
                                        <div className="flex h-5 w-5 items-center justify-center rounded-full bg-secondary text-[10px] font-medium shrink-0">
                                            {name[0].toUpperCase()}
                                        </div>
                                        <span className="flex-1">{name}</span>
                                        <button
                                            className="opacity-0 group-hover/assignee:opacity-100 transition-opacity"
                                            onClick={() => toggleAssignee(uid)}
                                            title="Remove assignee"
                                        >
                                            <X className="h-3.5 w-3.5 text-muted-foreground hover:text-foreground" />
                                        </button>
                                    </div>
                                );
                            })}
                            <DropdownMenu>
                                <DropdownMenuTrigger asChild>
                                    <button className="w-full flex items-center justify-center gap-2 px-3 py-2 border border-dashed border-border rounded-md text-sm text-muted-foreground hover:text-foreground hover:border-solid transition-colors">
                                        <User className="h-3.5 w-3.5" />
                                        Add assignee
                                    </button>
                                </DropdownMenuTrigger>
                                <DropdownMenuContent align="start" className="w-48">
                                    {availableCollaborators.filter((c) => !assignees.includes(c.userId)).length === 0 ? (
                                        <DropdownMenuItem disabled className="text-muted-foreground italic">
                                            No collaborators
                                        </DropdownMenuItem>
                                    ) : (
                                        availableCollaborators
                                            .filter((c) => !assignees.includes(c.userId))
                                            .map(({ userId, username }) => (
                                                <DropdownMenuItem
                                                    key={userId}
                                                    onClick={() => toggleAssignee(userId)}
                                                    className="flex items-center gap-2"
                                                >
                                                    <div className="flex h-5 w-5 items-center justify-center rounded-full bg-secondary text-[10px] font-medium shrink-0">
                                                        {username[0].toUpperCase()}
                                                    </div>
                                                    {username}
                                                </DropdownMenuItem>
                                            ))
                                    )}
                                </DropdownMenuContent>
                            </DropdownMenu>
                        </div>
                    </div>

                    <div>
                        <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">Labels</h3>
                        <div className="flex flex-wrap gap-2">
                            {labels.map((l) => {
                                const cfg = availableLabels.find((lb) => lb.name === l);
                                const color = cfg?.color ?? "#888888";
                                return (
                                    <span
                                        key={l}
                                        className="inline-flex items-center gap-1 px-2 py-0.5 text-xs border border-border rounded bg-secondary"
                                    >
                                        <span className="h-2 w-2 rounded-full shrink-0" style={{ backgroundColor: color }} />
                                        {l}
                                        <button
                                            onClick={() => toggleLabel(l)}
                                            className="text-muted-foreground hover:text-foreground ml-0.5"
                                        >
                                            <X className="h-3 w-3" />
                                        </button>
                                    </span>
                                );
                            })}
                            <DropdownMenu>
                                <DropdownMenuTrigger asChild>
                                    <button className="flex items-center gap-1 px-2 py-0.5 text-xs border border-dashed border-border rounded text-muted-foreground hover:text-foreground hover:border-solid transition-colors">
                                        <Tag className="h-3 w-3" />
                                        Add
                                    </button>
                                </DropdownMenuTrigger>
                                <DropdownMenuContent align="start" className="w-48">
                                    {availableLabels.filter((l) => !labels.includes(l.name)).length === 0 ? (
                                        <DropdownMenuItem disabled className="text-muted-foreground italic">
                                            No labels configured
                                        </DropdownMenuItem>
                                    ) : (
                                        availableLabels
                                            .filter((l) => !labels.includes(l.name))
                                            .map(({ name, color }) => (
                                                <DropdownMenuItem
                                                    key={name}
                                                    onClick={() => toggleLabel(name)}
                                                    className="flex items-center gap-2"
                                                >
                                                    <span
                                                        className="inline-block w-3 h-3 rounded-full shrink-0"
                                                        style={{ backgroundColor: color }}
                                                    />
                                                    {name}
                                                </DropdownMenuItem>
                                            ))
                                    )}
                                </DropdownMenuContent>
                            </DropdownMenu>
                        </div>
                    </div>

                    <div>
                        <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">Priority</h3>
                        <DropdownMenu>
                            <DropdownMenuTrigger asChild>
                                <button className="w-full flex items-center justify-between px-3 py-2 border border-border rounded-md hover:bg-accent/50 transition-colors text-sm">
                                    <div className="flex items-center gap-2">
                                        <PriorityIndicator priority={priority as Priority} />
                                        <span>{priorityConfig[priority as Priority].label}</span>
                                    </div>
                                    <ChevronDown className="h-4 w-4 text-muted-foreground" />
                                </button>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent align="start" className="w-40">
                                {(["none", "low", "medium", "high", "urgent"] as Priority[]).map((p) => (
                                    <DropdownMenuItem key={p} onClick={() => setPriority(p)} className="flex items-center gap-2">
                                        <PriorityIndicator priority={p} />
                                        <span className="flex-1">{priorityConfig[p].label}</span>
                                        {priority === p && <CheckCircle2 className="h-3.5 w-3.5 text-green-500" />}
                                    </DropdownMenuItem>
                                ))}
                            </DropdownMenuContent>
                        </DropdownMenu>
                    </div>

                    <div>
                        <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">Milestone</h3>
                        <DropdownMenu>
                            <DropdownMenuTrigger asChild>
                                <button className="w-full flex items-center justify-between px-3 py-2 border border-border rounded-md hover:bg-accent/50 transition-colors text-sm">
                                    <div className="flex items-center gap-2">
                                        <Milestone className="h-4 w-4 text-muted-foreground shrink-0" />
                                        <span>
                                            {milestoneId
                                                ? (availableMilestones.find((m) => m.id === milestoneId)?.title ?? "Unknown")
                                                : "No milestone"}
                                        </span>
                                    </div>
                                    <ChevronDown className="h-4 w-4 text-muted-foreground" />
                                </button>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent align="start" className="w-52">
                                <DropdownMenuItem onClick={() => setMilestoneId(null)} className="flex items-center gap-2">
                                    <span className="flex-1 text-muted-foreground italic">No milestone</span>
                                    {milestoneId === null && <CheckCircle2 className="h-3.5 w-3.5 text-green-500" />}
                                </DropdownMenuItem>
                                {availableMilestones.length === 0 ? (
                                    <DropdownMenuItem disabled className="text-muted-foreground italic">
                                        No milestones yet
                                    </DropdownMenuItem>
                                ) : (
                                    availableMilestones.map((m) => (
                                        <DropdownMenuItem
                                            key={m.id}
                                            onClick={() => setMilestoneId(m.id)}
                                            className="flex items-center gap-2"
                                        >
                                            <span className="flex-1">{m.title}</span>
                                            {milestoneId === m.id && <CheckCircle2 className="h-3.5 w-3.5 text-green-500" />}
                                        </DropdownMenuItem>
                                    ))
                                )}
                            </DropdownMenuContent>
                        </DropdownMenu>
                    </div>
                </aside>
            </div>
        </div>
    );
}

"use client";

import { useState, useCallback } from "react";
import { useParams, useRouter } from "next/navigation";
import Link from "next/link";
import useSWR from "swr";
import useSWRMutation from "swr/mutation";
import { jsonFetcher, patchJsonFetcher, postJsonFetcher, deleteFetcher } from "@/lib/fetchers";
import { formatDistanceToNow } from "date-fns";
import { uuidToDate } from "@/lib/utils";
import { TopBar } from "@/components/top-bar";
import { Button } from "@/components/ui/button";
import type { RepoMetadata } from "@/app/[user]/[repo]/page";
import { ArchivedBanner } from "@/components/archived-banner";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
    DropdownMenuSeparator,
    DropdownMenuLabel,
} from "@/components/ui/dropdown-menu";
import {
    AlertCircle,
    ChevronDown,
    CheckCircle2,
    Circle,
    CircleDot,
    MoreHorizontal,
    Code,
    ArrowLeft,
    Calendar,
    User,
    Tag,
    Link as LinkIcon,
    Edit,
    Trash2,
    Clock,
    Smile,
    XCircle,
    X,
    Eye,
    MessageSquare,
    Milestone,
    Loader2,
    Check,
} from "lucide-react";
import { Skeleton } from "@/components/ui/skeleton";
import { useAuth } from "@/hooks/use-auth";
import { MarkdownRenderer } from "@/components/markdown-renderer";
import { PriorityIndicator, priorityConfig, type Priority } from "@/components/priority-indicator";

interface IssueDetail {
    id: string;
    index: number;
    gitBugId: string;
    title: string;
    body: string;
    status: string;
    priority: string;
    labels: string[];
    commentCount: number;
    authorUsername: string;
    assignees: string[];
    milestone: { id: string; title: string; closed: boolean; dueDate: string | null } | null;
    reactions: ReactionGroup[];
    updatedAt: string;
}

interface ReactionGroup {
    emoji: string;
    count: number;
    reacted: boolean;
}

interface IssueComment {
    id: string;
    authorUsername: string;
    body: string;
    editedAt: string | null;
    reactions: ReactionGroup[];
}

interface IssueDetailResponse {
    issue: IssueDetail;
    comments: IssueComment[];
}

interface TimelineEvent {
    type: "created" | "status_changed" | "title_changed" | "label_added" | "label_removed";
    timestamp: number;
    authorUsername: string;
    oldTitle?: string;
    newTitle?: string;
    status?: string;
    label?: string;
}

interface IssueLabel {
    name: string;
    color: string;
}

interface LabelsResponse {
    labels: IssueLabel[];
}

interface CollaboratorsResponse {
    userId: string;
    username: string;
    accessLevel: string;
}

interface PermissionsResponse {
    permissions: {
        view: boolean;
        push: boolean;
        manageIssues: boolean;
        admin: boolean;
    };
}

interface MilestoneListItem {
    id: string;
    title: string;
    closed: boolean;
    dueDate: string | null;
    openIssues: number;
    closedIssues: number;
}

interface MilestonesResponse {
    milestones: MilestoneListItem[];
}

function LabelBadge({
    name,
    color,
    removable,
    removing,
    onRemove,
}: {
    name: string;
    color: string;
    removable?: boolean;
    removing?: boolean;
    onRemove?: () => void;
}) {
    const scopedIndex = name.indexOf("::");
    const isScoped = scopedIndex !== -1;
    const scopeKey = isScoped ? name.slice(0, scopedIndex) : null;
    const scopeValue = isScoped ? name.slice(scopedIndex + 2) : null;

    const removeButton = (
        <button
            className="ml-1.5 mr-1.5 opacity-0 group-hover/label:opacity-100 transition-opacity rounded"
            title="Remove label"
            style={{ color }}
            onClick={onRemove}
            disabled={removing}
        >
            {removing ? <Loader2 className="h-2.5 w-2.5 animate-spin" /> : <X className="h-2.5 w-2.5" />}
        </button>
    );

    if (isScoped) {
        return (
            <span className="group/label inline-flex items-center text-xs rounded overflow-hidden">
                <span className="px-2 py-0.5 font-medium" style={{ backgroundColor: `${color}35`, color }}>
                    {scopeKey}
                </span>
                <span className="px-2 py-0.5" style={{ backgroundColor: `${color}20`, color }}>
                    {scopeValue}
                </span>
                {removable && (
                    <span
                        className="grid grid-cols-[0fr] group-hover/label:grid-cols-[1fr] transition-all duration-150 self-stretch"
                        style={{ backgroundColor: `${color}20` }}
                    >
                        <span className="overflow-hidden flex items-center">{removeButton}</span>
                    </span>
                )}
            </span>
        );
    }

    return (
        <span className="group/label inline-flex items-center px-2 py-0.5 text-xs rounded" style={{ backgroundColor: `${color}20`, color }}>
            {name}
            {removable && (
                <span className="grid grid-cols-[0fr] group-hover/label:grid-cols-[1fr] transition-all duration-150">
                    <span className="overflow-hidden flex items-center">{removeButton}</span>
                </span>
            )}
        </span>
    );
}

function AuthorAvatar({ author }: { author: string }) {
    return (
        <div className="flex h-5 w-5 items-center justify-center rounded-full bg-secondary text-[10px] font-medium shrink-0">
            {author[0].toUpperCase()}
        </div>
    );
}

const EMOJI_CATEGORIES = [
    { label: "Classics", emojis: ["👍", "👎", "❤️", "🎉", "🚀", "🫶", "🙏", "🔥"] },
    { label: "Faces", emojis: ["😄", "😕", "😭", "😂", "🤬", "🥹", "🤣", "👀", "😈", "😔", "🤦"] },
    { label: "Exclamations", emojis: ["❓", "⁉️", "‼️", "💯", "✅", "❌"] },
    { label: "Modern", emojis: ["💀", "🤡", "🧢", "🐐", "🍔", "🥀"] },
    { label: "Combinations", emojis: ["😹😭✌️", "☝️🤓", "🕳️👨‍🦯"] },
];

function ReactionBar({
    reactions,
    onToggle,
    canReact,
}: {
    reactions: ReactionGroup[];
    onToggle: (emoji: string) => Promise<void>;
    canReact: boolean;
}) {
    const [pending, setPending] = useState<Set<string>>(new Set());

    const handleClick = useCallback(
        async (emoji: string) => {
            if (pending.has(emoji)) {
                return;
            }
            setPending((prev) => new Set(prev).add(emoji));
            try {
                await onToggle(emoji);
            } finally {
                setPending((prev) => {
                    const next = new Set(prev);
                    next.delete(emoji);
                    return next;
                });
            }
        },
        [pending, onToggle]
    );

    return (
        <div className="flex items-center flex-wrap gap-1.5 mt-3">
            {reactions.map((r) => (
                <button
                    key={r.emoji}
                    onClick={canReact ? () => handleClick(r.emoji) : undefined}
                    disabled={pending.has(r.emoji)}
                    className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs border transition-colors ${
                        r.reacted
                            ? "bg-accent border-primary/40 text-foreground"
                            : "bg-transparent border-border text-muted-foreground hover:bg-accent"
                    } ${pending.has(r.emoji) ? "animate-pulse opacity-60 cursor-wait" : ""} ${!canReact ? "cursor-default" : ""}`}
                >
                    <span>{r.emoji}</span>
                    <span>{r.count}</span>
                </button>
            ))}
            {canReact && (
                <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                        <button className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs border border-border text-muted-foreground hover:bg-accent transition-colors">
                            <Smile className="h-3 w-3" />
                        </button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="start" className="p-2 w-56">
                        {EMOJI_CATEGORIES.map((category, i) => (
                            <div key={category.label}>
                                {i > 0 && <DropdownMenuSeparator />}
                                <DropdownMenuLabel className="px-1 py-0.5 text-xs text-muted-foreground">
                                    {category.label}
                                </DropdownMenuLabel>
                                <div className="flex gap-0.5 flex-wrap py-1">
                                    {category.emojis.map((emoji) => (
                                        <button
                                            key={emoji}
                                            onClick={() => handleClick(emoji)}
                                            disabled={pending.has(emoji)}
                                            className={`p-1.5 text-base rounded hover:bg-accent transition-colors ${pending.has(emoji) ? "animate-pulse opacity-60 cursor-wait" : ""}`}
                                        >
                                            {emoji}
                                        </button>
                                    ))}
                                </div>
                            </div>
                        ))}
                    </DropdownMenuContent>
                </DropdownMenu>
            )}
        </div>
    );
}

function CommentBlock({
    comment,
    user,
    repo,
    onEdit,
    onDelete,
    onToggleReaction,
    canEdit,
    canReact,
}: {
    comment: IssueComment;
    user: string;
    repo: string;
    onEdit: (id: string, body: string) => void;
    onDelete: (id: string) => void;
    onToggleReaction: (commentId: string, emoji: string) => Promise<void>;
    canEdit: boolean;
    canReact: boolean;
}) {
    return (
        <div className="group pl-4 border-l-2 transition-colors border-border hover:border-muted-foreground/40">
            <div className="flex items-center gap-2 mb-2">
                <AuthorAvatar author={comment.authorUsername} />
                <span className="text-sm font-medium">{comment.authorUsername}</span>
                <span className="text-xs text-muted-foreground">{formatDistanceToNow(uuidToDate(comment.id), { addSuffix: true })}</span>
                {comment.editedAt && (
                    <>
                        <span className="text-muted-foreground/40">·</span>
                        <span className="text-xs text-muted-foreground">
                            Edited {formatDistanceToNow(new Date(comment.editedAt), { addSuffix: true })}
                        </span>
                    </>
                )}
                <div className="flex items-center gap-1 ml-auto opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none group-hover:pointer-events-auto">
                    {canEdit && (
                        <DropdownMenu>
                            <DropdownMenuTrigger asChild>
                                <button className="p-1 hover:bg-accent rounded transition-colors">
                                    <MoreHorizontal className="h-3.5 w-3.5 text-muted-foreground" />
                                </button>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent align="end">
                                <DropdownMenuItem onClick={() => onEdit(comment.id, comment.body)}>
                                    <Edit className="h-4 w-4 mr-2" />
                                    Edit
                                </DropdownMenuItem>
                                <DropdownMenuItem className="text-destructive" onClick={() => onDelete(comment.id)}>
                                    <Trash2 className="h-4 w-4 mr-2" />
                                    Delete
                                </DropdownMenuItem>
                            </DropdownMenuContent>
                        </DropdownMenu>
                    )}
                </div>
            </div>
            <MarkdownRenderer content={comment.body} user={user} repo={repo} className="space-y-4 text-sm leading-relaxed" />
            <ReactionBar reactions={comment.reactions} onToggle={(emoji) => onToggleReaction(comment.id, emoji)} canReact={canReact} />
        </div>
    );
}

function CommentComposer({
    label = "Comment",
    initialText = "",
    user = "",
    repo = "",
    extra,
    onSubmit,
    onCancel,
}: {
    label?: string;
    initialText?: string;
    user?: string;
    repo?: string;
    extra?: React.ReactNode;
    onSubmit: (text: string) => Promise<void>;
    onCancel?: () => void;
}) {
    const [text, setText] = useState(initialText);
    const [preview, setPreview] = useState(false);
    const [isSubmitting, setIsSubmitting] = useState(false);

    const handleSubmit = async () => {
        if (!text.trim() || isSubmitting) {
            return;
        }
        setIsSubmitting(true);
        try {
            await onSubmit(text);
            setText("");
            setPreview(false);
        } finally {
            setIsSubmitting(false);
        }
    };

    return (
        <div className="flex items-start gap-3">
            <div className="flex-1 border border-border rounded-md overflow-hidden focus-within:ring-1 focus-within:ring-ring/50">
                <div className="flex items-center border-b border-border">
                    <button
                        onClick={() => setPreview(false)}
                        className={`px-3 py-1.5 text-xs font-medium transition-colors ${!preview ? "text-foreground border-b-2 border-foreground -mb-px" : "text-muted-foreground hover:text-foreground"}`}
                    >
                        Write
                    </button>
                    <button
                        onClick={() => setPreview(true)}
                        className={`px-3 py-1.5 text-xs font-medium transition-colors flex items-center gap-1.5 ${preview ? "text-foreground border-b-2 border-foreground -mb-px" : "text-muted-foreground hover:text-foreground"}`}
                    >
                        <Eye className="h-3 w-3" />
                        Preview
                    </button>
                </div>
                {preview ? (
                    <div className="px-3 py-2 min-h-[160px]">
                        {text.trim() ? (
                            <MarkdownRenderer content={text} user={user} repo={repo} className="space-y-4 text-sm leading-relaxed" />
                        ) : (
                            <span className="text-muted-foreground italic">Nothing to preview.</span>
                        )}
                    </div>
                ) : (
                    <textarea
                        value={text}
                        onChange={(e) => setText(e.target.value)}
                        placeholder="Leave a comment... Markdown supported"
                        rows={6}
                        className="w-full px-3 py-2 bg-transparent text-base resize-none focus:outline-none"
                    />
                )}
                <div className="flex items-center justify-between px-3 py-2 border-t border-border bg-card/50">
                    <span className="text-xs text-muted-foreground font-mono">Markdown</span>
                    <div className="flex items-center gap-2">
                        {extra}
                        {onCancel && (
                            <Button size="sm" variant="ghost" onClick={onCancel}>
                                Cancel
                            </Button>
                        )}
                        <Button size="sm" disabled={!text.trim() || isSubmitting} onClick={handleSubmit}>
                            {isSubmitting && <Loader2 className="h-3.5 w-3.5 mr-1.5 animate-spin" />}
                            {label}
                        </Button>
                    </div>
                </div>
            </div>
        </div>
    );
}

function TimelineLabel({ name, color }: { name: string; color?: string }) {
    const scopedIndex = name.indexOf("::");
    const isScoped = scopedIndex !== -1;

    if (isScoped) {
        const scopeKey = name.slice(0, scopedIndex);
        const scopeValue = name.slice(scopedIndex + 2);
        return (
            <span className="inline-flex max-w-full items-center overflow-hidden rounded text-xs">
                <span
                    className="min-w-0 truncate px-1.5 py-0.5 font-medium"
                    style={color ? { backgroundColor: `${color}35`, color } : undefined}
                >
                    {scopeKey}
                </span>
                <span className="min-w-0 truncate px-1.5 py-0.5" style={color ? { backgroundColor: `${color}20`, color } : undefined}>
                    {scopeValue}
                </span>
            </span>
        );
    }

    return (
        <span
            className="inline-flex max-w-full items-center truncate rounded px-1.5 py-0.5 text-xs font-medium"
            style={color ? { backgroundColor: `${color}20`, color } : undefined}
        >
            {name}
        </span>
    );
}

function TimelineEventItem({ event, labelColor }: { event: TimelineEvent; labelColor?: string }) {
    const time = new Date(event.timestamp * 1000);
    const relative = formatDistanceToNow(time, { addSuffix: true });

    let icon: React.ReactNode;
    let action: string;
    let detail: React.ReactNode | undefined;

    switch (event.type) {
        case "status_changed":
            if (event.status === "open") {
                icon = <Circle className="h-4 w-4 text-green-500 shrink-0" />;
                action = "reopened this issue";
            } else {
                icon = <XCircle className="h-4 w-4 text-muted-foreground shrink-0" />;
                action = "closed this issue";
            }
            break;
        case "title_changed":
            icon = <Edit className="h-4 w-4 text-muted-foreground shrink-0" />;
            action = "changed the title to";
            detail = <span className="px-1.5 py-0.5 bg-secondary rounded text-xs font-medium text-foreground/80">{event.newTitle}</span>;
            break;
        case "label_added":
            icon = <Tag className="h-4 w-4 text-muted-foreground shrink-0" />;
            action = "added label";
            detail = <TimelineLabel name={event.label!} color={labelColor} />;
            break;
        case "label_removed":
            icon = <Tag className="h-4 w-4 text-muted-foreground shrink-0" />;
            action = "removed label";
            detail = <TimelineLabel name={event.label!} color={labelColor} />;
            break;
        default:
            return null;
    }

    return (
        <div className="flex items-start gap-2 py-1 text-sm text-muted-foreground">
            {icon}
            <div className="min-w-0 flex-1">
                <div className="min-w-0 leading-5">
                    <Link href={`/${event.authorUsername}`} className="font-medium text-foreground/70 hover:underline">
                        {event.authorUsername}
                    </Link>{" "}
                    <span>{action}</span>
                    {detail && <> {detail} </>}
                </div>
                <span className="mt-1 block text-xs sm:hidden">{relative}</span>
            </div>
            <span className="ml-auto hidden shrink-0 text-xs sm:block">{relative}</span>
        </div>
    );
}

function DetailSkeleton() {
    return (
        <div className="max-w-4xl mx-auto px-6 py-8">
            <Skeleton className="h-4 w-32 mb-6" />
            <Skeleton className="h-8 w-2/3 mb-3" />
            <Skeleton className="h-4 w-1/2 mb-8" />
            <Skeleton className="h-32 w-full mb-8" />
        </div>
    );
}

export default function IssuePage() {
    const params = useParams();
    const router = useRouter();
    const user = (params.user ?? params.org) as string;
    const repo = params.repo as string;
    const index = params.id as string;

    const { user: authUser, isAuthenticated } = useAuth();

    const apiBase = `/api/repos/${user}/${repo}/issues/${index}`;
    const { data, isLoading, mutate } = useSWR<IssueDetailResponse>(user && repo ? apiBase : null, jsonFetcher);
    const { data: labelsData } = useSWR<LabelsResponse>(user && repo ? `/api/repos/${user}/${repo}/labels` : null, jsonFetcher);
    const { data: collabData } = useSWR<CollaboratorsResponse[]>(
        user && repo ? `/api/repos/${user}/${repo}/collaborators` : null,
        jsonFetcher
    );
    const { data: permsData } = useSWR<PermissionsResponse>(user && repo ? `/api/repos/${user}/${repo}/permissions` : null, jsonFetcher);
    const { data: repoMeta } = useSWR<RepoMetadata>(user && repo ? `/api/repos/${user}/${repo}` : null, jsonFetcher);
    const { data: milestonesData } = useSWR<MilestonesResponse>(user && repo ? `/api/repos/${user}/${repo}/milestones` : null, jsonFetcher);
    const { data: timelineData, mutate: mutateTimeline } = useSWR<TimelineEvent[]>(
        user && repo ? `${apiBase}/timeline` : null,
        jsonFetcher
    );

    const labelMap = new Map<string, string>((labelsData?.labels ?? []).map((l) => [l.name, l.color]));
    const allLabels = labelsData?.labels ?? [];
    const collaborators = collabData ?? [];

    const { trigger: addComment } = useSWRMutation(`${apiBase}/comments`, (url, { arg }: { arg: { body: string } }) =>
        postJsonFetcher(url, { arg })
    );
    const { trigger: updateIssue } = useSWRMutation(apiBase, (url, { arg }: { arg: Record<string, unknown> }) =>
        patchJsonFetcher(url, { arg })
    );
    const { trigger: deleteIssue } = useSWRMutation(apiBase, (url) => deleteFetcher(url));
    const { trigger: deleteComment } = useSWRMutation(apiBase, (url, { arg }: { arg: { commentId: string } }) =>
        deleteFetcher(`${url}/comments/${arg.commentId}`)
    );

    const [editingComment, setEditingComment] = useState<{ id: string; body: string } | null>(null);
    const { trigger: editComment } = useSWRMutation(apiBase, (url, { arg }: { arg: { commentId: string; body: string } }) =>
        patchJsonFetcher(`${url}/comments/${arg.commentId}`, { arg: { body: arg.body } })
    );

    const { trigger: toggleIssueReaction } = useSWRMutation(`${apiBase}/reactions`, (url, { arg }: { arg: { emoji: string } }) =>
        postJsonFetcher(url, { arg })
    );
    const { trigger: toggleCommentReaction } = useSWRMutation(apiBase, (url, { arg }: { arg: { commentId: string; emoji: string } }) =>
        postJsonFetcher(`${url}/comments/${arg.commentId}/reactions`, { arg: { emoji: arg.emoji } })
    );

    const [editingTitle, setEditingTitle] = useState<string | null>(null);
    const [editingBody, setEditingBody] = useState(false);
    const [isDeleting, setIsDeleting] = useState(false);
    const [isTogglingStatus, setIsTogglingStatus] = useState(false);
    const [statusOpen, setStatusOpen] = useState(false);
    const [isAddingLabel, setIsAddingLabel] = useState(false);
    const [removingLabel, setRemovingLabel] = useState<string | null>(null);
    const [isAddingAssignee, setIsAddingAssignee] = useState(false);
    const [removingAssignee, setRemovingAssignee] = useState<string | null>(null);
    const [isUpdatingPriority, setIsUpdatingPriority] = useState(false);
    const [priorityOpen, setPriorityOpen] = useState(false);
    const [isUpdatingMilestone, setIsUpdatingMilestone] = useState(false);
    const [milestoneOpen, setMilestoneOpen] = useState(false);

    const allMilestones = milestonesData?.milestones ?? [];

    const handleToggleIssueReaction = async (emoji: string) => {
        await toggleIssueReaction({ emoji });
        await mutate();
    };

    const handleToggleCommentReaction = async (commentId: string, emoji: string) => {
        await toggleCommentReaction({ commentId, emoji });
        await mutate();
    };

    const handleAddComment = async (text: string) => {
        await addComment({ body: text });
        await mutate();
    };

    const handleSetStatus = async (status: string) => {
        if (!issue || isTogglingStatus) {
            return;
        }
        setIsTogglingStatus(true);
        try {
            await updateIssue({ status });
            await Promise.all([mutate(), mutateTimeline()]);
        } finally {
            setIsTogglingStatus(false);
        }
    };

    const handleDeleteIssue = async () => {
        setIsDeleting(true);
        try {
            await deleteIssue();
            router.push(`/${user}/${repo}/issues`);
        } finally {
            setIsDeleting(false);
        }
    };

    const handleSaveTitle = async (newTitle: string) => {
        if (!newTitle.trim()) {
            return;
        }
        await updateIssue({ title: newTitle.trim() });
        setEditingTitle(null);
        await Promise.all([mutate(), mutateTimeline()]);
    };

    const handleSaveBody = async (newBody: string) => {
        await updateIssue({ body: newBody });
        setEditingBody(false);
        await mutate();
    };

    const handleAddLabel = async (name: string) => {
        const sepIdx = name.indexOf("::");
        const scope = sepIdx !== -1 ? name.slice(0, sepIdx) : null;
        const toRemove = scope !== null ? (issue?.labels ?? []).filter((l) => l.startsWith(scope + "::")) : [];
        setIsAddingLabel(true);
        try {
            await updateIssue({ labelsAdd: [name], ...(toRemove.length > 0 ? { labelsRemove: toRemove } : {}) });
            await Promise.all([mutate(), mutateTimeline()]);
        } finally {
            setIsAddingLabel(false);
        }
    };

    const handleRemoveLabel = async (name: string) => {
        setRemovingLabel(name);
        try {
            await updateIssue({ labelsRemove: [name] });
            await Promise.all([mutate(), mutateTimeline()]);
        } finally {
            setRemovingLabel(null);
        }
    };

    const handleAddAssignee = async (userId: string) => {
        if (!issue) {
            return;
        }
        const currentIds = collaborators.filter((c) => issue.assignees.includes(c.username)).map((c) => c.userId);
        setIsAddingAssignee(true);
        try {
            await updateIssue({ assignees: [...currentIds, userId] });
            await mutate();
        } finally {
            setIsAddingAssignee(false);
        }
    };

    const handleRemoveAssignee = async (username: string) => {
        if (!issue) {
            return;
        }
        const remainingIds = collaborators
            .filter((c) => issue.assignees.includes(c.username) && c.username !== username)
            .map((c) => c.userId);
        setRemovingAssignee(username);
        try {
            await updateIssue({ assignees: remainingIds });
            await mutate();
        } finally {
            setRemovingAssignee(null);
        }
    };

    const handleSetPriority = async (priority: string) => {
        setIsUpdatingPriority(true);
        try {
            await updateIssue({ priority });
            await mutate();
        } finally {
            setIsUpdatingPriority(false);
        }
    };

    const handleSetMilestone = async (milestoneId: string | null) => {
        setIsUpdatingMilestone(true);
        try {
            await updateIssue({ milestoneId });
            await mutate();
        } finally {
            setIsUpdatingMilestone(false);
            setMilestoneOpen(false);
        }
    };

    const handleDeleteComment = async (commentId: string) => {
        await deleteComment({ commentId });
        await mutate();
    };

    const handleEditComment = async (body: string) => {
        if (!editingComment) {
            return;
        }
        await editComment({ commentId: editingComment.id, body });
        setEditingComment(null);
        await mutate();
    };
    const comments = data?.comments ?? [];
    const issue = data?.issue;

    type FeedItem =
        | { kind: "comment"; timestamp: number; comment: IssueComment }
        | { kind: "event"; timestamp: number; event: TimelineEvent };

    const feedItems: FeedItem[] = [
        ...comments.map((c) => ({ kind: "comment" as const, timestamp: uuidToDate(c.id).getTime(), comment: c })),
        ...(timelineData ?? [])
            .filter((e) => e.type !== "created")
            .map((e) => ({ kind: "event" as const, timestamp: e.timestamp * 1000, event: e })),
    ].sort((a, b) => a.timestamp - b.timestamp);

    const isArchived = repoMeta?.archivedAt != null;
    const canManage = (permsData?.permissions.manageIssues ?? false) && !isArchived;
    const canEditIssue = (canManage || (isAuthenticated && authUser?.username === issue?.authorUsername)) && !isArchived;

    const statusInfo = (() => {
        switch (issue?.status) {
            case "in_progress":
                return { icon: CircleDot, label: "In Progress", color: "text-yellow-500" };
            case "completed":
                return { icon: CheckCircle2, label: "Completed", color: "text-muted-foreground" };
            case "not_planned":
                return { icon: XCircle, label: "Not Planned", color: "text-muted-foreground" };
            default:
                return { icon: Circle, label: "Open", color: "text-green-500" };
        }
    })();
    const StatusIcon = statusInfo.icon;

    const PRIORITY_OPTIONS = (["none", "low", "medium", "high", "urgent"] as Priority[]).map((v) => ({
        value: v,
        label: priorityConfig[v].label,
    }));

    return (
        <div className="flex min-h-screen flex-col bg-background lg:h-screen">
            <TopBar
                breadcrumb={[
                    { label: user, href: `/${user}` },
                    { label: repo, href: `/${user}/${repo}` },
                    { label: "Issues", href: `/${user}/${repo}/issues` },
                    { label: `#${index}` },
                ]}
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

            <div className="flex flex-1 flex-col lg:min-h-0 lg:flex-row lg:overflow-hidden">
                <main className="w-full min-w-0 lg:flex-1 lg:overflow-y-auto">
                    {isLoading || !issue ? (
                        <DetailSkeleton />
                    ) : (
                        <div className="max-w-4xl mx-auto px-6 py-8">
                            <Link
                                href={`/${user}/${repo}/issues`}
                                className="inline-flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors mb-6"
                            >
                                <ArrowLeft className="h-4 w-4" />
                                Back to issues
                            </Link>

                            <div className="mb-8">
                                <div className="flex items-start gap-4 mb-3">
                                    {editingTitle !== null ? (
                                        <div className="flex-1 flex items-center gap-2">
                                            <input
                                                className="flex-1 bg-transparent border border-border rounded px-3 py-1.5 text-xl font-semibold focus:outline-none focus:ring-1 focus:ring-ring/50"
                                                value={editingTitle}
                                                onChange={(e) => setEditingTitle(e.target.value)}
                                                onKeyDown={(e) => {
                                                    if (e.key === "Enter") {
                                                        void handleSaveTitle(editingTitle);
                                                    }
                                                    if (e.key === "Escape") {
                                                        setEditingTitle(null);
                                                    }
                                                }}
                                                autoFocus
                                            />
                                            <Button size="sm" onClick={() => handleSaveTitle(editingTitle)}>
                                                Save
                                            </Button>
                                            <Button size="sm" variant="ghost" onClick={() => setEditingTitle(null)}>
                                                Cancel
                                            </Button>
                                        </div>
                                    ) : (
                                        <h1 className="text-2xl font-semibold flex-1 leading-snug">
                                            <span className="text-muted-foreground font-normal">#{issue.index}</span> {issue.title}
                                        </h1>
                                    )}
                                    <DropdownMenu>
                                        <DropdownMenuTrigger asChild>
                                            <Button variant="ghost" size="sm" className="shrink-0" disabled={isDeleting}>
                                                <MoreHorizontal className="h-5 w-5" />
                                            </Button>
                                        </DropdownMenuTrigger>
                                        <DropdownMenuContent align="end">
                                            {canEditIssue && (
                                                <DropdownMenuItem onClick={() => setEditingTitle(issue.title)}>
                                                    <Edit className="h-4 w-4 mr-2" />
                                                    Edit title
                                                </DropdownMenuItem>
                                            )}
                                            <DropdownMenuItem onClick={() => navigator.clipboard.writeText(window.location.href)}>
                                                <LinkIcon className="h-4 w-4 mr-2" />
                                                Copy link
                                            </DropdownMenuItem>
                                            {canManage && (
                                                <>
                                                    <DropdownMenuSeparator />
                                                    <DropdownMenuItem
                                                        className="text-destructive"
                                                        disabled={isDeleting}
                                                        onClick={handleDeleteIssue}
                                                    >
                                                        <Trash2 className="h-4 w-4 mr-2" />
                                                        {isDeleting ? "Deleting…" : "Delete"}
                                                    </DropdownMenuItem>
                                                </>
                                            )}
                                        </DropdownMenuContent>
                                    </DropdownMenu>
                                </div>
                                <div className="flex items-center gap-2.5 text-sm text-muted-foreground flex-wrap">
                                    <StatusIcon className={`h-4 w-4 ${statusInfo.color}`} />
                                    <span className={`font-medium ${statusInfo.color}`}>{statusInfo.label}</span>
                                    <span className="text-muted-foreground/40">·</span>
                                    <span>
                                        Opened by <span className="text-foreground font-medium">{issue.authorUsername}</span>{" "}
                                        {formatDistanceToNow(uuidToDate(issue.id), { addSuffix: true })}
                                    </span>
                                    <span className="text-muted-foreground/40">·</span>
                                    <span className="flex items-center gap-1">
                                        <MessageSquare className="h-3.5 w-3.5" />
                                        {comments.length} comments
                                    </span>
                                </div>
                            </div>

                            <div className="group/desc mb-8 pl-5 border-l-4 border-muted-foreground/20 hover:border-muted-foreground/40 transition-colors">
                                <div className="flex items-center gap-2 mb-3">
                                    <AuthorAvatar author={issue.authorUsername} />
                                    <span className="text-sm font-medium">{issue.authorUsername}</span>
                                    <span className="text-xs text-muted-foreground">
                                        {formatDistanceToNow(uuidToDate(issue.id), { addSuffix: true })}
                                    </span>
                                    {canEditIssue && (
                                        <button
                                            className="ml-auto opacity-0 group-hover/desc:opacity-100 transition-opacity p-1 hover:bg-accent rounded"
                                            onClick={() => setEditingBody(true)}
                                            title="Edit description"
                                        >
                                            <Edit className="h-3.5 w-3.5 text-muted-foreground" />
                                        </button>
                                    )}
                                </div>
                                {editingBody ? (
                                    <CommentComposer
                                        label="Save"
                                        initialText={issue.body}
                                        user={user}
                                        repo={repo}
                                        onSubmit={handleSaveBody}
                                        onCancel={() => setEditingBody(false)}
                                    />
                                ) : (
                                    <>
                                        {issue.body.trim() ? (
                                            <MarkdownRenderer content={issue.body} user={user} repo={repo} className="space-y-4 text-sm" />
                                        ) : (
                                            <p className="text-sm italic text-muted-foreground">No description provided.</p>
                                        )}
                                        <ReactionBar
                                            reactions={issue.reactions}
                                            onToggle={handleToggleIssueReaction}
                                            canReact={isAuthenticated}
                                        />
                                    </>
                                )}
                            </div>

                            {feedItems.length > 0 && (
                                <div className="mb-8">
                                    <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-5">Activity</h3>
                                    <div className="space-y-3">
                                        {feedItems.map((item, i) => {
                                            if (item.kind === "comment") {
                                                const comment = item.comment;
                                                return editingComment?.id === comment.id ? (
                                                    <CommentComposer
                                                        key={comment.id}
                                                        label="Save"
                                                        initialText={editingComment.body}
                                                        user={user}
                                                        repo={repo}
                                                        onSubmit={handleEditComment}
                                                        onCancel={() => setEditingComment(null)}
                                                    />
                                                ) : (
                                                    <div key={comment.id} className="mb-4">
                                                        <CommentBlock
                                                            comment={comment}
                                                            user={user}
                                                            repo={repo}
                                                            onEdit={(id, body) => setEditingComment({ id, body })}
                                                            onDelete={handleDeleteComment}
                                                            onToggleReaction={handleToggleCommentReaction}
                                                            canEdit={
                                                                !isArchived &&
                                                                isAuthenticated &&
                                                                (comment.authorUsername === authUser?.username || canManage)
                                                            }
                                                            canReact={isAuthenticated}
                                                        />
                                                    </div>
                                                );
                                            } else {
                                                return (
                                                    <TimelineEventItem
                                                        key={`event-${i}`}
                                                        event={item.event}
                                                        labelColor={item.event.label ? labelMap.get(item.event.label) : undefined}
                                                    />
                                                );
                                            }
                                        })}
                                    </div>
                                </div>
                            )}

                            <div className="border-t border-border pt-6">
                                {isAuthenticated && !isArchived && (
                                    <CommentComposer label="Comment" user={user} repo={repo} onSubmit={handleAddComment} />
                                )}
                            </div>
                        </div>
                    )}
                </main>

                <aside className="w-full shrink-0 space-y-6 border-t border-border p-5 lg:w-72 lg:overflow-y-auto lg:border-t-0 lg:border-l">
                    {isLoading || !issue ? (
                        <>
                            <Skeleton className="h-20 w-full" />
                            <Skeleton className="h-20 w-full" />
                        </>
                    ) : (
                        <>
                            <div>
                                <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-2">git-bug ID</h3>
                                <code className="text-xs text-muted-foreground font-mono bg-muted px-2 py-1 rounded">
                                    {issue.gitBugId.slice(0, 7)}
                                </code>
                            </div>

                            <div>
                                <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">Status</h3>
                                {canManage ? (
                                    <DropdownMenu open={isTogglingStatus ? false : statusOpen} onOpenChange={setStatusOpen}>
                                        <DropdownMenuTrigger asChild>
                                            <button className="w-full flex items-center justify-between px-3 py-2 border border-border rounded-md hover:bg-accent/50 transition-colors text-sm">
                                                <div className="flex items-center gap-2">
                                                    <StatusIcon className={`h-4 w-4 ${statusInfo.color}`} />
                                                    {statusInfo.label}
                                                </div>
                                                {isTogglingStatus ? (
                                                    <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
                                                ) : (
                                                    <ChevronDown className="h-4 w-4 text-muted-foreground" />
                                                )}
                                            </button>
                                        </DropdownMenuTrigger>
                                        <DropdownMenuContent align="start" className="w-48">
                                            <DropdownMenuItem onClick={issue.status !== "open" ? () => handleSetStatus("open") : undefined}>
                                                <Circle className="h-4 w-4 mr-2 text-green-500" />
                                                Open
                                            </DropdownMenuItem>
                                            <DropdownMenuItem
                                                onClick={issue.status !== "in_progress" ? () => handleSetStatus("in_progress") : undefined}
                                            >
                                                <CircleDot className="h-4 w-4 mr-2 text-yellow-500" />
                                                In Progress
                                            </DropdownMenuItem>
                                            <DropdownMenuItem
                                                onClick={issue.status !== "completed" ? () => handleSetStatus("completed") : undefined}
                                            >
                                                <CheckCircle2 className="h-4 w-4 mr-2 text-muted-foreground" />
                                                Completed
                                            </DropdownMenuItem>
                                            <DropdownMenuItem
                                                onClick={issue.status !== "not_planned" ? () => handleSetStatus("not_planned") : undefined}
                                            >
                                                <XCircle className="h-4 w-4 mr-2 text-muted-foreground" />
                                                Not Planned
                                            </DropdownMenuItem>
                                        </DropdownMenuContent>
                                    </DropdownMenu>
                                ) : (
                                    <div className="flex items-center gap-2 px-3 py-2 border border-border rounded-md text-sm">
                                        <StatusIcon className={`h-4 w-4 ${statusInfo.color}`} />
                                        <span>{statusInfo.label}</span>
                                    </div>
                                )}
                            </div>

                            <div>
                                <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">Assignees</h3>
                                <div className="space-y-1.5">
                                    {issue.assignees.map((assignee) => (
                                        <div
                                            key={assignee}
                                            className="group/assignee flex items-center gap-2 px-3 py-2 rounded-md border border-border text-sm"
                                        >
                                            <div className="flex h-5 w-5 items-center justify-center rounded-full bg-secondary text-[10px] font-medium shrink-0">
                                                {assignee[0].toUpperCase()}
                                            </div>
                                            <span className="flex-1">{assignee}</span>
                                            {canManage && (
                                                <button
                                                    disabled={removingAssignee === assignee}
                                                    className={`transition-opacity ${removingAssignee === assignee ? "opacity-100" : "opacity-0 group-hover/assignee:opacity-100"}`}
                                                    onClick={() => handleRemoveAssignee(assignee)}
                                                    title="Remove assignee"
                                                >
                                                    {removingAssignee === assignee ? (
                                                        <Loader2 className="h-3.5 w-3.5 animate-spin text-muted-foreground" />
                                                    ) : (
                                                        <X className="h-3.5 w-3.5 text-muted-foreground hover:text-foreground" />
                                                    )}
                                                </button>
                                            )}
                                        </div>
                                    ))}
                                    {canManage && (
                                        <DropdownMenu open={isAddingAssignee ? false : undefined}>
                                            <DropdownMenuTrigger asChild>
                                                <button
                                                    disabled={isAddingAssignee}
                                                    className="w-full flex items-center justify-center gap-2 px-3 py-2 border border-dashed border-border rounded-md text-sm text-muted-foreground hover:text-foreground hover:border-solid transition-colors"
                                                >
                                                    {isAddingAssignee ? (
                                                        <Loader2 className="h-3.5 w-3.5 animate-spin" />
                                                    ) : (
                                                        <User className="h-3.5 w-3.5" />
                                                    )}
                                                    Add assignee
                                                </button>
                                            </DropdownMenuTrigger>
                                            <DropdownMenuContent align="start" className="w-48">
                                                {(() => {
                                                    const list = [
                                                        ...(authUser &&
                                                        !issue.assignees.includes(authUser.username) &&
                                                        !collaborators.some((c) => c.userId === authUser.id)
                                                            ? [{ userId: authUser.id, username: authUser.username, accessLevel: "" }]
                                                            : []),
                                                        ...collaborators
                                                            .filter((c) => !issue.assignees.includes(c.username))
                                                            .sort((a, b) =>
                                                                a.userId === authUser?.id ? -1 : b.userId === authUser?.id ? 1 : 0
                                                            ),
                                                    ];
                                                    return list.length === 0 ? (
                                                        <DropdownMenuItem disabled>No more collaborators</DropdownMenuItem>
                                                    ) : (
                                                        list.map((c) => (
                                                            <DropdownMenuItem key={c.userId} onClick={() => handleAddAssignee(c.userId)}>
                                                                <div className="flex h-5 w-5 items-center justify-center rounded-full bg-secondary text-[10px] font-medium shrink-0 mr-2">
                                                                    {c.username[0].toUpperCase()}
                                                                </div>
                                                                {c.username}
                                                            </DropdownMenuItem>
                                                        ))
                                                    );
                                                })()}
                                            </DropdownMenuContent>
                                        </DropdownMenu>
                                    )}
                                </div>
                            </div>

                            <div>
                                <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">Labels</h3>
                                <div className="flex flex-wrap gap-2">
                                    {issue.labels.map((name) => {
                                        const color = labelMap.get(name) ?? "#888888";
                                        return (
                                            <LabelBadge
                                                key={name}
                                                name={name}
                                                color={color}
                                                removable={canManage}
                                                removing={removingLabel === name}
                                                onRemove={canManage ? () => handleRemoveLabel(name) : undefined}
                                            />
                                        );
                                    })}
                                    {canManage && (
                                        <DropdownMenu open={isAddingLabel ? false : undefined}>
                                            <DropdownMenuTrigger asChild>
                                                <button
                                                    disabled={isAddingLabel}
                                                    className="flex items-center gap-1 px-2 py-0.5 text-xs border border-dashed border-border rounded text-muted-foreground hover:text-foreground hover:border-solid transition-colors"
                                                >
                                                    {isAddingLabel ? (
                                                        <Loader2 className="h-3 w-3 animate-spin" />
                                                    ) : (
                                                        <Tag className="h-3 w-3" />
                                                    )}
                                                    Add
                                                </button>
                                            </DropdownMenuTrigger>
                                            <DropdownMenuContent align="start" className="w-48">
                                                {allLabels.filter((l) => !issue.labels.includes(l.name)).length === 0 ? (
                                                    <DropdownMenuItem disabled>No more labels</DropdownMenuItem>
                                                ) : (
                                                    allLabels
                                                        .filter((l) => !issue.labels.includes(l.name))
                                                        .map((l) => (
                                                            <DropdownMenuItem key={l.name} onClick={() => handleAddLabel(l.name)}>
                                                                <span
                                                                    className="inline-block w-3 h-3 rounded-full mr-2 shrink-0"
                                                                    style={{ backgroundColor: l.color }}
                                                                />
                                                                {l.name}
                                                            </DropdownMenuItem>
                                                        ))
                                                )}
                                            </DropdownMenuContent>
                                        </DropdownMenu>
                                    )}
                                </div>
                            </div>

                            <div>
                                <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">Priority</h3>
                                {canManage ? (
                                    <DropdownMenu open={isUpdatingPriority ? false : priorityOpen} onOpenChange={setPriorityOpen}>
                                        <DropdownMenuTrigger asChild>
                                            <button className="w-full flex items-center justify-between px-3 py-2 border border-border rounded-md hover:bg-accent/50 transition-colors text-sm">
                                                <div className="flex items-center gap-2">
                                                    <PriorityIndicator priority={issue.priority as Priority} />
                                                    <span>{priorityConfig[issue.priority as Priority].label}</span>
                                                </div>
                                                {isUpdatingPriority ? (
                                                    <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
                                                ) : (
                                                    <ChevronDown className="h-4 w-4 text-muted-foreground" />
                                                )}
                                            </button>
                                        </DropdownMenuTrigger>
                                        <DropdownMenuContent align="start" className="w-40">
                                            {PRIORITY_OPTIONS.map((opt) => (
                                                <DropdownMenuItem
                                                    key={opt.value}
                                                    onClick={issue.priority !== opt.value ? () => handleSetPriority(opt.value) : undefined}
                                                >
                                                    <PriorityIndicator priority={opt.value} />
                                                    {opt.label}
                                                </DropdownMenuItem>
                                            ))}
                                        </DropdownMenuContent>
                                    </DropdownMenu>
                                ) : (
                                    <div className="flex items-center gap-2 px-3 py-2 border border-border rounded-md text-sm">
                                        <PriorityIndicator priority={issue.priority as Priority} />
                                        <span>{priorityConfig[issue.priority as Priority].label}</span>
                                    </div>
                                )}
                            </div>

                            <div>
                                <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">Milestone</h3>
                                {canManage ? (
                                    <DropdownMenu open={isUpdatingMilestone ? false : milestoneOpen} onOpenChange={setMilestoneOpen}>
                                        <DropdownMenuTrigger asChild>
                                            <button className="w-full flex items-center justify-between px-3 py-2 border border-border rounded-md hover:bg-accent/50 transition-colors text-sm">
                                                <div className="flex items-center gap-2">
                                                    <Milestone className="h-4 w-4 text-muted-foreground shrink-0" />
                                                    {issue.milestone ? (
                                                        <span>{issue.milestone.title}</span>
                                                    ) : (
                                                        <span className="text-muted-foreground">No milestone</span>
                                                    )}
                                                </div>
                                                {isUpdatingMilestone ? (
                                                    <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
                                                ) : (
                                                    <ChevronDown className="h-4 w-4 text-muted-foreground" />
                                                )}
                                            </button>
                                        </DropdownMenuTrigger>
                                        <DropdownMenuContent align="start" className="w-52">
                                            {issue.milestone && (
                                                <DropdownMenuItem onClick={() => handleSetMilestone(null)}>
                                                    <X className="h-4 w-4 mr-2 text-muted-foreground" />
                                                    No milestone
                                                </DropdownMenuItem>
                                            )}
                                            {allMilestones.filter((m) => !m.closed || m.id === issue.milestone?.id).length === 0 &&
                                                !issue.milestone && <DropdownMenuItem disabled>No milestones available</DropdownMenuItem>}
                                            {allMilestones
                                                .filter((m) => !m.closed || m.id === issue.milestone?.id)
                                                .map((m) => (
                                                    <DropdownMenuItem
                                                        key={m.id}
                                                        onClick={m.id !== issue.milestone?.id ? () => handleSetMilestone(m.id) : undefined}
                                                    >
                                                        <Milestone className="h-4 w-4 mr-2 text-muted-foreground shrink-0" />
                                                        <span className="flex-1 truncate">{m.title}</span>
                                                        {m.id === issue.milestone?.id && <Check className="h-4 w-4 ml-2 shrink-0" />}
                                                    </DropdownMenuItem>
                                                ))}
                                        </DropdownMenuContent>
                                    </DropdownMenu>
                                ) : (
                                    issue.milestone && (
                                        <div className="flex items-center gap-2 px-3 py-2 rounded-md border border-border text-sm">
                                            <Milestone className="h-4 w-4 text-muted-foreground shrink-0" />
                                            <span className="flex-1">{issue.milestone.title}</span>
                                            {issue.milestone.closed && <span className="text-xs text-muted-foreground italic">closed</span>}
                                        </div>
                                    )
                                )}
                            </div>

                            <div>
                                <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">Timestamps</h3>
                                <div className="space-y-1.5 text-xs text-muted-foreground">
                                    <div className="flex items-center gap-2">
                                        <Calendar className="h-3.5 w-3.5" />
                                        <span>Created {formatDistanceToNow(uuidToDate(issue.id), { addSuffix: true })}</span>
                                    </div>
                                    <div className="flex items-center gap-2">
                                        <Clock className="h-3.5 w-3.5" />
                                        <span>Updated {formatDistanceToNow(new Date(issue.updatedAt), { addSuffix: true })}</span>
                                    </div>
                                </div>
                            </div>
                        </>
                    )}
                    <div className="pt-4 border-t border-border">
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
                </aside>
            </div>
        </div>
    );
}

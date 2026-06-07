"use client";

import { formatDistanceToNow } from "date-fns";
import { uuidToDate } from "@/lib/utils";
import {
    GitCommit,
    Star,
    AlertCircle,
    GitPullRequest,
    GitMerge,
    GitFork,
    Package,
    Building2,
    User,
    Code,
    MessageSquare,
    Tag,
    GitBranch,
} from "lucide-react";
import Link from "next/link";

export interface EventResponse {
    id: string;
    traceId?: string;
    actorId: string;
    actorUsername: string | null;
    class: "security" | "activity" | "system";
    type: string;
    subjectIdUser: string | null;
    subjectIdOrg: string | null;
    subjectIdRepo: string | null;
    subjectName: string | null;
    subjectNamespace: string | null;
    ipAddress: string | null;
    userAgent: string | null;
    payload: Record<string, unknown>;
}

function getBorderColor(type: string): string {
    if (type === "pr.merged") return "border-purple-500/50 hover:border-purple-500";
    if (type === "pr.opened") return "border-green-500/50 hover:border-green-500";
    if (type === "issue.opened") return "border-green-500/50 hover:border-green-500";
    if (type === "issue.closed") return "border-muted-foreground/30 hover:border-muted-foreground/50";
    if (type === "git.force_push") return "border-red-500/40 hover:border-red-500/70";
    return "border-border hover:border-muted-foreground/40";
}

function EventIcon({ type }: { type: string }) {
    const cls = "h-3.5 w-3.5 shrink-0";
    if (type === "git.push" || type === "git.force_push") return <GitCommit className={`${cls} text-muted-foreground`} />;
    if (type === "git.branch_created" || type === "git.branch_deleted") return <GitBranch className={`${cls} text-muted-foreground`} />;
    if (type === "git.tag_created" || type === "git.tag_deleted") return <Tag className={`${cls} text-muted-foreground`} />;
    if (type.startsWith("star.")) return <Star className={`${cls} text-yellow-500`} />;
    if (type === "issue.opened" || type === "issue.reopened") return <AlertCircle className={`${cls} text-green-500`} />;
    if (type === "issue.closed") return <AlertCircle className={`${cls} text-muted-foreground`} />;
    if (type.startsWith("issue.comment_")) return <MessageSquare className={`${cls} text-muted-foreground`} />;
    if (type.startsWith("issue.")) return <AlertCircle className={`${cls} text-muted-foreground`} />;
    if (type === "pr.merged") return <GitMerge className={`${cls} text-purple-500`} />;
    if (type === "pr.opened") return <GitPullRequest className={`${cls} text-green-500`} />;
    if (type === "pr.reviewed") return <GitPullRequest className={`${cls} text-muted-foreground`} />;
    if (type === "repo.forked") return <GitFork className={`${cls} text-muted-foreground`} />;
    if (type.startsWith("repo.")) return <Package className={`${cls} text-muted-foreground`} />;
    if (type.startsWith("org.")) return <Building2 className={`${cls} text-muted-foreground`} />;
    if (type.startsWith("user.")) return <User className={`${cls} text-muted-foreground`} />;
    return <Code className={`${cls} text-muted-foreground`} />;
}

function refName(ref: unknown): string {
    if (typeof ref !== "string") return "unknown";
    return ref.replace(/^refs\/(heads|tags)\//, "");
}

function issueRef(payload: Record<string, unknown>): string {
    return typeof payload.index === "number" ? `#${payload.index}` : "an issue";
}

function describeEvent(event: EventResponse): string {
    const { type, subjectName, subjectNamespace, subjectIdRepo, payload } = event;
    const repoLabel = subjectIdRepo && subjectNamespace ? `${subjectNamespace}/${subjectName}` : (subjectName ?? "a repository");

    switch (type) {
        case "git.push": {
            const branch = refName(payload.ref);
            const commits = typeof payload.commits === "number" ? payload.commits : 0;
            return `Pushed ${commits} commit${commits !== 1 ? "s" : ""} to ${branch}`;
        }
        case "git.force_push":
            return `Force-pushed to ${refName(payload.ref)}`;
        case "git.branch_created":
            return `Created branch ${refName(payload.ref)}`;
        case "git.branch_deleted":
            return `Deleted branch ${refName(payload.ref)}`;
        case "git.tag_created":
            return `Created tag ${refName(payload.ref)}`;
        case "git.tag_deleted":
            return `Deleted tag ${refName(payload.ref)}`;
        case "star.added":
            return `Starred ${repoLabel}`;
        case "star.removed":
            return `Unstarred ${repoLabel}`;
        case "issue.opened": {
            const title = typeof payload.title === "string" ? payload.title : null;
            const ref = issueRef(payload);
            return title ? `Opened issue ${ref}: ${title}` : `Opened issue ${ref}`;
        }
        case "issue.closed":
            return `Closed issue ${issueRef(payload)}`;
        case "issue.reopened":
            return `Reopened issue ${issueRef(payload)}`;
        case "issue.updated":
            return `Updated issue ${issueRef(payload)}`;
        case "issue.deleted":
            return `Deleted issue ${issueRef(payload)}`;
        case "issue.locked":
            return `Locked issue ${issueRef(payload)}`;
        case "issue.unlocked":
            return `Unlocked issue ${issueRef(payload)}`;
        case "issue.comment_added":
            return `Commented on issue ${issueRef(payload)}`;
        case "issue.comment_updated":
            return `Edited a comment on issue ${issueRef(payload)}`;
        case "issue.comment_deleted":
            return `Deleted a comment on issue ${issueRef(payload)}`;
        case "pr.opened": {
            const title = typeof payload.title === "string" ? payload.title : "a pull request";
            return `Opened pull request: ${title}`;
        }
        case "pr.merged": {
            const title = typeof payload.title === "string" ? payload.title : "a pull request";
            return `Merged pull request: ${title}`;
        }
        case "pr.reviewed":
            return "Reviewed a pull request";
        case "repo.created":
            return `Created ${repoLabel}`;
        case "repo.deleted":
            return `Deleted ${repoLabel}`;
        case "repo.updated":
            return `Updated ${repoLabel}`;
        case "repo.forked":
            return `Forked ${repoLabel}`;
        case "repo.imported": {
            const from = typeof payload.from === "string" ? ` from ${payload.from}` : "";
            return `Imported ${repoLabel}${from}`;
        }
        case "repo.mirrored": {
            const from = typeof payload.from === "string" ? ` from ${payload.from}` : "";
            return `Mirrored ${repoLabel}${from}`;
        }
        case "repo.archived":
            return `Archived ${repoLabel}`;
        case "repo.unarchived":
            return `Unarchived ${repoLabel}`;
        case "org.created":
            return `Created organization ${subjectName ?? ""}`;
        case "org.updated":
            return `Updated organization ${subjectName ?? ""}`;
        case "org.deleted":
            return `Deleted organization ${subjectName ?? ""}`;
        case "org.member_added": {
            const member = typeof payload.username === "string" ? payload.username : "a user";
            return `Added ${member} to the organization`;
        }
        case "org.member_removed": {
            const member = typeof payload.username === "string" ? payload.username : "a user";
            return `Removed ${member} from the organization`;
        }
        case "org.member_role_changed": {
            const member = typeof payload.username === "string" ? payload.username : "a user";
            return `Changed role of ${member} in the organization`;
        }
        case "user.created":
            return "Created account";
        case "user.updated":
            return "Updated profile";
        case "user.deleted":
            return "Deleted account";
        default:
            return type;
    }
}

interface ActivityEventProps {
    event: EventResponse;
}

export function ActivityEvent({ event }: ActivityEventProps) {
    const date = uuidToDate(event.id);
    const timeAgo = formatDistanceToNow(date, { addSuffix: true });
    const text = describeEvent(event);
    const borderColor = getBorderColor(event.type);

    const actorUsername = event.actorUsername ?? "anonymous";
    const initial = actorUsername[0].toUpperCase();

    const subjectHref =
        event.subjectIdRepo && event.subjectNamespace
            ? `/${event.subjectNamespace}/${event.subjectName}`
            : event.subjectName
              ? `/${event.subjectName}`
              : null;
    const subjectLabel =
        event.subjectIdRepo && event.subjectNamespace ? `${event.subjectNamespace}/${event.subjectName}` : event.subjectName;

    return (
        <div className={`group pl-4 border-l-2 transition-colors ${borderColor}`}>
            <div className="grid items-center gap-x-3 grid-cols-[1rem_1fr_8rem_1rem_11rem_9rem]">
                <div className="w-4 h-4 flex items-center justify-center shrink-0">
                    <EventIcon type={event.type} />
                </div>
                <p className="text-sm font-medium truncate min-w-0">{text}</p>
                <div className="flex items-center gap-1.5 min-w-0 text-xs text-muted-foreground">
                    <div className="h-4 w-4 flex items-center justify-center rounded-full bg-secondary border border-border text-[9px] font-medium shrink-0">
                        {initial}
                    </div>
                    {event.actorUsername ? (
                        <Link href={`/${event.actorUsername}`} className="truncate hover:text-foreground transition-colors">
                            {event.actorUsername}
                        </Link>
                    ) : (
                        <span className="truncate">anonymous</span>
                    )}
                </div>
                <span className="text-xs text-muted-foreground/40 text-center">{subjectHref ? "→" : ""}</span>
                <div className="min-w-0 text-xs text-muted-foreground">
                    {subjectHref && subjectLabel && (
                        <Link href={subjectHref} className="font-mono truncate block hover:text-foreground transition-colors">
                            {subjectLabel}
                        </Link>
                    )}
                </div>
                <span className="text-xs text-muted-foreground text-right whitespace-nowrap">{timeAgo}</span>
            </div>
        </div>
    );
}

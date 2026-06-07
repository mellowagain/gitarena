"use client";

import { useState, type ReactNode } from "react";
import { formatDistanceToNow } from "date-fns";
import { uuidToDate } from "@/lib/utils";
import { Shield, Key, LogIn, LogOut, Mail, User, Lock, ChevronDown, ChevronUp } from "lucide-react";
import Link from "next/link";
import type { EventResponse } from "@/components/activity-event";

function getEventStyle(type: string): { bg: string; color: string } {
    if (type === "auth.login") return { bg: "bg-green-500/10", color: "text-green-500" };
    if (type === "auth.login_failed") return { bg: "bg-red-500/10", color: "text-red-500" };
    if (type === "auth.logout") return { bg: "bg-secondary", color: "text-muted-foreground" };
    if (type.startsWith("session.")) return { bg: "bg-amber-500/10", color: "text-amber-500" };
    if (type.startsWith("ssh_key.") || type.startsWith("passkey.")) return { bg: "bg-blue-500/10", color: "text-blue-500" };
    if (type.startsWith("email.")) return { bg: "bg-purple-500/10", color: "text-purple-500" };
    if (type.startsWith("privilege.") || type === "user.disabled") return { bg: "bg-red-500/10", color: "text-red-500" };
    return { bg: "bg-secondary", color: "text-muted-foreground" };
}

function AuditIcon({ type, className }: { type: string; className?: string }) {
    const cls = className ?? "h-4 w-4 text-muted-foreground";
    if (type.startsWith("auth.")) return <LogIn className={cls} />;
    if (type.startsWith("session.")) return <LogOut className={cls} />;
    if (type.startsWith("ssh_key.") || type.startsWith("passkey.")) return <Key className={cls} />;
    if (type.startsWith("email.")) return <Mail className={cls} />;
    if (type.startsWith("privilege.") || type === "user.disabled") return <Shield className={cls} />;
    if (type.startsWith("repo.")) return <Lock className={cls} />;
    return <User className={cls} />;
}

function subjectLink(event: EventResponse): ReactNode {
    const { subjectName, subjectIdRepo, subjectNamespace } = event;
    if (!subjectName) return null;
    const href = subjectIdRepo && subjectNamespace ? `/${subjectNamespace}/${subjectName}` : `/${subjectName}`;
    return (
        <Link href={href} className="font-medium hover:underline">
            {subjectIdRepo && subjectNamespace ? `${subjectNamespace}/${subjectName}` : subjectName}
        </Link>
    );
}

function describeSecurity(event: EventResponse): ReactNode {
    const { type, payload } = event;
    const subject = subjectLink(event);

    switch (type) {
        case "auth.login":
            return "Signed in";
        case "auth.login_failed":
            return "Failed sign-in attempt";
        case "auth.logout":
            return "Signed out";
        case "session.revoked":
            return "Session revoked";
        case "session.revoked_all":
            return "All other sessions revoked";
        case "ssh_key.added": {
            const title = typeof payload.title === "string" ? payload.title : "a key";
            return `Added SSH key: ${title}`;
        }
        case "ssh_key.removed": {
            const title = typeof payload.title === "string" ? payload.title : "a key";
            return `Removed SSH key: ${title}`;
        }
        case "passkey.added": {
            const name = typeof payload.name === "string" ? payload.name : "a passkey";
            return `Added passkey: ${name}`;
        }
        case "passkey.removed": {
            const name = typeof payload.name === "string" ? payload.name : "a passkey";
            return `Removed passkey: ${name}`;
        }
        case "email.added": {
            const email = typeof payload.email === "string" ? payload.email : "an email";
            return `Added email: ${email}`;
        }
        case "email.removed": {
            const email = typeof payload.email === "string" ? payload.email : "an email";
            return `Removed email: ${email}`;
        }
        case "email.verified": {
            const email = typeof payload.email === "string" ? payload.email : "an email";
            return `Verified email: ${email}`;
        }
        case "email.updated": {
            const email = typeof payload.email === "string" ? payload.email : "an email";
            return `Updated email settings: ${email}`;
        }
        case "email.primary_changed": {
            const email = typeof payload.email === "string" ? payload.email : "an email";
            return `Changed primary email to: ${email}`;
        }
        case "user.disabled":
            return subject ? <>Account disabled: {subject}</> : "Account disabled";
        case "privilege.granted": {
            const level = typeof payload.new_level === "string" ? payload.new_level : "access";
            return subject ? (
                <>
                    Granted {level} access to {subject}
                </>
            ) : (
                `Granted ${level} access`
            );
        }
        case "privilege.changed": {
            const oldLevel = typeof payload.old_level === "string" ? payload.old_level : "?";
            const newLevel = typeof payload.new_level === "string" ? payload.new_level : "?";
            return subject ? (
                <>
                    Changed access from {oldLevel} to {newLevel} for {subject}
                </>
            ) : (
                `Changed access from ${oldLevel} to ${newLevel}`
            );
        }
        case "privilege.revoked":
            return subject ? <>Revoked access from {subject}</> : "Revoked access";
        case "repo.visibility_changed": {
            const visibility = typeof payload.visibility === "string" ? payload.visibility : "unknown";
            return subject ? (
                <>
                    Changed {subject} visibility to {visibility}
                </>
            ) : (
                `Changed repository visibility to ${visibility}`
            );
        }
        case "repo.transferred":
            return subject ? <>Transferred {subject}</> : "Transferred repository";
        default:
            return type;
    }
}

interface AuditLogEventProps {
    event: EventResponse;
    showActor?: boolean;
}

export function AuditLogEvent({ event, showActor = false }: AuditLogEventProps) {
    const [expanded, setExpanded] = useState(false);
    const date = uuidToDate(event.id);
    const timeAgo = formatDistanceToNow(date, { addSuffix: true });
    const description = describeSecurity(event);
    const hasPayload = Object.keys(event.payload).length > 0;
    const hasExpandable = hasPayload || !!event.userAgent || !!event.traceId;
    const { bg, color } = getEventStyle(event.type);

    return (
        <div>
            <div className="flex items-start gap-3 px-4 py-3.5">
                <div className={`flex h-8 w-8 shrink-0 items-center justify-center rounded-full ${bg}`}>
                    <AuditIcon type={event.type} className={`h-4 w-4 ${color}`} />
                </div>
                <div className="flex-1 min-w-0">
                    <div className="flex items-start justify-between gap-4">
                        <span className="text-sm font-medium">
                            {showActor && <span className="text-muted-foreground font-normal">{event.actorUsername ?? "anonymous"}: </span>}
                            {description}
                        </span>
                        <span className="text-xs text-muted-foreground shrink-0 mt-0.5">{timeAgo}</span>
                    </div>
                    <div className="flex items-center gap-2 mt-1.5 flex-wrap">
                        <code className="text-[11px] bg-secondary px-1.5 py-0.5 rounded font-mono text-muted-foreground">{event.type}</code>
                        {event.ipAddress && <span className="text-xs text-muted-foreground font-mono">{event.ipAddress}</span>}
                        {hasExpandable && (
                            <button
                                onClick={() => setExpanded((v) => !v)}
                                className="flex items-center gap-0.5 text-xs text-muted-foreground hover:text-foreground transition-colors ml-auto"
                            >
                                {expanded ? <ChevronUp className="h-3 w-3" /> : <ChevronDown className="h-3 w-3" />}
                                {expanded ? "Hide details" : "Details"}
                            </button>
                        )}
                    </div>
                </div>
            </div>
            {expanded && (
                <div className="px-4 pb-3.5 pl-[60px] space-y-2">
                    {event.traceId && <p className="text-xs text-muted-foreground font-mono">trace: {event.traceId}</p>}
                    {event.userAgent && <p className="text-xs text-muted-foreground font-mono break-all">{event.userAgent}</p>}
                    {hasPayload && (
                        <pre className="text-xs font-mono bg-secondary/50 rounded p-2 overflow-x-auto whitespace-pre-wrap break-words max-h-48 overflow-y-auto">
                            {JSON.stringify(event.payload, null, 2)}
                        </pre>
                    )}
                </div>
            )}
        </div>
    );
}

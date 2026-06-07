"use client";

import { useState } from "react";
import Link from "next/link";
import { useParams, useRouter, useSearchParams } from "next/navigation";
import {
    Building2,
    Globe,
    Lock,
    Users,
    Shield,
    Webhook,
    Key,
    AlertTriangle,
    Plus,
    Check,
    X,
    AlertCircle,
    ShieldCheck,
    Settings,
    Loader2,
    FileText,
} from "lucide-react";
import { TopBar } from "@/components/top-bar";
import useSWR, { mutate } from "swr";
import useSWRMutation from "swr/mutation";
import { jsonFetcher, putJsonVoidFetcher, deleteFetcher, patchJsonFetcher, authFetcher } from "@/lib/fetchers";
import { useAuth } from "@/hooks/use-auth";
import { toast } from "sonner";
import { AuditLogEvent } from "@/components/audit-log-event";
import type { EventResponse } from "@/components/activity-event";

// ── Types ──────────────────────────────────────────────────────────────────────

interface OrgInfo {
    id: string;
    name: string;
    description: string;
}

interface OrgMemberRaw {
    userId: string;
    role: "owner" | "admin" | "member";
}

interface UserByIdResponse {
    id: string;
    username: string;
}

type Tab = "general" | "members" | "teams" | "security" | "audit-log" | "webhooks" | "tokens" | "danger";

const navItems: { id: Tab; label: string; icon: React.ElementType }[] = [
    { id: "general", label: "General", icon: Building2 },
    { id: "members", label: "Members", icon: Users },
    { id: "teams", label: "Teams", icon: Shield },
    { id: "security", label: "Security", icon: ShieldCheck },
    { id: "audit-log", label: "Audit Log", icon: FileText },
    { id: "webhooks", label: "Webhooks", icon: Webhook },
    { id: "tokens", label: "Tokens", icon: Key },
    { id: "danger", label: "Danger Zone", icon: AlertTriangle },
];

// ── Shared primitives ──────────────────────────────────────────────────────────

function FieldLabel({ children }: { children: React.ReactNode }) {
    return <label className="block text-sm font-medium mb-1.5">{children}</label>;
}

function FieldHint({ children }: { children: React.ReactNode }) {
    return <p className="mt-1.5 text-xs text-muted-foreground">{children}</p>;
}

function Divider() {
    return <div className="border-t border-border my-6" />;
}

function SectionTitle({ children }: { children: React.ReactNode }) {
    return <h2 className="text-base font-semibold mb-4">{children}</h2>;
}

function SaveButton({ onClick, disabled }: { onClick?: () => void; disabled?: boolean }) {
    return (
        <button
            onClick={onClick}
            disabled={disabled}
            className="inline-flex items-center gap-2 px-4 h-9 bg-foreground text-background text-sm font-medium rounded-md hover:opacity-90 transition-opacity disabled:opacity-50 disabled:pointer-events-none"
        >
            <Check className="h-4 w-4" />
            Save changes
        </button>
    );
}

function WipTag() {
    return (
        <span className="inline-flex items-center px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wider border border-amber-500/40 rounded bg-amber-500/10 text-amber-600 dark:text-amber-400">
            WIP
        </span>
    );
}

// ── Tabs ───────────────────────────────────────────────────────────────────────

function GeneralTab({ org }: { org: OrgInfo }) {
    const [description, setDescription] = useState(org.description);

    const { trigger: saveDescription, isMutating: isSaving } = useSWRMutation(
        `/api/orgs/${org.name}`,
        (url: string, { arg }: { arg: { description: string } }) => patchJsonFetcher<{ description: string }, void>(url, { arg }),
        {
            onSuccess: () => {
                mutate(`/api/orgs/${org.name}`);
                toast.success("Description saved");
            },
            onError: (err: Error) => toast.error(err.message),
        }
    );

    return (
        <div className="space-y-6">
            <SectionTitle>General</SectionTitle>

            {/* Avatar — WIP */}
            <div>
                <div className="flex items-center gap-2 mb-1.5">
                    <FieldLabel>Organization avatar</FieldLabel>
                    <WipTag />
                </div>
                <div className="flex items-center gap-4">
                    <div className="h-16 w-16 rounded-xl bg-secondary border border-border flex items-center justify-center text-2xl font-semibold">
                        {org.name[0].toUpperCase()}
                    </div>
                    <div className="space-y-1.5">
                        <button
                            disabled
                            className="inline-flex items-center gap-2 px-3 h-8 text-sm border border-border rounded-md opacity-50 cursor-not-allowed"
                        >
                            Upload image
                        </button>
                        <p className="text-xs text-muted-foreground">PNG, JPG or GIF, max 1 MB</p>
                    </div>
                </div>
            </div>

            <Divider />

            {/* Display name — WIP */}
            <div>
                <div className="flex items-center gap-2 mb-1.5">
                    <FieldLabel>Display name</FieldLabel>
                    <WipTag />
                </div>
                <input
                    type="text"
                    disabled
                    defaultValue={org.name}
                    className="w-full h-9 px-3 bg-card border border-border rounded-md text-sm opacity-50 cursor-not-allowed"
                />
                <FieldHint>Display name editing is not yet available.</FieldHint>
            </div>

            {/* Description */}
            <div>
                <FieldLabel>Description</FieldLabel>
                <textarea
                    value={description}
                    onChange={(e) => setDescription(e.target.value)}
                    rows={3}
                    maxLength={256}
                    className="w-full px-3 py-2 bg-card border border-border rounded-md text-sm resize-none focus:outline-none focus:ring-1 focus:ring-ring"
                />
                <FieldHint>Max 256 characters.</FieldHint>
            </div>

            <Divider />

            {/* Visibility — WIP */}
            <div>
                <div className="flex items-center gap-2 mb-2">
                    <FieldLabel>Organization visibility</FieldLabel>
                    <WipTag />
                </div>
                <div className="space-y-2 opacity-50 pointer-events-none">
                    {(["public", "private"] as const).map((v) => (
                        <label
                            key={v}
                            className={`flex items-start gap-3 p-3 border rounded-md ${v === "public" ? "border-foreground bg-accent/30" : "border-border"}`}
                        >
                            <div
                                className={`mt-0.5 h-4 w-4 rounded-full border-2 flex items-center justify-center shrink-0 ${v === "public" ? "border-foreground" : "border-muted-foreground"}`}
                            >
                                {v === "public" && <div className="h-2 w-2 rounded-full bg-foreground" />}
                            </div>
                            <div>
                                <div className="flex items-center gap-2 text-sm font-medium">
                                    {v === "public" ? <Globe className="h-3.5 w-3.5" /> : <Lock className="h-3.5 w-3.5" />}
                                    <span className="capitalize">{v}</span>
                                </div>
                                <p className="text-xs text-muted-foreground mt-0.5">
                                    {v === "public"
                                        ? "Anyone can view this organization and its public repositories."
                                        : "Only members can view this organization and its repositories."}
                                </p>
                            </div>
                        </label>
                    ))}
                </div>
            </div>

            <Divider />

            {/* Repository defaults — WIP */}
            <div>
                <div className="flex items-center gap-2 mb-4">
                    <SectionTitle>Repository defaults</SectionTitle>
                    <WipTag />
                </div>
                <div className="space-y-4 opacity-50 pointer-events-none">
                    <div>
                        <FieldLabel>Default branch name</FieldLabel>
                        <input
                            type="text"
                            disabled
                            defaultValue="main"
                            className="w-full h-9 px-3 bg-card border border-border rounded-md text-sm font-mono"
                        />
                        <FieldHint>Applied to all newly created repositories in this organization.</FieldHint>
                    </div>
                    <div>
                        <FieldLabel>Default repository visibility</FieldLabel>
                        <select disabled className="w-full h-9 px-3 bg-card border border-border rounded-md text-sm">
                            <option value="public">Public</option>
                            <option value="private">Private</option>
                        </select>
                    </div>
                </div>
            </div>

            <Divider />

            {/* Member permissions — WIP */}
            <div>
                <div className="flex items-center gap-2 mb-4">
                    <SectionTitle>Member permissions</SectionTitle>
                    <WipTag />
                </div>
                <div className="space-y-3 opacity-50 pointer-events-none">
                    {[
                        {
                            key: "allowForking",
                            label: "Allow forking of private repositories",
                            hint: "Members with access can fork private repositories within the org.",
                        },
                        {
                            key: "allowMembersCreatePublic",
                            label: "Allow members to create public repos",
                            hint: "Members can create public repositories under this organization.",
                        },
                        {
                            key: "allowMembersCreatePrivate",
                            label: "Allow members to create private repos",
                            hint: "Members can create private repositories under this organization.",
                        },
                    ].map((item) => (
                        <div key={item.key} className="flex items-start gap-3 p-3 border border-border rounded-md">
                            <input type="checkbox" disabled className="mt-0.5 rounded border-border" />
                            <div>
                                <p className="text-sm font-medium">{item.label}</p>
                                <p className="text-xs text-muted-foreground mt-0.5">{item.hint}</p>
                            </div>
                        </div>
                    ))}
                </div>
            </div>

            <SaveButton onClick={() => saveDescription({ description })} disabled={isSaving} />
        </div>
    );
}

function MemberRow({
    member,
    onRemove,
    onRoleChange,
}: {
    member: OrgMemberRaw;
    onRemove: (username: string) => void;
    onRoleChange: (username: string, role: string) => void;
}) {
    const { data: user } = useSWR<UserByIdResponse>(`/api/users/by-id/${member.userId}`, jsonFetcher);
    const username = user?.username ?? `…`;

    const roleBadge: Record<string, string> = {
        owner: "text-amber-500 bg-amber-500/10 border-amber-500/30",
        admin: "text-blue-500 bg-blue-500/10 border-blue-500/30",
        member: "text-muted-foreground bg-secondary border-border",
    };

    return (
        <div className="flex items-center gap-3 px-4 py-3 hover:bg-accent/20 transition-colors border-t border-border first:border-t-0">
            <div className="h-7 w-7 flex items-center justify-center rounded-full bg-secondary border border-border text-xs font-medium shrink-0">
                {username[0]?.toUpperCase() ?? "?"}
            </div>
            <div className="flex-1 min-w-0">
                {user ? (
                    <Link href={`/${username}`} className="text-sm font-medium hover:underline">
                        @{username}
                    </Link>
                ) : (
                    <span className="text-sm font-medium text-muted-foreground">Loading…</span>
                )}
            </div>
            <span
                className={`inline-flex items-center px-2 py-0.5 text-xs font-medium border rounded capitalize ${roleBadge[member.role]}`}
            >
                {member.role}
            </span>
            <select
                value={member.role}
                onChange={(e) => onRoleChange(username, e.target.value)}
                disabled={member.role === "owner" || !user}
                className="h-7 px-2 bg-card border border-border rounded text-xs focus:outline-none focus:ring-1 focus:ring-ring disabled:opacity-40 disabled:cursor-not-allowed"
            >
                <option value="member">Member</option>
                <option value="admin">Admin</option>
                <option value="owner">Owner</option>
            </select>
            <button
                onClick={() => onRemove(username)}
                disabled={member.role === "owner" || !user}
                title={member.role === "owner" ? "Cannot remove the last owner" : "Remove member"}
                className="h-7 w-7 flex items-center justify-center text-muted-foreground hover:text-destructive hover:bg-destructive/10 rounded transition-colors disabled:opacity-30 disabled:pointer-events-none"
            >
                <X className="h-3.5 w-3.5" />
            </button>
        </div>
    );
}

function MembersTab({ orgName }: { orgName: string }) {
    const membersKey = `/api/orgs/${orgName}/members`;
    const { data: rawMembers, isLoading } = useSWR<OrgMemberRaw[]>(membersKey, jsonFetcher);
    const [inviteInput, setInviteInput] = useState("");
    const [inviteRole, setInviteRole] = useState<"member" | "admin" | "owner">("member");
    const [isInviting, setIsInviting] = useState(false);

    const { trigger: addMember } = useSWRMutation(
        membersKey,
        (_url: string, { arg }: { arg: { username: string; role: string } }) =>
            putJsonVoidFetcher<{ username: string; role: string }>(membersKey, { arg }),
        {
            onSuccess: () => {
                setInviteInput("");
                mutate(membersKey);
                toast.success("Member added successfully");
            },
            onError: (err: Error) => toast.error(err.message),
        }
    );

    const { trigger: removeMember } = useSWRMutation(
        membersKey,
        (_url: string, { arg }: { arg: string }) => deleteFetcher(`/api/orgs/${orgName}/members/${arg}`),
        {
            onSuccess: () => {
                mutate(membersKey);
                toast.success("Member removed");
            },
            onError: (err: Error) => toast.error(err.message),
        }
    );

    const { trigger: changeRole } = useSWRMutation(
        membersKey,
        (_url: string, { arg }: { arg: { username: string; role: string } }) =>
            putJsonVoidFetcher<{ username: string; role: string }>(membersKey, { arg }),
        {
            onSuccess: () => {
                mutate(membersKey);
                toast.success("Role updated");
            },
            onError: (err: Error) => toast.error(err.message),
        }
    );

    async function handleInvite() {
        if (!inviteInput.trim()) {
            return;
        }
        setIsInviting(true);
        try {
            await addMember({ username: inviteInput.trim(), role: inviteRole });
        } finally {
            setIsInviting(false);
        }
    }

    return (
        <div className="space-y-6">
            <SectionTitle>Members</SectionTitle>

            {/* Invite */}
            <div className="p-4 border border-border rounded-md space-y-3">
                <p className="text-sm font-medium">Add a member</p>
                <div className="flex gap-2">
                    <input
                        type="text"
                        value={inviteInput}
                        onChange={(e) => setInviteInput(e.target.value)}
                        placeholder="Username"
                        className="flex-1 h-9 px-3 bg-card border border-border rounded-md text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
                    />
                    <select
                        value={inviteRole}
                        onChange={(e) => setInviteRole(e.target.value as typeof inviteRole)}
                        className="h-9 px-2 bg-card border border-border rounded-md text-sm focus:outline-none focus:ring-1 focus:ring-ring"
                    >
                        <option value="member">Member</option>
                        <option value="admin">Admin</option>
                        <option value="owner">Owner</option>
                    </select>
                    <button
                        className="inline-flex items-center gap-2 px-3 h-9 bg-foreground text-background text-sm font-medium rounded-md hover:opacity-90 transition-opacity whitespace-nowrap disabled:opacity-50"
                        onClick={handleInvite}
                        disabled={isInviting || !inviteInput.trim()}
                    >
                        {isInviting ? <Loader2 className="h-4 w-4 animate-spin" /> : <Plus className="h-4 w-4" />}
                        Add member
                    </button>
                </div>
            </div>

            {/* Member list */}
            {isLoading ? (
                <div className="space-y-2">
                    {[0, 1, 2].map((i) => (
                        <div key={i} className="h-12 bg-secondary/50 rounded-md animate-pulse" />
                    ))}
                </div>
            ) : (
                <div className="border border-border rounded-md overflow-hidden">
                    {(rawMembers ?? []).map((m) => (
                        <MemberRow
                            key={m.userId}
                            member={m}
                            onRemove={(username) => removeMember(username)}
                            onRoleChange={(username, role) => changeRole({ username, role })}
                        />
                    ))}
                    {(rawMembers ?? []).length === 0 && (
                        <div className="px-4 py-8 text-center text-sm text-muted-foreground">No members found.</div>
                    )}
                </div>
            )}
            <p className="text-xs text-muted-foreground">
                {(rawMembers ?? []).length} member{(rawMembers ?? []).length !== 1 ? "s" : ""}.
            </p>
        </div>
    );
}

function TeamsTab() {
    return (
        <div className="space-y-6">
            <div className="flex items-center gap-2 mb-4">
                <SectionTitle>Teams</SectionTitle>
                <WipTag />
            </div>
            <div className="flex items-center gap-2 p-4 border border-amber-500/30 rounded-md bg-amber-500/5">
                <AlertCircle className="h-4 w-4 text-amber-500 shrink-0" />
                <p className="text-sm text-muted-foreground">Teams are not yet available. This feature is coming soon.</p>
            </div>
        </div>
    );
}

function SecurityTab() {
    return (
        <div className="space-y-6">
            <div className="flex items-center gap-2 mb-4">
                <SectionTitle>Security</SectionTitle>
                <WipTag />
            </div>

            {/* 2FA requirement — WIP */}
            <div className="p-4 border border-border rounded-md space-y-3 opacity-50 pointer-events-none">
                <div className="flex items-start justify-between gap-4">
                    <div>
                        <p className="text-sm font-medium">Require two-factor authentication</p>
                        <p className="text-xs text-muted-foreground mt-1">
                            All members must have 2FA enabled to join or remain in this organization.
                        </p>
                    </div>
                    <div className="relative inline-flex h-5 w-9 items-center rounded-full bg-border shrink-0">
                        <span className="inline-block h-3.5 w-3.5 rounded-full bg-background shadow translate-x-1" />
                    </div>
                </div>
            </div>

            {/* IP allowlist — WIP */}
            <div className="opacity-50 pointer-events-none">
                <p className="text-sm font-medium mb-1.5">IP allowlist</p>
                <p className="text-xs text-muted-foreground mb-3">Restrict access to specific IP addresses or CIDR ranges.</p>
                <textarea
                    rows={4}
                    disabled
                    placeholder={"192.168.1.0/24\n10.0.0.1"}
                    className="w-full px-3 py-2 bg-card border border-border rounded-md text-sm font-mono placeholder:text-muted-foreground resize-none"
                />
            </div>
        </div>
    );
}

function AuditLogTab({ orgName }: { orgName: string }) {
    const { data: events, isLoading } = useSWR<EventResponse[] | null>(`/api/orgs/${orgName}/audit-log`, authFetcher);

    return (
        <div>
            <SectionTitle>Audit Log</SectionTitle>
            <p className="text-sm text-muted-foreground mb-6">
                Security events for this organization, including membership changes and permission updates.
            </p>

            {isLoading && (
                <div className="space-y-3">
                    {[0, 1, 2, 3, 4].map((i) => (
                        <div key={i} className="h-12 rounded-md bg-secondary/50 animate-pulse" />
                    ))}
                </div>
            )}
            {!isLoading && (!events || events.length === 0) && (
                <div className="border border-border rounded-md px-4 py-8 text-center text-sm text-muted-foreground">
                    No security events recorded yet.
                </div>
            )}
            {!isLoading && events && events.length > 0 && (
                <div className="border border-border rounded-md divide-y divide-border">
                    {events.map((event) => (
                        <AuditLogEvent key={event.id} event={event} showActor />
                    ))}
                </div>
            )}
        </div>
    );
}

function WebhooksTab() {
    return (
        <div className="space-y-6">
            <div className="flex items-center gap-2 mb-4">
                <SectionTitle>Webhooks</SectionTitle>
                <WipTag />
            </div>
            <div className="flex items-center gap-2 p-4 border border-amber-500/30 rounded-md bg-amber-500/5">
                <AlertCircle className="h-4 w-4 text-amber-500 shrink-0" />
                <p className="text-sm text-muted-foreground">Organization webhooks are not yet available.</p>
            </div>
        </div>
    );
}

function TokensTab() {
    return (
        <div className="space-y-6">
            <div className="flex items-center gap-2 mb-4">
                <SectionTitle>Organization tokens</SectionTitle>
                <WipTag />
            </div>
            <div className="flex items-center gap-2 p-4 border border-amber-500/30 rounded-md bg-amber-500/5">
                <AlertCircle className="h-4 w-4 text-amber-500 shrink-0" />
                <p className="text-sm text-muted-foreground">Organization tokens are not yet available.</p>
            </div>
        </div>
    );
}

function DangerTab({ orgName }: { orgName: string }) {
    const router = useRouter();
    const [deleteInput, setDeleteInput] = useState("");
    const [showDelete, setShowDelete] = useState(false);

    const { trigger: deleteOrg, isMutating: isDeleting } = useSWRMutation(`/api/orgs/${orgName}`, (url: string) => deleteFetcher(url), {
        onSuccess: () => {
            toast.success("Organization deleted");
            router.push("/");
        },
        onError: (err: Error) => toast.error(err.message),
    });

    function handleDelete() {
        if (deleteInput !== orgName) {
            return;
        }
        deleteOrg();
    }

    return (
        <div className="space-y-6">
            <SectionTitle>Danger Zone</SectionTitle>

            {/* Rename — WIP */}
            <div className="p-4 border border-destructive/30 rounded-md space-y-3 opacity-60 pointer-events-none">
                <div>
                    <div className="flex items-center gap-2">
                        <p className="text-sm font-semibold text-destructive">Rename organization</p>
                        <WipTag />
                    </div>
                    <p className="text-xs text-muted-foreground mt-1">
                        Renaming breaks existing clone URLs and references to <code className="font-mono">@{orgName}</code>.
                    </p>
                </div>
                <div className="flex gap-2">
                    <input
                        type="text"
                        disabled
                        placeholder={`New name for ${orgName}`}
                        className="flex-1 h-9 px-3 bg-card border border-border rounded-md text-sm placeholder:text-muted-foreground"
                    />
                    <button
                        disabled
                        className="px-4 h-9 text-sm font-medium text-destructive border border-destructive/50 rounded-md opacity-40"
                    >
                        Rename
                    </button>
                </div>
            </div>

            {/* Transfer — WIP */}
            <div className="p-4 border border-destructive/30 rounded-md space-y-3 opacity-60 pointer-events-none">
                <div>
                    <div className="flex items-center gap-2">
                        <p className="text-sm font-semibold text-destructive">Transfer ownership</p>
                        <WipTag />
                    </div>
                    <p className="text-xs text-muted-foreground mt-1">Transfer this organization to another user.</p>
                </div>
                <button
                    disabled
                    className="px-4 h-9 text-sm font-medium text-destructive border border-destructive/50 rounded-md opacity-40"
                >
                    Transfer ownership
                </button>
            </div>

            {/* Delete */}
            <div className="p-4 border border-destructive/50 bg-destructive/5 rounded-md space-y-3">
                <div>
                    <p className="text-sm font-semibold text-destructive">Delete this organization</p>
                    <p className="text-xs text-muted-foreground mt-1">
                        This will permanently delete <strong>{orgName}</strong> and all of its repositories, issues, merge requests, and
                        member data. This action cannot be undone.
                    </p>
                </div>
                {!showDelete ? (
                    <button
                        onClick={() => setShowDelete(true)}
                        className="px-4 h-9 text-sm font-medium text-destructive border border-destructive/50 rounded-md hover:bg-destructive/10 transition-colors"
                    >
                        Delete organization
                    </button>
                ) : (
                    <div className="space-y-3">
                        <p className="text-xs font-medium">
                            Type <code className="font-mono">{orgName}</code> to confirm deletion:
                        </p>
                        <input
                            type="text"
                            value={deleteInput}
                            onChange={(e) => setDeleteInput(e.target.value)}
                            placeholder={orgName}
                            className="w-full h-9 px-3 bg-card border border-destructive/50 rounded-md text-sm font-mono placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-destructive"
                        />
                        <div className="flex gap-2">
                            <button
                                disabled={deleteInput !== orgName || isDeleting}
                                onClick={handleDelete}
                                className="px-4 h-9 text-sm font-medium text-white bg-destructive rounded-md hover:opacity-90 transition-opacity disabled:opacity-40 disabled:pointer-events-none inline-flex items-center gap-2"
                            >
                                {isDeleting && <Loader2 className="h-3.5 w-3.5 animate-spin" />}I understand, delete this organization
                            </button>
                            <button
                                onClick={() => {
                                    setShowDelete(false);
                                    setDeleteInput("");
                                }}
                                className="px-3 h-9 text-sm border border-border rounded-md hover:bg-accent/50 transition-colors"
                            >
                                Cancel
                            </button>
                        </div>
                    </div>
                )}
            </div>
        </div>
    );
}

// ── Page ───────────────────────────────────────────────────────────────────────

export default function OrgSettingsPage() {
    const params = useParams();
    const orgName = params.name as string;
    const { user: authUser } = useAuth();
    const searchParams = useSearchParams();

    const { data: org, isLoading, error } = useSWR<OrgInfo>(`/api/orgs/${orgName}`, jsonFetcher);
    const { data: members } = useSWR<OrgMemberRaw[]>(`/api/orgs/${orgName}/members`, jsonFetcher);

    const tabFromQuery = searchParams.get("tab") as Tab | null;
    const validTabFromQuery = tabFromQuery && navItems.some((n) => n.id === tabFromQuery) ? tabFromQuery : null;
    const [activeTab, setActiveTab] = useState<Tab>(validTabFromQuery ?? "general");

    // Determine if the current user is an admin/owner
    const myRole = authUser && members ? members.find((m) => m.userId === authUser.id)?.role : undefined;
    const isAdmin = myRole === "owner" || myRole === "admin";

    if (isLoading) {
        return (
            <div className="flex flex-col h-screen overflow-hidden bg-background text-foreground font-sans items-center justify-center">
                <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            </div>
        );
    }

    if (error || !org) {
        return (
            <div className="flex flex-col h-screen overflow-hidden bg-background text-foreground font-sans items-center justify-center">
                <p className="text-sm text-muted-foreground">Organization not found.</p>
            </div>
        );
    }

    const tabContent: Record<Tab, React.ReactNode> = {
        general: <GeneralTab org={org} />,
        members: <MembersTab orgName={orgName} />,
        teams: <TeamsTab />,
        security: <SecurityTab />,
        "audit-log": <AuditLogTab orgName={orgName} />,
        webhooks: <WebhooksTab />,
        tokens: <TokensTab />,
        danger: <DangerTab orgName={orgName} />,
    };

    return (
        <div className="flex flex-col h-screen overflow-hidden bg-background text-foreground font-sans">
            <TopBar
                breadcrumb={[{ label: "orgs" }, { label: orgName, href: `/${orgName}` }, { label: "Settings" }]}
                navLinks={[
                    { label: "Overview", href: `/${orgName}`, icon: <Building2 className="h-[18px] w-[18px]" /> },
                    { label: "Settings", href: `/${orgName}/settings`, icon: <Settings className="h-[18px] w-[18px]" />, active: true },
                ]}
                hasNotifications
            />

            <div className="flex-1 flex overflow-hidden">
                {/* Left sidebar nav */}
                <aside className="w-60 border-r border-border shrink-0 overflow-y-auto">
                    <div className="p-3">
                        <h3 className="px-3 mb-2 mt-2 text-xs font-medium uppercase tracking-widest text-muted-foreground">
                            Organization settings
                        </h3>
                        <nav className="space-y-0.5">
                            {navItems.map((item) => (
                                <button
                                    key={item.id}
                                    onClick={() => setActiveTab(item.id)}
                                    className={`w-full flex items-center gap-2.5 px-3 py-2 text-sm rounded-md transition-colors text-left ${
                                        activeTab === item.id
                                            ? "bg-accent text-foreground"
                                            : "text-muted-foreground hover:text-foreground hover:bg-accent/50"
                                    } ${item.id === "danger" ? "text-destructive hover:text-destructive" : ""}`}
                                >
                                    <item.icon className="h-4 w-4 shrink-0" />
                                    {item.label}
                                </button>
                            ))}
                        </nav>
                    </div>
                </aside>

                {/* Main content */}
                <main className="flex-1 overflow-y-auto">
                    <div className="max-w-2xl mx-auto px-8 py-8">
                        {!isAdmin && activeTab !== "general" && (
                            <div className="flex items-center gap-2 p-4 mb-6 border border-amber-500/30 rounded-md bg-amber-500/5">
                                <AlertCircle className="h-4 w-4 text-amber-500 shrink-0" />
                                <p className="text-sm text-muted-foreground">
                                    You need admin or owner permissions to modify these settings.
                                </p>
                            </div>
                        )}
                        {tabContent[activeTab]}
                    </div>
                </main>
            </div>
        </div>
    );
}

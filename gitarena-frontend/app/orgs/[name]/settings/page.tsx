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
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { UserAvatar } from "@/components/ui/user-avatar";
import { TokenManager } from "@/components/token-manager";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { Switch } from "@/components/ui/switch";
import { Separator } from "@/components/ui/separator";
import { Field, FieldDescription, FieldLabel, FieldLegend, FieldSet, FieldTitle } from "@/components/ui/field";
import { Button, buttonVariants } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";
import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Skeleton } from "@/components/ui/skeleton";

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

function Divider() {
    return <Separator className="my-6" />;
}

function SectionTitle({ children }: { children: React.ReactNode }) {
    return <h2 className="text-base font-semibold mb-4">{children}</h2>;
}

function SaveButton({ onClick, disabled }: { onClick?: () => void; disabled?: boolean }) {
    return (
        <Button onClick={onClick} disabled={disabled}>
            <Check className="h-4 w-4" />
            Save changes
        </Button>
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
                <FieldTitle className="mb-1.5">
                    Organization avatar
                    <WipTag />
                </FieldTitle>
                <div className="flex items-center gap-4">
                    <div className="h-16 w-16 rounded-xl bg-secondary border border-border flex items-center justify-center text-2xl font-semibold">
                        {org.name[0].toUpperCase()}
                    </div>
                    <div className="space-y-1.5">
                        <Button variant="outline" size="sm" disabled className="opacity-50 cursor-not-allowed">
                            Upload image
                        </Button>
                        <p className="text-xs text-muted-foreground">PNG, JPG or GIF, max 1 MB</p>
                    </div>
                </div>
            </div>

            <Divider />

            {/* Display name — WIP */}
            <Field className="gap-1.5">
                <FieldLabel htmlFor="org-display-name">
                    Display name
                    <WipTag />
                </FieldLabel>
                <Input id="org-display-name" disabled defaultValue={org.name} />
                <FieldDescription className="text-xs">Display name editing is not yet available.</FieldDescription>
            </Field>

            {/* Description */}
            <Field className="gap-1.5">
                <FieldLabel htmlFor="org-description">Description</FieldLabel>
                <Textarea
                    id="org-description"
                    value={description}
                    onChange={(e) => setDescription(e.target.value)}
                    rows={3}
                    maxLength={256}
                />
                <FieldDescription className="text-xs">Max 256 characters.</FieldDescription>
            </Field>

            <Divider />

            {/* Visibility — WIP */}
            <FieldSet className="gap-0">
                <FieldLegend variant="label" className="mb-2 flex items-center gap-2">
                    Organization visibility
                    <WipTag />
                </FieldLegend>
                <RadioGroup value="public" disabled className="gap-2 opacity-50 pointer-events-none">
                    {(["public", "private"] as const).map((v) => (
                        <label
                            key={v}
                            className={`flex items-start gap-3 p-3 border rounded-md ${v === "public" ? "border-foreground bg-accent/30" : "border-border"}`}
                        >
                            <RadioGroupItem value={v} className="mt-0.5" />
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
                </RadioGroup>
            </FieldSet>

            <Divider />

            {/* Repository defaults — WIP */}
            <div>
                <div className="flex items-center gap-2 mb-4">
                    <SectionTitle>Repository defaults</SectionTitle>
                    <WipTag />
                </div>
                <div className="space-y-4 opacity-50 pointer-events-none">
                    <Field className="gap-1.5">
                        <FieldLabel htmlFor="org-default-branch">Default branch name</FieldLabel>
                        <Input id="org-default-branch" disabled defaultValue="main" className="font-mono" />
                        <FieldDescription className="text-xs">
                            Applied to all newly created repositories in this organization.
                        </FieldDescription>
                    </Field>
                    <Field className="gap-1.5">
                        <FieldLabel htmlFor="org-default-visibility">Default repository visibility</FieldLabel>
                        <Select disabled defaultValue="public">
                            <SelectTrigger id="org-default-visibility" className="w-full bg-card">
                                <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                                <SelectItem value="public">Public</SelectItem>
                                <SelectItem value="private">Private</SelectItem>
                            </SelectContent>
                        </Select>
                    </Field>
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
                            <Checkbox disabled className="mt-0.5" />
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
            <UserAvatar userId={member.userId} username={username} size="md" className="size-7" />
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
            <Select value={member.role} onValueChange={(role) => onRoleChange(username, role)} disabled={member.role === "owner" || !user}>
                <SelectTrigger size="sm" className="bg-card text-xs">
                    <SelectValue />
                </SelectTrigger>
                <SelectContent>
                    <SelectItem value="member">Member</SelectItem>
                    <SelectItem value="admin">Admin</SelectItem>
                    <SelectItem value="owner">Owner</SelectItem>
                </SelectContent>
            </Select>
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
                    <Input value={inviteInput} onChange={(e) => setInviteInput(e.target.value)} placeholder="Username" className="flex-1" />
                    <Select value={inviteRole} onValueChange={(role) => setInviteRole(role as typeof inviteRole)}>
                        <SelectTrigger className="bg-card">
                            <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                            <SelectItem value="member">Member</SelectItem>
                            <SelectItem value="admin">Admin</SelectItem>
                            <SelectItem value="owner">Owner</SelectItem>
                        </SelectContent>
                    </Select>
                    <Button className="px-3" onClick={handleInvite} disabled={isInviting || !inviteInput.trim()}>
                        {isInviting ? <Spinner /> : <Plus className="h-4 w-4" />}
                        Add member
                    </Button>
                </div>
            </div>

            {/* Member list */}
            {isLoading ? (
                <div className="space-y-2">
                    {[0, 1, 2].map((i) => (
                        <Skeleton key={i} className="h-12 rounded-md" />
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
            <Alert variant="warning">
                <AlertCircle />
                <AlertDescription>Teams are not yet available. This feature is coming soon.</AlertDescription>
            </Alert>
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
                    <Switch disabled />
                </div>
            </div>

            {/* IP allowlist — WIP */}
            <div className="opacity-50 pointer-events-none">
                <p className="text-sm font-medium mb-1.5">IP allowlist</p>
                <p className="text-xs text-muted-foreground mb-3">Restrict access to specific IP addresses or CIDR ranges.</p>
                <Textarea rows={4} disabled placeholder={"192.168.1.0/24\n10.0.0.1"} className="font-mono" />
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
                        <Skeleton key={i} className="h-12 rounded-md" />
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
            <Alert variant="warning">
                <AlertCircle />
                <AlertDescription>Organization webhooks are not yet available.</AlertDescription>
            </Alert>
        </div>
    );
}

function TokensTab({ org }: { org: OrgInfo }) {
    return (
        <TokenManager
            owner={{ kind: "org", id: org.id, name: org.name }}
            title="Organization tokens"
            description="Tokens that act on behalf of the organization, scoped to all of its resources or a selection of repositories."
        />
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
                    <Input disabled placeholder={`New name for ${orgName}`} className="flex-1" />
                    <Button variant="outline" disabled className="text-destructive border-destructive/50 hover:text-destructive">
                        Rename
                    </Button>
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
                <Button variant="outline" disabled className="text-destructive border-destructive/50 hover:text-destructive">
                    Transfer ownership
                </Button>
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
                <Button
                    variant="outline"
                    onClick={() => setShowDelete(true)}
                    className="text-destructive border-destructive/50 hover:bg-destructive/10 hover:text-destructive"
                >
                    Delete organization
                </Button>
            </div>
            <AlertDialog
                open={showDelete}
                onOpenChange={(open) => {
                    if (isDeleting) {
                        return;
                    }
                    setShowDelete(open);
                    if (!open) {
                        setDeleteInput("");
                    }
                }}
            >
                <AlertDialogContent>
                    <AlertDialogHeader>
                        <AlertDialogTitle>Delete this organization?</AlertDialogTitle>
                        <AlertDialogDescription>
                            This will permanently delete <strong>{orgName}</strong> and all of its repositories, issues, merge requests, and
                            member data. This action cannot be undone.
                        </AlertDialogDescription>
                    </AlertDialogHeader>
                    <Field className="gap-1.5">
                        <FieldDescription id="org-delete-confirm-label" className="text-xs font-medium text-foreground">
                            Type <code className="font-mono">{orgName}</code> to confirm deletion:
                        </FieldDescription>
                        <Input
                            aria-labelledby="org-delete-confirm-label"
                            value={deleteInput}
                            onChange={(e) => setDeleteInput(e.target.value)}
                            placeholder={orgName}
                            className="border-destructive/50 focus-visible:ring-destructive/40 font-mono"
                        />
                    </Field>
                    <AlertDialogFooter>
                        <AlertDialogCancel disabled={isDeleting}>Cancel</AlertDialogCancel>
                        <AlertDialogAction
                            onClick={(e) => {
                                e.preventDefault();
                                handleDelete();
                            }}
                            disabled={deleteInput !== orgName || isDeleting}
                            className={buttonVariants({ variant: "destructive" })}
                        >
                            {isDeleting && <Spinner />}I understand, delete this organization
                        </AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>
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
                <Spinner className="size-6 text-muted-foreground" />
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
        tokens: <TokensTab org={org} />,
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
                            <Alert variant="warning" className="mb-6">
                                <AlertCircle />
                                <AlertDescription>You need admin or owner permissions to modify these settings.</AlertDescription>
                            </Alert>
                        )}
                        {tabContent[activeTab]}
                    </div>
                </main>
            </div>
        </div>
    );
}

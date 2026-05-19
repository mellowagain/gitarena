"use client";

import { useState } from "react";
import useSWR, { mutate } from "swr";
import useSWRMutation from "swr/mutation";
import {
    Settings,
    Users,
    ShieldCheck,
    Webhook,
    KeyRound,
    Code2,
    AlertTriangle,
    Plus,
    Trash2,
    X,
    Globe,
    Lock,
    GitBranch,
    GitMerge,
    Tag,
    Check,
    Copy,
    AlertCircle,
    ChevronDown,
    Archive,
} from "lucide-react";
import { Skeleton } from "@/components/ui/skeleton";
import { jsonFetcher, putJsonFetcher, deleteFetcher } from "@/lib/fetchers";
import { TopBar } from "@/components/top-bar";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { useParams } from "next/navigation";

// ── Types ──────────────────────────────────────────────────────────────────────

type Tab = "general" | "collaboration" | "branches" | "integrations" | "danger";

// ── Mock data ──────────────────────────────────────────────────────────────────

const repoData = {
    org: "mellowagain",
    name: "test",
    description: "A lightweight git hosting solution built for speed and simplicity.",
    visibility: "public" as "public" | "internal" | "private",
    defaultBranch: "main",
    branches: ["main", "develop", "feature/auth"],
    topics: ["git", "self-hosted", "rust"],
    features: {
        issues: true,
        wiki: false,
        packages: false,
        releases: true,
        forking: true,
    },
    mergeStrategies: {
        mergeCommit: true,
        squash: true,
        rebase: false,
    },
    autoDeleteBranch: true,
    requireSignedCommits: false,
};

const mockBranchRules = [
    {
        id: "1",
        pattern: "main",
        requirePR: true,
        requiredReviews: 1,
        requireCI: true,
        noForcePush: true,
        noDeletion: true,
        requireSignedCommits: true,
    },
    {
        id: "2",
        pattern: "develop",
        requirePR: true,
        requiredReviews: 0,
        requireCI: true,
        noForcePush: true,
        noDeletion: false,
        requireSignedCommits: false,
    },
];

const mockTagRules = [{ id: "1", pattern: "v*", allowedRoles: ["admin"] }];

const mockWebhooks = [
    { id: "1", url: "https://ci.example.com/hooks/gitarena", events: ["push", "pull_request"], active: true, createdAt: "3 months ago" },
    { id: "2", url: "https://discord.webhook.com/api/webhooks/123", events: ["push"], active: false, createdAt: "1 month ago" },
];

const mockDeployKeys = [
    { id: "1", title: "Production Server", fingerprint: "SHA256:AbCdEfGhIjKlMnOpQrStUvWxYz1234", readOnly: true, addedAt: "6 months ago" },
];

const mockRepoTokens = [
    {
        id: "1",
        name: "GitHub Actions",
        scopes: ["repo:read", "repo:write"],
        createdAt: "1 month ago",
        lastUsed: "2 hours ago",
        expiresAt: null,
    },
];

type Role = "viewer" | "supporter" | "coder" | "manager" | "admin";

// ── Nav items ──────────────────────────────────────────────────────────────────

const navItems: { id: Tab; label: string; icon: React.ElementType; wip?: boolean }[] = [
    { id: "general", label: "General", icon: Settings, wip: true },
    { id: "collaboration", label: "Collaboration", icon: Users },
    { id: "branches", label: "Branches & Tags", icon: ShieldCheck, wip: true },
    { id: "integrations", label: "Integrations", icon: Webhook, wip: true },
    { id: "danger", label: "Danger Zone", icon: AlertTriangle, wip: true },
];

// ── Shared helpers ─────────────────────────────────────────────────────────────

function SectionHeader({ title, description, wip }: { title: string; description?: string; wip?: boolean }) {
    return (
        <div className="mb-6">
            <div className="flex items-center gap-2">
                <h2 className="text-lg font-semibold">{title}</h2>
                {wip && (
                    <span className="px-1.5 py-0.5 text-[10px] font-medium rounded bg-secondary text-muted-foreground border border-border leading-none">
                        WIP
                    </span>
                )}
            </div>
            {description && <p className="text-sm text-muted-foreground mt-0.5 leading-relaxed">{description}</p>}
        </div>
    );
}

function FieldLabel({ children, optional }: { children: React.ReactNode; optional?: boolean }) {
    return (
        <label className="block text-sm font-medium mb-1.5">
            {children}
            {optional && <span className="ml-1.5 text-xs text-muted-foreground font-normal">optional</span>}
        </label>
    );
}

function Input({ className = "", ...props }: React.InputHTMLAttributes<HTMLInputElement>) {
    return (
        <input
            {...props}
            className={`w-full h-9 px-3 bg-card border border-border rounded-md text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring transition-shadow ${className}`}
        />
    );
}

function SaveButton({ children = "Save changes" }: { children?: React.ReactNode }) {
    return (
        <button className="inline-flex items-center gap-2 px-4 h-9 bg-foreground text-background text-sm font-medium rounded-md hover:opacity-90 transition-opacity">
            {children}
        </button>
    );
}

function DangerButton({ children, onClick }: { children: React.ReactNode; onClick?: () => void }) {
    return (
        <button
            onClick={onClick}
            className="inline-flex items-center gap-2 px-3 h-8 text-sm border border-destructive/50 text-destructive rounded-md hover:bg-destructive/10 transition-colors"
        >
            {children}
        </button>
    );
}

function Toggle({ checked, onChange }: { checked: boolean; onChange: (v: boolean) => void }) {
    return (
        <button
            role="switch"
            aria-checked={checked}
            onClick={() => onChange(!checked)}
            className={`relative inline-flex h-5 w-9 shrink-0 items-center rounded-full border-2 border-transparent transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 ${checked ? "bg-foreground" : "bg-secondary"}`}
        >
            <span
                className={`inline-block h-4 w-4 transform rounded-full bg-background transition-transform ${checked ? "translate-x-4" : "translate-x-0"}`}
            />
        </button>
    );
}

function ToggleRow({
    label,
    description,
    checked,
    onChange,
}: {
    label: string;
    description?: string;
    checked: boolean;
    onChange: (v: boolean) => void;
}) {
    return (
        <div className="flex items-center justify-between px-4 py-3 border-b border-border last:border-0">
            <div className="flex-1 min-w-0 pr-6">
                <p className="text-sm font-medium">{label}</p>
                {description && <p className="text-xs text-muted-foreground mt-0.5 leading-relaxed">{description}</p>}
            </div>
            <Toggle checked={checked} onChange={onChange} />
        </div>
    );
}

function Divider() {
    return <div className="border-t border-border my-8" />;
}

const roleColors: Record<Role, string> = {
    admin: "text-red-500 border-red-500/30 bg-red-500/5",
    manager: "text-orange-500 border-orange-500/30 bg-orange-500/5",
    coder: "text-blue-500 border-blue-500/30 bg-blue-500/5",
    supporter: "text-green-500 border-green-500/30 bg-green-500/5",
    viewer: "text-muted-foreground border-border bg-secondary",
};

// ── Tab panels ─────────────────────────────────────────────────────────────────

const roleMeta: Record<Role, string> = {
    viewer: "View code and issues",
    supporter: "View code · manage issues",
    coder: "View code · push branches",
    manager: "Push branches · manage issues",
    admin: "Full repository access",
};

interface Collaborator {
    userId: string;
    username: string;
    accessLevel: Role;
}

interface UpsertCollaboratorArg {
    username: string;
    accessLevel: Role;
}

interface RepoMeta {
    description: string | null;
    visibility: "public" | "internal" | "private";
    defaultBranch: string;
}

function CollaborationTab({ namespace, repo }: { namespace: string; repo: string }) {
    const apiBase = `/api/repos/${namespace}/${repo}/collaborators`;
    const metaUrl = `/api/repos/${namespace}/${repo}`;

    const { data: repoMeta } = useSWR<RepoMeta>(metaUrl, jsonFetcher);
    const { data: collaborators, isLoading, error } = useSWR<Collaborator[]>(apiBase, jsonFetcher);

    const { trigger: upsert, isMutating: isUpserting } = useSWRMutation<Collaborator, Error, string, UpsertCollaboratorArg>(
        apiBase,
        (url, { arg }) => putJsonFetcher<UpsertCollaboratorArg, Collaborator>(url, { arg }),
        { onSuccess: () => mutate(apiBase) }
    );

    const { trigger: remove } = useSWRMutation<void, Error, string, string>(
        apiBase,
        (url, { arg: username }) => deleteFetcher(`${url}/${username}`),
        { onSuccess: () => mutate(apiBase) }
    );

    const [addUsername, setAddUsername] = useState("");

    const isPrivate = repoMeta?.visibility === "private";
    const allRoles: Role[] = ["viewer", "supporter", "coder", "manager", "admin"];
    const roleOptions: Role[] = isPrivate ? allRoles : allRoles.filter((r) => r !== "viewer");

    const [addRole, setAddRole] = useState<Role>("supporter");

    const handleInvite = async () => {
        if (!addUsername.trim()) {
            return;
        }
        await upsert({ username: addUsername.trim(), accessLevel: addRole });
        setAddUsername("");
    };

    return (
        <div>
            <SectionHeader title="Collaborators" description="Manage who has access to this repository." />

            {/* Existing collaborators */}
            <div className="border border-border rounded-md overflow-hidden mb-6">
                {isLoading &&
                    Array.from({ length: 3 }, (_, i) => (
                        <div key={i} className={`flex items-center gap-3 px-4 py-3 ${i > 0 ? "border-t border-border" : ""}`}>
                            <Skeleton className="h-7 w-7 rounded-full shrink-0" />
                            <div className="flex-1 min-w-0">
                                <Skeleton className="h-4 w-28" />
                            </div>
                            <Skeleton className="h-6 w-20 rounded-md" />
                            <Skeleton className="h-7 w-7 rounded-md" />
                        </div>
                    ))}
                {error && <div className="px-4 py-6 text-sm text-destructive text-center">Failed to load collaborators.</div>}
                {collaborators &&
                    collaborators.map((c, i) => (
                        <div key={c.userId} className={`flex items-center gap-3 px-4 py-3 ${i > 0 ? "border-t border-border" : ""}`}>
                            <div className="h-7 w-7 flex items-center justify-center rounded-full bg-secondary border border-border text-xs font-medium shrink-0">
                                {c.username[0].toUpperCase()}
                            </div>
                            <div className="flex-1 min-w-0">
                                <p className="text-sm font-medium">{c.username}</p>
                            </div>
                            <DropdownMenu>
                                <DropdownMenuTrigger asChild>
                                    <button
                                        className={`flex items-center gap-1.5 px-2 py-1 text-xs border rounded-md transition-colors ${roleColors[c.accessLevel]}`}
                                    >
                                        {c.accessLevel}
                                        <ChevronDown className="h-3 w-3" />
                                    </button>
                                </DropdownMenuTrigger>
                                <DropdownMenuContent align="end" className="w-56">
                                    {roleOptions.map((r) => (
                                        <DropdownMenuItem
                                            key={r}
                                            onClick={() => upsert({ username: c.username, accessLevel: r })}
                                            className="flex items-center gap-2"
                                        >
                                            {r === c.accessLevel ? (
                                                <Check className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
                                            ) : (
                                                <span className="w-3.5 shrink-0" />
                                            )}
                                            <div>
                                                <p className={`text-xs font-medium capitalize ${roleColors[r].split(" ")[0]}`}>{r}</p>
                                                <p className="text-xs text-muted-foreground">{roleMeta[r]}</p>
                                            </div>
                                        </DropdownMenuItem>
                                    ))}
                                </DropdownMenuContent>
                            </DropdownMenu>
                            <button
                                onClick={() => remove(c.username)}
                                className="h-7 w-7 flex items-center justify-center rounded-md text-muted-foreground hover:text-destructive hover:bg-destructive/10 transition-colors"
                            >
                                <Trash2 className="h-3.5 w-3.5" />
                            </button>
                        </div>
                    ))}
            </div>

            {/* Add collaborator */}
            <div className="p-4 border border-border rounded-md mb-6">
                <p className="text-sm font-medium mb-3">Add collaborator</p>
                <div className="flex gap-2">
                    <Input
                        value={addUsername}
                        onChange={(e) => setAddUsername(e.target.value)}
                        placeholder="Username…"
                        onKeyDown={(e) => {
                            if (e.key === "Enter") {
                                handleInvite();
                            }
                        }}
                    />
                    <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                            <button className="flex items-center gap-1.5 px-3 h-9 text-sm border border-border rounded-md hover:bg-accent/50 transition-colors shrink-0">
                                <span className="capitalize">{addRole}</span>
                                <ChevronDown className="h-3.5 w-3.5 text-muted-foreground" />
                            </button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end" className="w-56">
                            {roleOptions.map((r) => (
                                <DropdownMenuItem key={r} onClick={() => setAddRole(r)} className="flex items-center gap-2">
                                    {r === addRole ? (
                                        <Check className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
                                    ) : (
                                        <span className="w-3.5 shrink-0" />
                                    )}
                                    <div>
                                        <p className={`text-xs font-medium capitalize ${roleColors[r].split(" ")[0]}`}>{r}</p>
                                        <p className="text-xs text-muted-foreground">{roleMeta[r]}</p>
                                    </div>
                                </DropdownMenuItem>
                            ))}
                        </DropdownMenuContent>
                    </DropdownMenu>
                    <button
                        onClick={handleInvite}
                        disabled={isUpserting || !addUsername.trim()}
                        className="inline-flex items-center gap-1.5 px-3 h-9 bg-foreground text-background text-sm font-medium rounded-md hover:opacity-90 transition-opacity disabled:opacity-50 disabled:cursor-not-allowed shrink-0"
                    >
                        <Plus className="h-4 w-4" />
                        Add
                    </button>
                </div>
            </div>
        </div>
    );
}

function GeneralTab({ namespace, repo }: { namespace: string; repo: string }) {
    const { data: meta } = useSWR<RepoMeta>(`/api/repos/${namespace}/${repo}`, jsonFetcher);

    const description = meta?.description ?? "";
    const visibility = meta?.visibility ?? "public";
    const defaultBranch = meta?.defaultBranch ?? "";
    const [topics, setTopics] = useState(repoData.topics);
    const [newTopic, setNewTopic] = useState("");
    const [features, setFeatures] = useState(repoData.features);

    const visibilityOptions: { value: typeof visibility; label: string; description: string; icon: React.ElementType }[] = [
        { value: "public", label: "Public", description: "Anyone can see this repository.", icon: Globe },
        { value: "internal", label: "Internal", description: "Only logged-in users can see this repository.", icon: Lock },
        { value: "private", label: "Private", description: "Only collaborators can see this repository.", icon: Lock },
    ];

    const addTopic = () => {
        const t = newTopic.trim().toLowerCase();
        if (t && !topics.includes(t)) {
            setTopics((prev) => [...prev, t]);
        }
        setNewTopic("");
    };

    return (
        <div>
            <SectionHeader title="General" description="Core repository settings and metadata." wip />

            {/* Description */}
            <div className="mb-6">
                <FieldLabel optional>Description</FieldLabel>
                <textarea
                    value={description}
                    readOnly
                    rows={3}
                    className="w-full px-3 py-2 bg-card border border-border rounded-md text-sm placeholder:text-muted-foreground focus:outline-none resize-none opacity-60 cursor-not-allowed"
                />
            </div>

            <SaveButton />

            <Divider />

            {/* Visibility */}
            <SectionHeader title="Visibility" description="Control who can see and interact with this repository." wip />
            <div className="space-y-2 mb-6">
                {visibilityOptions.map(({ value, label, description: desc, icon: Icon }) => (
                    <div
                        key={value}
                        className={`flex items-start gap-3 p-3 border rounded-md transition-colors ${
                            visibility === value ? "border-foreground/40 bg-accent/30" : "border-border opacity-40"
                        }`}
                    >
                        <div
                            className={`mt-0.5 h-4 w-4 shrink-0 rounded-full border-2 flex items-center justify-center transition-colors ${visibility === value ? "border-foreground" : "border-border"}`}
                        >
                            {visibility === value && <div className="h-1.5 w-1.5 rounded-full bg-foreground" />}
                        </div>
                        <Icon className="h-4 w-4 mt-0.5 shrink-0 text-muted-foreground" />
                        <div>
                            <p className="text-sm font-medium">{label}</p>
                            <p className="text-xs text-muted-foreground">{desc}</p>
                        </div>
                    </div>
                ))}
            </div>
            <SaveButton>Update visibility</SaveButton>

            <Divider />

            {/* Default branch */}
            <SectionHeader title="Default branch" description="The branch shown by default when visiting the repository." wip />
            <div className="mb-6">
                <div className="inline-flex items-center gap-2 px-3 h-9 border border-border rounded-md text-sm opacity-60 cursor-not-allowed">
                    <GitBranch className="h-3.5 w-3.5 text-muted-foreground" />
                    <code className="font-mono">{defaultBranch}</code>
                </div>
            </div>
            <SaveButton>Update default branch</SaveButton>

            <Divider />

            {/* Topics */}
            <SectionHeader title="Topics" description="Add topics to help people discover this repository." wip />
            <div className="flex flex-wrap gap-2 mb-3">
                {topics.map((t) => (
                    <span
                        key={t}
                        className="inline-flex items-center gap-1 pl-2 pr-1 py-0.5 text-xs border border-border rounded-full bg-secondary text-muted-foreground"
                    >
                        {t}
                        <button
                            onClick={() => setTopics((prev) => prev.filter((x) => x !== t))}
                            className="ml-0.5 hover:text-foreground transition-colors"
                        >
                            <X className="h-3 w-3" />
                        </button>
                    </span>
                ))}
                {topics.length === 0 && <p className="text-xs text-muted-foreground">No topics added yet.</p>}
            </div>
            <div className="flex gap-2 mb-6">
                <Input
                    value={newTopic}
                    onChange={(e) => setNewTopic(e.target.value)}
                    placeholder="Add a topic…"
                    onKeyDown={(e) => e.key === "Enter" && (e.preventDefault(), addTopic())}
                    className="max-w-64"
                />
                <button
                    onClick={addTopic}
                    className="inline-flex items-center gap-1.5 px-3 h-9 text-sm border border-border rounded-md hover:bg-accent/50 transition-colors"
                >
                    <Plus className="h-3.5 w-3.5" />
                    Add
                </button>
            </div>
            <SaveButton>Save topics</SaveButton>

            <Divider />

            {/* Features */}
            <SectionHeader title="Features" description="Toggle optional repository features on or off." wip />
            <div className="border border-border rounded-md overflow-hidden mb-6">
                <ToggleRow
                    label="Issues"
                    description="Enable the issue tracker for bug reports and feature requests."
                    checked={features.issues}
                    onChange={(v) => setFeatures((f) => ({ ...f, issues: v }))}
                />
                <ToggleRow
                    label="Wiki"
                    description="Enable the built-in wiki for documentation."
                    checked={features.wiki}
                    onChange={(v) => setFeatures((f) => ({ ...f, wiki: v }))}
                />
                <ToggleRow
                    label="Packages"
                    description="Enable the package registry for this repository."
                    checked={features.packages}
                    onChange={(v) => setFeatures((f) => ({ ...f, packages: v }))}
                />
                <ToggleRow
                    label="Releases"
                    description="Enable the releases tab for tagging and publishing releases."
                    checked={features.releases}
                    onChange={(v) => setFeatures((f) => ({ ...f, releases: v }))}
                />
                <ToggleRow
                    label="Allow forking"
                    description="Allow other users to fork this repository."
                    checked={features.forking}
                    onChange={(v) => setFeatures((f) => ({ ...f, forking: v }))}
                />
            </div>
        </div>
    );
}

function BranchesTab() {
    const [rules, setRules] = useState(mockBranchRules);
    const [tagRules] = useState(mockTagRules);
    const [mergeStrategies, setMergeStrategies] = useState(repoData.mergeStrategies);
    const [autoDelete, setAutoDelete] = useState(repoData.autoDeleteBranch);
    const [requireSigned, setRequireSigned] = useState(repoData.requireSignedCommits);

    const toggleRule = (id: string, key: keyof (typeof rules)[0]) => {
        setRules((prev) => prev.map((r) => (r.id === id ? { ...r, [key]: !r[key as string] } : r)));
    };

    return (
        <div>
            {/* Branch protection rules */}
            <SectionHeader
                title="Branch protection rules"
                description="Protect important branches from force-pushes, deletions, and direct commits."
            />

            <div className="space-y-3 mb-6">
                {rules.map((rule) => (
                    <div key={rule.id} className="border border-border rounded-md overflow-hidden">
                        <div className="flex items-center gap-3 px-4 py-3 border-b border-border bg-secondary/30">
                            <GitBranch className="h-4 w-4 text-muted-foreground shrink-0" />
                            <code className="text-sm font-mono font-medium flex-1">{rule.pattern}</code>
                            <button className="h-7 w-7 flex items-center justify-center rounded-md text-muted-foreground hover:text-destructive hover:bg-destructive/10 transition-colors">
                                <Trash2 className="h-3.5 w-3.5" />
                            </button>
                        </div>
                        <div className="p-4 space-y-0">
                            {[
                                { key: "requirePR", label: "Require pull request before merging" },
                                { key: "requireCI", label: "Require CI to pass before merging" },
                                { key: "noForcePush", label: "Disallow force pushes" },
                                { key: "noDeletion", label: "Disallow branch deletion" },
                                { key: "requireSignedCommits", label: "Require signed commits" },
                            ].map(({ key, label }) => (
                                <div key={key} className="flex items-center justify-between py-2 border-b border-border last:border-0">
                                    <span className="text-sm">{label}</span>
                                    <Toggle
                                        checked={rule[key as keyof typeof rule] as boolean}
                                        onChange={() => toggleRule(rule.id, key as keyof typeof rule)}
                                    />
                                </div>
                            ))}
                            {rule.requirePR && (
                                <div className="flex items-center justify-between py-2 border-b border-border last:border-0">
                                    <span className="text-sm">Required approving reviews</span>
                                    <div className="flex items-center gap-2">
                                        <button
                                            onClick={() =>
                                                setRules((prev) =>
                                                    prev.map((r) =>
                                                        r.id === rule.id ? { ...r, requiredReviews: Math.max(0, r.requiredReviews - 1) } : r
                                                    )
                                                )
                                            }
                                            className="h-6 w-6 flex items-center justify-center border border-border rounded hover:bg-accent/50 text-sm"
                                        >
                                            −
                                        </button>
                                        <span className="text-sm font-mono w-4 text-center">{rule.requiredReviews}</span>
                                        <button
                                            onClick={() =>
                                                setRules((prev) =>
                                                    prev.map((r) =>
                                                        r.id === rule.id ? { ...r, requiredReviews: Math.min(6, r.requiredReviews + 1) } : r
                                                    )
                                                )
                                            }
                                            className="h-6 w-6 flex items-center justify-center border border-border rounded hover:bg-accent/50 text-sm"
                                        >
                                            +
                                        </button>
                                    </div>
                                </div>
                            )}
                        </div>
                    </div>
                ))}
            </div>

            <button className="inline-flex items-center gap-2 px-3 h-9 text-sm border border-border rounded-md hover:bg-accent/50 transition-colors mb-6">
                <Plus className="h-4 w-4" />
                Add branch protection rule
            </button>

            <Divider />

            {/* Tag protection */}
            <SectionHeader title="Tag protection rules" description="Restrict who can create tags matching a pattern." />
            <div className="border border-border rounded-md overflow-hidden mb-4">
                {tagRules.map((rule, i) => (
                    <div key={rule.id} className={`flex items-center gap-3 px-4 py-3 ${i > 0 ? "border-t border-border" : ""}`}>
                        <Tag className="h-4 w-4 text-muted-foreground shrink-0" />
                        <code className="text-sm font-mono flex-1">{rule.pattern}</code>
                        <span className="text-xs text-muted-foreground">allowed: {rule.allowedRoles.join(", ")}</span>
                        <button className="h-7 w-7 flex items-center justify-center rounded-md text-muted-foreground hover:text-destructive hover:bg-destructive/10 transition-colors">
                            <Trash2 className="h-3.5 w-3.5" />
                        </button>
                    </div>
                ))}
            </div>
            <button className="inline-flex items-center gap-2 px-3 h-9 text-sm border border-border rounded-md hover:bg-accent/50 transition-colors">
                <Plus className="h-4 w-4" />
                Add tag protection rule
            </button>

            <Divider />

            {/* Merge strategies */}
            <SectionHeader
                title="Merge strategies"
                description="Choose which merge methods contributors may use when merging pull requests."
            />
            <div className="border border-border rounded-md overflow-hidden mb-6">
                {[
                    { key: "mergeCommit", label: "Merge commit", description: "Add all commits from the branch with a merge commit." },
                    { key: "squash", label: "Squash merge", description: "Combine all commits into one before merging." },
                    { key: "rebase", label: "Rebase merge", description: "Rebase commits onto the base branch individually." },
                ].map(({ key, label, description }) => (
                    <div key={key} className="flex items-center justify-between px-4 py-3 border-b border-border last:border-0">
                        <div>
                            <p className="text-sm font-medium">{label}</p>
                            <p className="text-xs text-muted-foreground">{description}</p>
                        </div>
                        <Toggle
                            checked={mergeStrategies[key as keyof typeof mergeStrategies]}
                            onChange={(v) => setMergeStrategies((m) => ({ ...m, [key]: v }))}
                        />
                    </div>
                ))}
            </div>

            {/* Auto-delete + signed commits */}
            <div className="border border-border rounded-md overflow-hidden mb-6">
                <ToggleRow
                    label="Auto-delete head branch"
                    description="Automatically delete the source branch after a pull request is merged."
                    checked={autoDelete}
                    onChange={setAutoDelete}
                />
                <ToggleRow
                    label="Require signed commits"
                    description="Require all commits pushed to this repository to be signed with GPG or SSH."
                    checked={requireSigned}
                    onChange={setRequireSigned}
                />
            </div>

            <SaveButton>Save branch settings</SaveButton>
        </div>
    );
}

function IntegrationsTab() {
    const [webhooks, setWebhooks] = useState(mockWebhooks);
    const [deployKeys, setDeployKeys] = useState(mockDeployKeys);
    const [tokens] = useState(mockRepoTokens);
    const [copied, setCopied] = useState<string | null>(null);

    const copyToken = (id: string) => {
        setCopied(id);
        setTimeout(() => setCopied(null), 2000);
    };

    return (
        <div>
            {/* Webhooks */}
            <SectionHeader title="Webhooks" description="Send HTTP POST requests to external URLs when repository events occur." />

            <div className="border border-border rounded-md overflow-hidden mb-4">
                {webhooks.map((wh, i) => (
                    <div key={wh.id} className={`flex items-start gap-3 px-4 py-3 ${i > 0 ? "border-t border-border" : ""}`}>
                        <div className={`mt-1 h-2 w-2 rounded-full shrink-0 ${wh.active ? "bg-green-500" : "bg-muted-foreground/30"}`} />
                        <div className="flex-1 min-w-0">
                            <code className="text-xs font-mono text-foreground break-all">{wh.url}</code>
                            <div className="flex flex-wrap gap-1 mt-1.5">
                                {wh.events.map((e) => (
                                    <span
                                        key={e}
                                        className="px-1.5 py-0.5 text-[10px] border border-border rounded bg-secondary text-muted-foreground"
                                    >
                                        {e}
                                    </span>
                                ))}
                            </div>
                            <p className="text-xs text-muted-foreground mt-1">Added {wh.createdAt}</p>
                        </div>
                        <button
                            onClick={() => setWebhooks((prev) => prev.filter((x) => x.id !== wh.id))}
                            className="h-7 w-7 flex items-center justify-center rounded-md text-muted-foreground hover:text-destructive hover:bg-destructive/10 transition-colors shrink-0 mt-0.5"
                        >
                            <Trash2 className="h-3.5 w-3.5" />
                        </button>
                    </div>
                ))}
            </div>

            <button className="inline-flex items-center gap-2 px-3 h-9 text-sm border border-border rounded-md hover:bg-accent/50 transition-colors mb-6">
                <Plus className="h-4 w-4" />
                Add webhook
            </button>

            <Divider />

            {/* Deploy keys */}
            <SectionHeader title="Deploy keys" description="Read-only SSH keys for CI/CD pipelines and deployment scripts." />

            <div className="border border-border rounded-md overflow-hidden mb-4">
                {deployKeys.map((key, i) => (
                    <div key={key.id} className={`flex items-center gap-3 px-4 py-3 ${i > 0 ? "border-t border-border" : ""}`}>
                        <KeyRound className="h-4 w-4 text-muted-foreground shrink-0" />
                        <div className="flex-1 min-w-0">
                            <p className="text-sm font-medium">{key.title}</p>
                            <p className="text-xs text-muted-foreground font-mono truncate">{key.fingerprint}</p>
                            <p className="text-xs text-muted-foreground mt-0.5">
                                Added {key.addedAt} · {key.readOnly ? "Read-only" : "Read/write"}
                            </p>
                        </div>
                        <button
                            onClick={() => setDeployKeys((prev) => prev.filter((x) => x.id !== key.id))}
                            className="h-7 w-7 flex items-center justify-center rounded-md text-muted-foreground hover:text-destructive hover:bg-destructive/10 transition-colors shrink-0"
                        >
                            <Trash2 className="h-3.5 w-3.5" />
                        </button>
                    </div>
                ))}
            </div>

            <button className="inline-flex items-center gap-2 px-3 h-9 text-sm border border-border rounded-md hover:bg-accent/50 transition-colors mb-6">
                <Plus className="h-4 w-4" />
                Add deploy key
            </button>

            <Divider />

            {/* Repository access tokens */}
            <SectionHeader
                title="Repository access tokens"
                description="Scoped tokens for programmatic access to this specific repository."
            />

            <div className="border border-border rounded-md overflow-hidden mb-4">
                {tokens.map((token, i) => (
                    <div key={token.id} className={`flex items-center gap-3 px-4 py-3 ${i > 0 ? "border-t border-border" : ""}`}>
                        <Code2 className="h-4 w-4 text-muted-foreground shrink-0" />
                        <div className="flex-1 min-w-0">
                            <p className="text-sm font-medium">{token.name}</p>
                            <div className="flex flex-wrap gap-1 mt-1">
                                {token.scopes.map((s) => (
                                    <code
                                        key={s}
                                        className="text-[10px] px-1.5 py-0.5 border border-border rounded bg-secondary text-muted-foreground"
                                    >
                                        {s}
                                    </code>
                                ))}
                            </div>
                            <p className="text-xs text-muted-foreground mt-1">
                                Created {token.createdAt} · Last used {token.lastUsed}
                            </p>
                        </div>
                        <button
                            onClick={() => copyToken(token.id)}
                            className="h-7 w-7 flex items-center justify-center rounded-md text-muted-foreground hover:text-foreground hover:bg-accent/50 transition-colors"
                            title="Copy token"
                        >
                            {copied === token.id ? <Check className="h-3.5 w-3.5 text-green-500" /> : <Copy className="h-3.5 w-3.5" />}
                        </button>
                    </div>
                ))}
            </div>

            <button className="inline-flex items-center gap-2 px-3 h-9 bg-foreground text-background text-sm font-medium rounded-md hover:opacity-90 transition-opacity">
                <Plus className="h-4 w-4" />
                Generate access token
            </button>
        </div>
    );
}

function DangerTab({ org, repo }: { org: string; repo: string }) {
    const [archiveConfirm, setArchiveConfirm] = useState(false);
    const [deleteConfirm, setDeleteConfirm] = useState(false);
    const [deleteInput, setDeleteInput] = useState("");

    const fullName = `${org}/${repo}`;

    return (
        <div>
            <SectionHeader title="Danger Zone" description="These actions are irreversible. Please proceed with caution." />

            <div className="border border-destructive/30 rounded-md overflow-hidden space-y-0">
                {/* Rename */}
                <div className="flex items-start justify-between gap-4 p-4 border-b border-destructive/20">
                    <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2 mb-1">
                            <Settings className="h-4 w-4 text-muted-foreground" />
                            <p className="text-sm font-medium">Rename this repository</p>
                        </div>
                        <p className="text-xs text-muted-foreground leading-relaxed">
                            Renaming will break existing clone URLs. A redirect will be set up automatically from the old name.
                        </p>
                    </div>
                    <div className="shrink-0">
                        <DangerButton>Rename repository</DangerButton>
                    </div>
                </div>
                {/* Transfer ownership */}
                <div className="flex items-start justify-between gap-4 p-4 border-b border-destructive/20">
                    <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2 mb-1">
                            <Users className="h-4 w-4 text-muted-foreground" />
                            <p className="text-sm font-medium">Transfer ownership</p>
                        </div>
                        <p className="text-xs text-muted-foreground leading-relaxed">
                            Transfer this repository to another user or organization. You will lose admin access unless the new owner
                            re-invites you.
                        </p>
                    </div>
                    <div className="shrink-0">
                        <DangerButton>Transfer repository</DangerButton>
                    </div>
                </div>
                {/* Archive */}{" "}
                <div className="flex items-start justify-between gap-4 p-4 border-b border-destructive/20">
                    <div>
                        <div className="flex items-center gap-2 mb-1">
                            <Archive className="h-4 w-4 text-muted-foreground" />
                            <p className="text-sm font-medium">Archive this repository</p>
                        </div>
                        <p className="text-xs text-muted-foreground leading-relaxed">
                            Mark this repository as read-only. Issues, merge requests, and commits will be locked. The repository can be
                            unarchived later.
                        </p>
                        {archiveConfirm && (
                            <div className="mt-3 flex items-center gap-2 p-3 bg-amber-500/10 border border-amber-500/30 rounded-md">
                                <AlertCircle className="h-4 w-4 text-amber-500 shrink-0" />
                                <p className="text-xs text-muted-foreground">
                                    This will make the repository read-only and hide it from search.
                                </p>
                            </div>
                        )}
                    </div>
                    <div className="flex gap-2 shrink-0">
                        {archiveConfirm ? (
                            <>
                                <button
                                    onClick={() => setArchiveConfirm(false)}
                                    className="inline-flex items-center gap-1.5 px-3 h-8 text-sm border border-border rounded-md hover:bg-accent/50 transition-colors"
                                >
                                    Cancel
                                </button>
                                <DangerButton>Confirm archive</DangerButton>
                            </>
                        ) : (
                            <button
                                onClick={() => setArchiveConfirm(true)}
                                className="inline-flex items-center gap-1.5 px-3 h-8 text-sm border border-border rounded-md hover:bg-accent/50 transition-colors"
                            >
                                Archive repository
                            </button>
                        )}
                    </div>
                </div>
                {/* Delete */}
                <div className="flex items-start justify-between gap-4 p-4">
                    <div className="flex-1">
                        <div className="flex items-center gap-2 mb-1">
                            <Trash2 className="h-4 w-4 text-destructive" />
                            <p className="text-sm font-medium text-destructive">Delete this repository</p>
                        </div>
                        <p className="text-xs text-muted-foreground leading-relaxed mb-3">
                            Once deleted, this repository cannot be recovered. All issues, merge requests, comments, and commits will be
                            permanently removed.
                        </p>

                        {deleteConfirm && (
                            <div className="mt-2 space-y-3">
                                <div className="p-3 bg-destructive/5 border border-destructive/20 rounded-md">
                                    <p className="text-xs text-muted-foreground">
                                        Please type <code className="font-mono font-semibold text-foreground">{fullName}</code> to confirm.
                                    </p>
                                </div>
                                <Input
                                    value={deleteInput}
                                    onChange={(e) => setDeleteInput(e.target.value)}
                                    placeholder={fullName}
                                    className="border-destructive/40 focus:ring-destructive/40"
                                />
                                <div className="flex gap-2">
                                    <button
                                        onClick={() => {
                                            setDeleteConfirm(false);
                                            setDeleteInput("");
                                        }}
                                        className="inline-flex items-center gap-1.5 px-3 h-8 text-sm border border-border rounded-md hover:bg-accent/50 transition-colors"
                                    >
                                        Cancel
                                    </button>
                                    <button
                                        disabled={deleteInput !== fullName}
                                        className="inline-flex items-center gap-2 px-3 h-8 text-sm font-medium bg-destructive text-destructive-foreground rounded-md hover:opacity-90 transition-opacity disabled:opacity-40 disabled:cursor-not-allowed"
                                    >
                                        <Trash2 className="h-3.5 w-3.5" />I understand, delete this repository
                                    </button>
                                </div>
                            </div>
                        )}
                    </div>
                    {!deleteConfirm && (
                        <div className="shrink-0">
                            <DangerButton onClick={() => setDeleteConfirm(true)}>
                                <Trash2 className="h-3.5 w-3.5" />
                                Delete repository
                            </DangerButton>
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
}

// ── Page ───────────────────────────────────────────────────────────────────────

export default function RepoSettingsPage() {
    const params = useParams();
    const namespace = (params.user ?? params.org) as string;
    const repoName = params.repo as string;

    const [activeTab, setActiveTab] = useState<Tab>("general");

    const tabContent: Record<Tab, React.ReactNode> = {
        general: <GeneralTab namespace={namespace} repo={repoName} />,
        collaboration: <CollaborationTab namespace={namespace} repo={repoName} />,
        branches: <BranchesTab />,
        integrations: <IntegrationsTab />,
        danger: <DangerTab org={namespace} repo={repoName} />,
    };

    return (
        <div className="flex flex-col h-screen overflow-hidden bg-background text-foreground font-sans">
            <TopBar
                breadcrumb={[
                    { label: namespace, href: `/${namespace}` },
                    { label: repoName, href: `/${namespace}/${repoName}` },
                    { label: "Settings" },
                ]}
                navLinks={[
                    { label: "Code", href: `/${namespace}/${repoName}`, icon: <GitBranch className="h-[18px] w-[18px]" /> },
                    { label: "Issues", href: `/${namespace}/${repoName}/issues`, icon: <GitMerge className="h-[18px] w-[18px]" /> },
                    {
                        label: "Merge Requests",
                        href: `/${namespace}/${repoName}/merge-requests`,
                        icon: <GitMerge className="h-[18px] w-[18px]" />,
                    },
                ]}
                hasNotifications
            />

            <div className="flex-1 flex overflow-hidden">
                {/* Left sidebar */}
                <aside className="w-60 border-r border-border shrink-0 overflow-y-auto">
                    <div className="p-3">
                        <h3 className="px-3 mb-2 mt-2 text-xs font-medium uppercase tracking-widest text-muted-foreground">
                            Repository Settings
                        </h3>
                        <nav className="space-y-0.5">
                            {navItems.map((item) => (
                                <button
                                    key={item.id}
                                    onClick={() => setActiveTab(item.id)}
                                    className={`w-full flex items-center gap-2.5 px-3 py-2 text-sm rounded-md transition-colors text-left ${
                                        activeTab === item.id
                                            ? "bg-accent text-foreground"
                                            : item.id === "danger"
                                              ? "text-destructive/70 hover:text-destructive hover:bg-destructive/10"
                                              : "text-muted-foreground hover:text-foreground hover:bg-accent/50"
                                    }`}
                                >
                                    <item.icon className="h-4 w-4 shrink-0" />
                                    <span className="flex-1">{item.label}</span>
                                    {item.wip && (
                                        <span className="px-1.5 py-0.5 text-[10px] font-medium rounded bg-secondary text-muted-foreground border border-border leading-none">
                                            WIP
                                        </span>
                                    )}
                                </button>
                            ))}
                        </nav>
                    </div>
                </aside>

                {/* Main content */}
                <main className="flex-1 overflow-y-auto">
                    <div className="max-w-2xl mx-auto px-8 py-8">{tabContent[activeTab]}</div>
                </main>
            </div>
        </div>
    );
}

"use client";

import { useState } from "react";
import useSWR from "swr";
import useSWRMutation from "swr/mutation";
import { addDays, format, isValid } from "date-fns";
import { BookMarked, Building2, Globe, Loader2, Lock, Search } from "lucide-react";
import { toast } from "sonner";
import { Badge } from "@/components/ui/badge";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { SlidingTabBar } from "@/components/ui/sliding-tab-bar";
import { TokenPermissionPicker } from "@/components/token-permission-picker";
import { patchJsonFetcher, postJsonFetcher } from "@/lib/fetchers";
import {
    CreateTokenRequest,
    CreateTokenResponse,
    TokenOwner,
    TokenResponse,
    TokenScope,
    TokenType,
    UpdateTokenRequest,
    scopesFor,
    tokenScopeLabels,
    tokenTypeLabels,
    tokenTypesFor,
} from "@/lib/tokens";

interface OrgEntry {
    id: string;
    name: string;
}

interface RepoEntry {
    id: string;
    name: string;
    visibility: "public" | "internal" | "private";
}

interface UserProfileResponse {
    repos: RepoEntry[];
}

const expiryPresets = [
    { value: "7", label: "7 days" },
    { value: "30", label: "30 days" },
    { value: "60", label: "60 days" },
    { value: "90", label: "90 days" },
    { value: "365", label: "1 year" },
    { value: "custom", label: "Custom" },
    { value: "never", label: "No expiration" },
];

/** Resolves the expiry selection to the Unix timestamp the API expects. */
function expiresAtFrom(preset: string, customDate: string): number | null {
    if (preset === "never") {
        return null;
    }

    const date = preset === "custom" ? new Date(customDate) : addDays(new Date(), Number(preset));
    return Math.floor(date.getTime() / 1000);
}

interface ScopeTargetPickerProps {
    owner: TokenOwner;
    scopeOrgs: string[];
    scopeRepos: string[];
    onScopeOrgsChange: (orgs: string[]) => void;
    onScopeReposChange: (repos: string[]) => void;
}

type ScopeTab = "orgs" | "repos";

interface ScopeRowProps {
    checked: boolean;
    onToggle: () => void;
    icon: React.ElementType;
    children: React.ReactNode;
}

function ScopeRow({ checked, onToggle, icon: Icon, children }: ScopeRowProps) {
    return (
        <label className="flex items-center gap-2.5 px-2.5 py-2 rounded-md cursor-pointer hover:bg-accent/30 transition-colors min-w-0">
            <Checkbox checked={checked} onCheckedChange={onToggle} />
            <Icon className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
            {children}
        </label>
    );
}

/** Picks the organizations and repositories a `selected` scoped token is limited to. */
function ScopeTargetPicker({ owner, scopeOrgs, scopeRepos, onScopeOrgsChange, onScopeReposChange }: ScopeTargetPickerProps) {
    const username = owner.kind === "user" ? owner.username : null;
    const orgName = owner.kind === "org" ? owner.name : null;
    const namespace = username ?? orgName;

    const { data: orgs, isLoading: orgsLoading } = useSWR<OrgEntry[]>(username ? `/api/users/${username}/orgs` : null);
    const { data: profile, isLoading: profileLoading } = useSWR<UserProfileResponse>(username ? `/api/users/${username}` : null);
    const { data: orgRepos, isLoading: orgReposLoading } = useSWR<RepoEntry[]>(orgName ? `/api/orgs/${orgName}/repos` : null);

    const repos = username ? profile?.repos : orgRepos;
    const reposLoading = username ? profileLoading : orgReposLoading;

    // Organization tokens can only be limited to repositories, so they get no organization tab
    const [tab, setTab] = useState<ScopeTab>(username ? "orgs" : "repos");
    const [filter, setFilter] = useState("");

    const query = filter.trim().toLowerCase();
    const matchingOrgs = (orgs ?? []).filter((org) => org.name.toLowerCase().includes(query));
    const matchingRepos = (repos ?? []).filter((repo) => repo.name.toLowerCase().includes(query));

    const showingOrgs = tab === "orgs";
    const isLoading = showingOrgs ? orgsLoading : reposLoading;
    const total = showingOrgs ? (orgs?.length ?? 0) : (repos?.length ?? 0);
    const matching = showingOrgs ? matchingOrgs.length : matchingRepos.length;
    const selectedHere = showingOrgs ? scopeOrgs : scopeRepos;

    const tabs = [
        ...(username ? [{ id: "orgs" as const, label: "Organizations", icon: Building2, count: scopeOrgs.length || null }] : []),
        { id: "repos" as const, label: "Repositories", icon: BookMarked, count: scopeRepos.length || null },
    ];

    function toggle(id: string, current: string[], onChange: (ids: string[]) => void) {
        onChange(current.includes(id) ? current.filter((entry) => entry !== id) : [...current, id]);
    }

    function changeTab(next: ScopeTab) {
        setTab(next);
        setFilter("");
    }

    function clearSelection() {
        if (showingOrgs) {
            onScopeOrgsChange([]);
            return;
        }

        onScopeReposChange([]);
    }

    return (
        <div className="border border-border rounded-md overflow-hidden">
            <div className="flex items-center gap-3 px-3 py-2.5 border-b border-border">
                <SlidingTabBar items={tabs} active={tab} onChange={changeTab} />
                {selectedHere.length > 0 && (
                    <button
                        type="button"
                        onClick={clearSelection}
                        className="ml-auto text-xs text-muted-foreground hover:text-foreground transition-colors underline underline-offset-2"
                    >
                        Clear
                    </button>
                )}
            </div>

            <div className="flex items-center gap-2 px-3 h-9 border-b border-border">
                <Search className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                <input
                    type="text"
                    value={filter}
                    onChange={(e) => setFilter(e.target.value)}
                    placeholder={showingOrgs ? "Filter organizations…" : "Filter repositories…"}
                    className="flex-1 min-w-0 bg-transparent text-xs focus:outline-none placeholder:text-muted-foreground"
                />
                <span className="text-xs text-muted-foreground tabular-nums shrink-0">{selectedHere.length} selected</span>
            </div>

            <div className="max-h-56 overflow-y-auto scrollbar-dark p-1.5">
                {isLoading && (
                    <div className="space-y-1">
                        {[0, 1, 2, 3].map((i) => (
                            <div key={i} className="flex items-center gap-2.5 px-2.5 py-2">
                                <Skeleton className="h-3.5 w-3.5" />
                                <Skeleton className="h-3.5 w-40" />
                            </div>
                        ))}
                    </div>
                )}

                {!isLoading && total === 0 && (
                    <p className="px-2.5 py-6 text-center text-xs text-muted-foreground">
                        {showingOrgs ? "You are not a member of any organization." : "There are no repositories to choose from."}
                    </p>
                )}

                {!isLoading && total > 0 && matching === 0 && (
                    <p className="px-2.5 py-6 text-center text-xs text-muted-foreground">No matches for “{filter.trim()}”.</p>
                )}

                {!isLoading &&
                    showingOrgs &&
                    matchingOrgs.map((org) => (
                        <ScopeRow
                            key={org.id}
                            icon={Building2}
                            checked={scopeOrgs.includes(org.id)}
                            onToggle={() => toggle(org.id, scopeOrgs, onScopeOrgsChange)}
                        >
                            <span className="text-xs truncate">{org.name}</span>
                        </ScopeRow>
                    ))}

                {!isLoading &&
                    !showingOrgs &&
                    matchingRepos.map((repo) => (
                        <ScopeRow
                            key={repo.id}
                            icon={BookMarked}
                            checked={scopeRepos.includes(repo.id)}
                            onToggle={() => toggle(repo.id, scopeRepos, onScopeReposChange)}
                        >
                            <span className="text-xs truncate">
                                <span className="text-muted-foreground">{namespace}/</span>
                                {repo.name}
                            </span>
                            {repo.visibility === "private" && (
                                <Badge variant="secondary" className="ml-auto shrink-0">
                                    <Lock className="h-3 w-3" />
                                    Private
                                </Badge>
                            )}
                            {repo.visibility === "internal" && (
                                <Badge variant="outline" className="ml-auto shrink-0">
                                    <Globe className="h-3 w-3" />
                                    Internal
                                </Badge>
                            )}
                        </ScopeRow>
                    ))}
            </div>
        </div>
    );
}

interface TokenFormProps {
    owner: TokenOwner;
    /** Token being edited, or null when creating a new one */
    token: TokenResponse | null;
    onCancel: () => void;
    onCreated: (result: CreateTokenResponse) => void;
    onUpdated: () => void;
}

/** Create and edit form for a token. Expiration and type are fixed once a token exists, so they are only offered on create. */
export function TokenForm({ owner, token, onCancel, onCreated, onUpdated }: TokenFormProps) {
    const availableTypes = tokenTypesFor(owner);

    const [name, setName] = useState(token?.name ?? "");
    const [tokenType, setTokenType] = useState<TokenType>(token?.tokenType ?? availableTypes[0]);
    const [scope, setScope] = useState<TokenScope>(token?.scope ?? "all");
    const [permissions, setPermissions] = useState<string[]>(token?.permissions ?? []);
    const [scopeOrgs, setScopeOrgs] = useState<string[]>(token?.scopeOrgs ?? []);
    const [scopeRepos, setScopeRepos] = useState<string[]>(token?.scopeRepos ?? []);
    const [expiryPreset, setExpiryPreset] = useState("30");
    const [customExpiry, setCustomExpiry] = useState("");

    const { trigger: createToken, isMutating: creating } = useSWRMutation(
        "/api/tokens",
        postJsonFetcher<CreateTokenRequest, CreateTokenResponse>,
        {
            onSuccess: onCreated,
            onError: (err: Error) => toast.error(err.message),
        }
    );

    const { trigger: updateToken, isMutating: updating } = useSWRMutation(
        `/api/tokens/${token?.id}`,
        patchJsonFetcher<UpdateTokenRequest, TokenResponse>,
        {
            onSuccess: onUpdated,
            onError: (err: Error) => toast.error(err.message),
        }
    );

    const availableScopes = scopesFor(tokenType);
    const targetsMissing = scope === "selected" && scopeOrgs.length === 0 && scopeRepos.length === 0;
    const expiryInvalid = !token && expiryPreset === "custom" && !isValid(new Date(customExpiry));
    const canSubmit = name.trim() !== "" && permissions.length > 0 && !targetsMissing && !expiryInvalid;
    const isMutating = creating || updating;

    function handleTypeChange(next: TokenType) {
        setTokenType(next);

        // The previous scope may not be valid for the new type, e.g. `public` is personal-only
        if (!scopesFor(next).includes(scope)) {
            setScope("all");
        }
    }

    function handleSubmit() {
        if (!canSubmit) {
            return;
        }

        const orgs = scope === "selected" ? scopeOrgs : [];
        const repos = scope === "selected" ? scopeRepos : [];

        if (token) {
            updateToken({ name: name.trim(), permissions, scope, scopeOrgs: orgs, scopeRepos: repos });
            return;
        }

        createToken({
            name: name.trim(),
            tokenType,
            scope,
            permissions,
            expiresAt: expiresAtFrom(expiryPreset, customExpiry),
            ...(owner.kind === "org" ? { ownerOrg: owner.id } : {}),
            ...(owner.kind === "repo" ? { ownerRepo: owner.id } : {}),
            scopeOrgs: orgs,
            scopeRepos: repos,
        });
    }

    return (
        <div className="space-y-6">
            <div>
                <Label htmlFor="token-name" className="mb-1.5">
                    Name
                </Label>
                <Input
                    id="token-name"
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                    placeholder="e.g. CI deployment"
                    className="bg-card"
                />
            </div>

            {!token && availableTypes.length > 1 && (
                <div>
                    <Label className="mb-1.5">Type</Label>
                    <Select value={tokenType} onValueChange={(value) => handleTypeChange(value as TokenType)}>
                        <SelectTrigger className="w-full bg-card">
                            <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                            {availableTypes.map((type) => (
                                <SelectItem key={type} value={type}>
                                    {tokenTypeLabels[type]}
                                </SelectItem>
                            ))}
                        </SelectContent>
                    </Select>
                </div>
            )}

            {!token && (
                <div>
                    <Label className="mb-1.5">Expiration</Label>
                    <div className="flex gap-3">
                        <Select value={expiryPreset} onValueChange={setExpiryPreset}>
                            <SelectTrigger className="w-48 bg-card">
                                <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                                {expiryPresets.map((preset) => (
                                    <SelectItem key={preset.value} value={preset.value}>
                                        {preset.label}
                                    </SelectItem>
                                ))}
                            </SelectContent>
                        </Select>
                        {expiryPreset === "custom" && (
                            <Input
                                type="date"
                                value={customExpiry}
                                min={format(addDays(new Date(), 1), "yyyy-MM-dd")}
                                onChange={(e) => setCustomExpiry(e.target.value)}
                                className="w-44 bg-card"
                            />
                        )}
                    </div>
                    {expiryPreset === "never" && (
                        <p className="text-xs text-muted-foreground mt-1.5">A token that never expires stays valid until it is revoked.</p>
                    )}
                </div>
            )}

            {availableScopes.length > 1 && (
                <div>
                    <Label className="mb-1.5">Scope</Label>
                    <Select value={scope} onValueChange={(value) => setScope(value as TokenScope)}>
                        <SelectTrigger className="w-full bg-card">
                            <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                            {availableScopes.map((entry) => (
                                <SelectItem key={entry} value={entry}>
                                    {tokenScopeLabels[entry]}
                                </SelectItem>
                            ))}
                        </SelectContent>
                    </Select>
                </div>
            )}

            {scope === "selected" && (
                <ScopeTargetPicker
                    owner={owner}
                    scopeOrgs={scopeOrgs}
                    scopeRepos={scopeRepos}
                    onScopeOrgsChange={setScopeOrgs}
                    onScopeReposChange={setScopeRepos}
                />
            )}

            <div>
                <Label className="mb-1.5">Permissions</Label>
                <p className="text-xs text-muted-foreground mb-2.5">Grant only what the token needs. {permissions.length} selected.</p>
                <TokenPermissionPicker selected={permissions} onChange={setPermissions} />
            </div>

            <div className="flex items-center gap-3">
                <button
                    onClick={handleSubmit}
                    disabled={!canSubmit || isMutating}
                    className="inline-flex items-center gap-2 px-4 h-9 bg-foreground text-background text-sm font-medium rounded-md hover:opacity-90 transition-opacity disabled:opacity-50"
                >
                    {isMutating && <Loader2 className="h-4 w-4 animate-spin" />}
                    {token ? "Save changes" : "Generate token"}
                </button>
                <button
                    onClick={onCancel}
                    className="inline-flex items-center px-4 h-9 text-sm border border-border rounded-md hover:bg-accent/50 transition-colors"
                >
                    Cancel
                </button>
            </div>
        </div>
    );
}

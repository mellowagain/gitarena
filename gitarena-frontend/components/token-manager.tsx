"use client";

import { useState } from "react";
import useSWR, { mutate } from "swr";
import useSWRMutation from "swr/mutation";
import { formatDistanceToNow } from "date-fns";
import { AlertCircle, Check, Copy, KeyRound, Pencil, Plus, Trash2 } from "lucide-react";
import { toast } from "sonner";
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
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { EmptyState } from "@/components/ui/empty-state";
import { TokenForm } from "@/components/token-form";
import { deleteFetcher } from "@/lib/fetchers";
import { uuidToDate } from "@/lib/utils";
import {
    CreateTokenResponse,
    TokenOwner,
    TokenResponse,
    tokenListKey,
    tokenScopeLabels,
    tokenTypeLabels,
    tokenTypesFor,
} from "@/lib/tokens";

/** The secret is returned only once, right after creation, so it stays on screen until dismissed. */
function SecretPanel({ result, onDismiss }: { result: CreateTokenResponse; onDismiss: () => void }) {
    const [copied, setCopied] = useState(false);

    async function copy() {
        try {
            await navigator.clipboard.writeText(result.secret);
        } catch {
            toast.error("Could not copy the token. Select it and copy it manually before dismissing this panel.");
            return;
        }

        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
    }

    return (
        <div className="border border-border rounded-md overflow-hidden mb-6">
            <div className="px-4 py-3 border-b border-border bg-secondary/50">
                <p className="text-sm font-medium">Token &ldquo;{result.token.name}&rdquo; created</p>
                <p className="text-xs text-muted-foreground mt-0.5">
                    Copy it now — this is the only time it will be shown. It cannot be retrieved again.
                </p>
            </div>
            <div className="px-4 py-4 space-y-3">
                <div className="flex items-center gap-2">
                    <code className="flex-1 font-mono text-xs bg-card border border-border rounded-md px-3 py-2 break-all">
                        {result.secret}
                    </code>
                    <button
                        onClick={copy}
                        className="inline-flex items-center gap-1.5 px-3 h-9 text-sm border border-border rounded-md hover:bg-accent/50 transition-colors shrink-0"
                    >
                        {copied ? <Check className="h-3.5 w-3.5 text-green-500" /> : <Copy className="h-3.5 w-3.5" />}
                        {copied ? "Copied" : "Copy"}
                    </button>
                </div>
                <button
                    onClick={onDismiss}
                    className="text-xs text-muted-foreground hover:text-foreground transition-colors underline underline-offset-2"
                >
                    I have saved it
                </button>
            </div>
        </div>
    );
}

function TokenRow({
    token,
    showType,
    onEdit,
    onRevoke,
}: {
    token: TokenResponse;
    showType: boolean;
    onEdit: () => void;
    onRevoke: () => void;
}) {
    const revoked = token.revokedAt !== null;
    const expired = token.expiresAt !== null && new Date(token.expiresAt) < new Date();

    return (
        <div className="flex items-start gap-3 px-4 py-4">
            <KeyRound className="h-4 w-4 text-muted-foreground shrink-0 mt-0.5" />

            <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2 flex-wrap mb-1.5">
                    <span className="text-sm font-medium">{token.name}</span>
                    {showType && <Badge variant="secondary">{tokenTypeLabels[token.tokenType]}</Badge>}
                    <Badge variant="secondary">{tokenScopeLabels[token.scope]}</Badge>
                    {revoked && <Badge variant="destructive">Revoked</Badge>}
                    {!revoked && expired && <Badge variant="destructive">Expired</Badge>}
                </div>

                <Tooltip>
                    <TooltipTrigger asChild>
                        <p className="text-xs text-muted-foreground mb-0.5 w-fit cursor-default underline decoration-dashed underline-offset-2">
                            {token.permissions.length} {token.permissions.length === 1 ? "permission" : "permissions"}
                        </p>
                    </TooltipTrigger>
                    <TooltipContent className="max-w-xs">
                        <span className="font-mono text-xs">{token.permissions.join(", ")}</span>
                    </TooltipContent>
                </Tooltip>

                <p className="text-xs text-muted-foreground">
                    Created {formatDistanceToNow(uuidToDate(token.id), { addSuffix: true })}
                    {" · "}
                    {token.lastUsedAt ? `Last used ${formatDistanceToNow(new Date(token.lastUsedAt), { addSuffix: true })}` : "Never used"}
                    {token.expiresAt && ` · ${expired ? "Expired" : "Expires"} ${new Date(token.expiresAt).toLocaleDateString()}`}
                </p>
            </div>

            {!revoked && (
                <div className="flex items-center gap-1 shrink-0">
                    <button
                        onClick={onEdit}
                        className="p-1.5 text-muted-foreground hover:text-foreground transition-colors"
                        aria-label="Edit token"
                    >
                        <Pencil className="h-4 w-4" />
                    </button>
                    <button
                        onClick={onRevoke}
                        className="p-1.5 text-muted-foreground hover:text-destructive transition-colors"
                        aria-label="Revoke token"
                    >
                        <Trash2 className="h-4 w-4" />
                    </button>
                </div>
            )}
        </div>
    );
}

interface TokenManagerProps {
    owner: TokenOwner;
    title: string;
    description: string;
}

/** Lists and manages the API tokens of a single owner: a user, an organization, a repository or the instance itself. */
export function TokenManager({ owner, title, description }: TokenManagerProps) {
    const [includeRevoked, setIncludeRevoked] = useState(false);
    const [creating, setCreating] = useState(false);
    const [editing, setEditing] = useState<TokenResponse | null>(null);
    const [created, setCreated] = useState<CreateTokenResponse | null>(null);
    const [revoking, setRevoking] = useState<TokenResponse | null>(null);

    const listKey = tokenListKey(owner, includeRevoked);
    const { data: tokens, isLoading, error } = useSWR<TokenResponse[]>(listKey);

    const { trigger: revokeToken } = useSWRMutation(
        listKey,
        (_url: string, { arg }: { arg: string }) => deleteFetcher(`/api/tokens/${arg}`),
        {
            onSuccess: () => {
                mutate(listKey);
                toast.success("Token revoked");
            },
            onError: (err: Error) => toast.error(err.message),
        }
    );

    const showType = tokenTypesFor(owner).length > 1;

    if (creating || editing) {
        return (
            <div>
                <div className="mb-6">
                    <h2 className="text-lg font-semibold">{editing ? "Edit token" : "New token"}</h2>
                    <p className="text-sm text-muted-foreground mt-0.5 leading-relaxed">
                        {editing
                            ? "Permissions and scope take effect immediately. The secret itself stays the same."
                            : "The secret is shown once after creation and cannot be retrieved later."}
                    </p>
                </div>

                <TokenForm
                    owner={owner}
                    token={editing}
                    onCancel={() => {
                        setCreating(false);
                        setEditing(null);
                    }}
                    onCreated={(result) => {
                        setCreating(false);
                        setCreated(result);
                        mutate(listKey);
                    }}
                    onUpdated={() => {
                        setEditing(null);
                        mutate(listKey);
                        toast.success("Token updated");
                    }}
                />
            </div>
        );
    }

    return (
        <div>
            <div className="flex items-start justify-between gap-4 mb-6">
                <div>
                    <h2 className="text-lg font-semibold">{title}</h2>
                    <p className="text-sm text-muted-foreground mt-0.5 leading-relaxed">{description}</p>
                </div>
                <button
                    onClick={() => setCreating(true)}
                    className="inline-flex items-center gap-2 px-4 h-9 bg-foreground text-background text-sm font-medium rounded-md hover:opacity-90 transition-opacity shrink-0"
                >
                    <Plus className="h-4 w-4" />
                    New token
                </button>
            </div>

            {created && <SecretPanel result={created} onDismiss={() => setCreated(null)} />}

            <label className="flex items-center gap-2.5 mb-4 w-fit cursor-pointer">
                <Switch checked={includeRevoked} onCheckedChange={setIncludeRevoked} />
                <span className="text-sm text-muted-foreground">Show revoked tokens</span>
            </label>

            <div className="border border-border rounded-md overflow-hidden mb-6 divide-y divide-border">
                {isLoading &&
                    [0, 1, 2].map((i) => (
                        <div key={i} className="flex items-start gap-3 px-4 py-4">
                            <Skeleton className="h-4 w-4 shrink-0 mt-0.5" />
                            <div className="flex-1 min-w-0 space-y-2">
                                <div className="flex items-center gap-2">
                                    <Skeleton className="h-4 w-32" />
                                    <Skeleton className="h-4 w-20" />
                                </div>
                                <Skeleton className="h-3 w-24" />
                                <Skeleton className="h-3 w-56" />
                            </div>
                            <Skeleton className="h-4 w-12 shrink-0" />
                        </div>
                    ))}

                {!isLoading && error && <div className="px-4 py-8 text-center text-sm text-muted-foreground">Failed to load tokens.</div>}

                {!isLoading && !error && tokens && tokens.length === 0 && (
                    <EmptyState
                        icon={KeyRound}
                        title="No tokens yet"
                        hint="Tokens authenticate API requests and Git operations over HTTP."
                        className="py-12"
                    />
                )}

                {!isLoading &&
                    !error &&
                    tokens?.map((token) => (
                        <TokenRow
                            key={token.id}
                            token={token}
                            showType={showType}
                            onEdit={() => setEditing(token)}
                            onRevoke={() => setRevoking(token)}
                        />
                    ))}
            </div>

            <div className="flex items-start gap-3 p-4 border border-amber-500/30 bg-amber-500/5 rounded-md">
                <AlertCircle className="h-4 w-4 text-amber-500 shrink-0 mt-0.5" />
                <p className="text-xs text-muted-foreground leading-relaxed">
                    Treat tokens like passwords. Do not share them or include them in version-controlled code.
                </p>
            </div>

            <AlertDialog open={revoking !== null} onOpenChange={(open) => !open && setRevoking(null)}>
                <AlertDialogContent>
                    <AlertDialogHeader>
                        <AlertDialogTitle>Revoke this token?</AlertDialogTitle>
                        <AlertDialogDescription>
                            Anything still authenticating with &ldquo;{revoking?.name}&rdquo; will stop working immediately. This cannot be
                            undone.
                        </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel>Cancel</AlertDialogCancel>
                        <AlertDialogAction
                            onClick={() => {
                                if (revoking) {
                                    revokeToken(revoking.id);
                                }
                                setRevoking(null);
                            }}
                            className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
                        >
                            Revoke token
                        </AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>
        </div>
    );
}

"use client";

import { useState } from "react";
import {
    User,
    Mail,
    KeyRound,
    Monitor,
    Key,
    GitBranch,
    Code2,
    Plus,
    Trash2,
    X,
    Eye,
    EyeOff,
    Smartphone,
    LogOut,
    AlertCircle,
    Loader2,
    Info,
    Palette,
} from "lucide-react";
import { TopBar } from "@/components/top-bar";
import useSWR, { mutate } from "swr";
import useSWRMutation from "swr/mutation";
import { postEmptyFetcher, postJsonVoidFetcher, postJsonFetcher, patchJsonFetcher, putJsonFetcher, deleteFetcher } from "@/lib/fetchers";
import { useAuth } from "@/hooks/use-auth";
import { useLocalStorage } from "@/hooks/use-local-storage";
import { Switch } from "@/components/ui/switch";
import { Tooltip, TooltipTrigger, TooltipContent } from "@/components/ui/tooltip";
import { startRegistration } from "@simplewebauthn/browser";
import type { PublicKeyCredentialCreationOptionsJSON } from "@simplewebauthn/browser";
import { formatDistanceToNow } from "date-fns";
import { toast } from "sonner";
import DeviceDetector from "device-detector-js";

// ── Types ──────────────────────────────────────────────────────────────────────

interface EmailResponse {
    id: number;
    email: string;
    primary: boolean;
    commit: boolean;
    notification: boolean;
    public: boolean;
    createdAt: string;
    verifiedAt: string | null;
}

interface SessionResponse {
    hash: string;
    ipAddress: string;
    userAgent: string;
    city: string | null;
    country: string | null;
    isCurrent: boolean;
    updatedAt: string;
}

interface SshKeyResponse {
    id: number;
    title: string;
    fingerprint: string;
    algorithm: string;
    pubkey: string;
    createdAt: string;
    expiresAt: string | null;
}

interface PasskeyItem {
    id: string;
    name: string;
}

interface RegisterStartOptions {
    publicKey: PublicKeyCredentialCreationOptionsJSON;
}

interface RegisterStartResponse {
    challenge_id: string;
    options: RegisterStartOptions;
}

const PASSKEYS_KEY = "/api/auth/passkey";
const EMAILS_KEY = "/api/emails";
const SESSIONS_KEY = "/api/sessions";
const SSH_KEYS_KEY = "/api/ssh-keys";

const deviceDetector = new DeviceDetector();

function uuidv7ToDate(uuid: string): Date {
    const ms = parseInt(uuid.replace(/-/g, "").slice(0, 12), 16);
    return new Date(ms);
}

function parseDevice(userAgent: string): { label: string; isMobile: boolean } {
    try {
        const result = deviceDetector.parse(userAgent);
        const client = result.client?.name ?? "Unknown browser";
        const os = (result.os?.name ?? "Unknown OS").replaceAll("GNU/Linux", "Linux");
        const isMobile = result.device?.type === "smartphone" || result.device?.type === "tablet" || result.device?.type === "phablet";
        return { label: `${client} on ${os}`, isMobile };
    } catch {
        return { label: userAgent.slice(0, 60), isMobile: false };
    }
}

type Tab = "profile" | "emails" | "authentication" | "sessions" | "keys" | "repositories" | "api-keys" | "appearance";

// ── Nav items ──────────────────────────────────────────────────────────────────

const navItems: { id: Tab; label: string; icon: React.ElementType }[] = [
    { id: "profile", label: "Profile", icon: User },
    { id: "appearance", label: "Appearance", icon: Palette },
    { id: "emails", label: "Emails", icon: Mail },
    { id: "authentication", label: "Authentication", icon: KeyRound },
    { id: "sessions", label: "Sessions", icon: Monitor },
    { id: "keys", label: "SSH & GPG Keys", icon: Key },
    { id: "repositories", label: "Repository Settings", icon: GitBranch },
    { id: "api-keys", label: "API Keys", icon: Code2 },
];

// ── Small helpers ──────────────────────────────────────────────────────────────

function SectionHeader({ title, description }: { title: string; description?: string }) {
    return (
        <div className="mb-6">
            <h2 className="text-lg font-semibold">{title}</h2>
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

function Textarea({ className = "", ...props }: React.TextareaHTMLAttributes<HTMLTextAreaElement>) {
    return (
        <textarea
            {...props}
            className={`w-full px-3 py-2 bg-card border border-border rounded-md text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring resize-none transition-shadow ${className}`}
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
            className="inline-flex items-center gap-2 px-3 h-8 text-sm border border-destructive/50 text-destructive rounded-md hover:bg-destructive/10 transition-colors shrink-0 whitespace-nowrap"
        >
            {children}
        </button>
    );
}

function Tag({ children }: { children: React.ReactNode }) {
    return (
        <span className="inline-flex items-center px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wider border border-border rounded bg-secondary text-muted-foreground">
            {children}
        </span>
    );
}

function WipTag() {
    return (
        <span className="inline-flex items-center px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wider border border-amber-500/40 rounded bg-amber-500/10 text-amber-600 dark:text-amber-400">
            WIP
        </span>
    );
}

function Divider() {
    return <div className="border-t border-border my-8" />;
}

// ── Tab panels ─────────────────────────────────────────────────────────────────

function ProfileTab() {
    const { user: me, isLoading } = useAuth();

    if (isLoading) {
        return (
            <div className="space-y-0">
                <SectionHeader title="Profile" description="This information will be visible to other users on GitArena." />
                <div className="mb-6">
                    <FieldLabel>Avatar</FieldLabel>
                    <div className="flex items-center gap-4">
                        <div className="h-16 w-16 rounded-full bg-muted animate-pulse" />
                        <div className="flex flex-col gap-2">
                            <div className="h-8 w-28 bg-muted animate-pulse rounded-md" />
                        </div>
                    </div>
                </div>
                {[180, 120, 160, 140].map((w, i) => (
                    <div key={i} className="mb-6">
                        <div className="h-3 w-16 bg-muted animate-pulse rounded mb-2" />
                        <div className={`h-9 bg-muted animate-pulse rounded-md`} style={{ width: `${w}px` }} />
                    </div>
                ))}
            </div>
        );
    }

    const username = me?.username ?? "";
    const initial = username ? username[0].toUpperCase() : "?";

    return (
        <div className="space-y-0">
            <SectionHeader title="Profile" description="This information will be visible to other users on GitArena." />

            {/* Avatar */}
            <div className="mb-6">
                <FieldLabel>Avatar</FieldLabel>
                <div className="flex items-center gap-4">
                    <div className="h-16 w-16 rounded-full bg-secondary border border-border flex items-center justify-center text-xl font-semibold">
                        {initial}
                    </div>
                    <div className="flex flex-col gap-2">
                        <button className="inline-flex items-center gap-2 px-3 h-8 text-sm border border-border rounded-md hover:bg-accent/50 transition-colors">
                            <Plus className="h-3.5 w-3.5" />
                            Upload avatar
                        </button>
                        <p className="text-xs text-muted-foreground">JPG, PNG or GIF. Max 1 MB.</p>
                    </div>
                </div>
            </div>

            <Divider />

            {/* Username (read-only) */}
            <div className="mb-4">
                <FieldLabel>Username</FieldLabel>
                <Input value={username} readOnly className="opacity-60 cursor-not-allowed" />
                <p className="text-xs text-muted-foreground mt-1.5">Username changes are not yet supported.</p>
            </div>

            {/* Bio / website / location — WIP */}
            <div className="mb-4">
                <div className="flex items-center gap-2 mb-1.5">
                    <FieldLabel optional>Bio</FieldLabel>
                    <WipTag />
                </div>
                <Textarea rows={3} disabled placeholder="Bio editing coming soon" className="opacity-50 cursor-not-allowed" />
            </div>

            <div className="grid grid-cols-2 gap-4 mb-6">
                <div>
                    <div className="flex items-center gap-2 mb-1.5">
                        <FieldLabel optional>Website</FieldLabel>
                        <WipTag />
                    </div>
                    <Input disabled placeholder="Coming soon" type="url" className="opacity-50 cursor-not-allowed" />
                </div>
                <div>
                    <div className="flex items-center gap-2 mb-1.5">
                        <FieldLabel optional>Location</FieldLabel>
                        <WipTag />
                    </div>
                    <Input disabled placeholder="Coming soon" className="opacity-50 cursor-not-allowed" />
                </div>
            </div>

            <Divider />

            {/* Danger zone */}
            <div className="border border-destructive/30 rounded-md overflow-hidden">
                <div className="px-4 py-3 border-b border-destructive/20 bg-destructive/5">
                    <h3 className="text-sm font-medium text-destructive">Danger zone</h3>
                </div>
                <div className="px-4 py-4 flex items-center justify-between gap-4">
                    <div>
                        <p className="text-sm font-medium">Delete account</p>
                        <p className="text-xs text-muted-foreground mt-0.5">
                            Permanently delete your account and all associated data. This cannot be undone.
                        </p>
                    </div>
                    <DangerButton>
                        <Trash2 className="h-3.5 w-3.5" />
                        Delete account
                    </DangerButton>
                </div>
            </div>
        </div>
    );
}

function EmailsTab() {
    const { data: emails, isLoading } = useSWR<EmailResponse[]>(EMAILS_KEY);
    const [newEmail, setNewEmail] = useState("");

    const { trigger: addEmail, isMutating: adding } = useSWRMutation(EMAILS_KEY, postJsonFetcher<{ email: string }, EmailResponse>, {
        onSuccess: () => {
            setNewEmail("");
            mutate(EMAILS_KEY);
            toast.success("Email added. A verification email will be sent if SMTP is configured.");
        },
        onError: (err: Error) => toast.error(err.message),
    });

    const { trigger: deleteEmail } = useSWRMutation(
        EMAILS_KEY,
        (_url: string, { arg }: { arg: number }) => deleteFetcher(`/api/emails/${arg}`),
        {
            onSuccess: () => mutate(EMAILS_KEY),
            onError: (err: Error) => toast.error(err.message),
        }
    );

    const { trigger: patchEmail } = useSWRMutation(
        EMAILS_KEY,
        (_url: string, { arg }: { arg: { id: number; patch: { primary?: boolean; notification?: boolean; public?: boolean } } }) =>
            patchJsonFetcher<typeof arg.patch, EmailResponse>(`/api/emails/${arg.id}`, { arg: arg.patch }),
        {
            onSuccess: () => mutate(EMAILS_KEY),
            onError: (err: Error) => toast.error(err.message),
        }
    );

    const { trigger: resendVerify } = useSWRMutation(
        EMAILS_KEY,
        (_url: string, { arg }: { arg: number }) => postJsonVoidFetcher<Record<string, never>>(`/api/emails/${arg}/verify`, { arg: {} }),
        {
            onSuccess: () => toast.success("Verification email sent"),
            onError: (err: Error) => toast.error(err.message),
        }
    );

    return (
        <div>
            <SectionHeader title="Emails" description="Manage email addresses associated with your account." />

            <div className="border border-border rounded-md overflow-hidden mb-6">
                {isLoading &&
                    [0, 1, 2].map((i) => (
                        <div key={i} className={`flex items-start gap-3 px-4 py-4 ${i > 0 ? "border-t border-border" : ""}`}>
                            <div className="mt-1 h-2 w-2 rounded-full bg-muted animate-pulse shrink-0" />
                            <div className="flex-1 min-w-0 space-y-2">
                                <div className="flex items-center gap-2">
                                    <div className="h-4 w-48 bg-muted animate-pulse rounded" />
                                    <div className="h-4 w-14 bg-muted animate-pulse rounded" />
                                </div>
                                <div className="h-3 w-32 bg-muted animate-pulse rounded" />
                            </div>
                            <div className="h-7 w-16 bg-muted animate-pulse rounded shrink-0" />
                        </div>
                    ))}
                {!isLoading && emails && emails.length === 0 && (
                    <div className="px-4 py-8 text-center text-sm text-muted-foreground">No email addresses added.</div>
                )}
                {!isLoading &&
                    emails &&
                    emails.map((email, i) => {
                        const verified = email.verifiedAt !== null;
                        return (
                            <div key={email.id} className={`flex items-start gap-3 px-4 py-4 ${i > 0 ? "border-t border-border" : ""}`}>
                                {/* Status dot */}
                                <div
                                    className={`mt-1 h-2 w-2 rounded-full shrink-0 ${verified ? "bg-green-500" : "bg-muted-foreground/40"}`}
                                />

                                <div className="flex-1 min-w-0">
                                    <div className="flex items-center gap-2 flex-wrap mb-1.5">
                                        <span className="text-sm font-medium">{email.email}</span>
                                        {email.primary && <Tag>Primary</Tag>}
                                        {verified ? <Tag>Verified</Tag> : <Tag>Unverified</Tag>}
                                        {email.notification && <Tag>Notifications</Tag>}
                                        {email.public && <Tag>Public</Tag>}
                                    </div>
                                    <div className="flex items-center gap-2 flex-wrap">
                                        {!email.primary && verified && (
                                            <button
                                                onClick={() => patchEmail({ id: email.id, patch: { primary: true } })}
                                                className="text-xs text-muted-foreground hover:text-foreground transition-colors underline underline-offset-2"
                                            >
                                                Make primary
                                            </button>
                                        )}
                                        {!verified && (
                                            <button
                                                onClick={() => resendVerify(email.id)}
                                                className="text-xs text-muted-foreground hover:text-foreground transition-colors underline underline-offset-2"
                                            >
                                                Resend verification
                                            </button>
                                        )}
                                        <button
                                            onClick={() => patchEmail({ id: email.id, patch: { notification: !email.notification } })}
                                            className="text-xs text-muted-foreground hover:text-foreground transition-colors underline underline-offset-2"
                                        >
                                            {email.notification ? "Disable notifications" : "Enable notifications"}
                                        </button>
                                        <button
                                            onClick={() => patchEmail({ id: email.id, patch: { public: !email.public } })}
                                            className="text-xs text-muted-foreground hover:text-foreground transition-colors underline underline-offset-2"
                                        >
                                            {email.public ? "Make private" : "Make public"}
                                        </button>
                                    </div>
                                </div>

                                {!email.primary && (
                                    <button
                                        onClick={() => deleteEmail(email.id)}
                                        className="text-muted-foreground hover:text-destructive transition-colors mt-0.5"
                                        title="Remove email"
                                    >
                                        <X className="h-4 w-4" />
                                    </button>
                                )}
                            </div>
                        );
                    })}
            </div>

            {/* Add email */}
            <div>
                <FieldLabel>Add email address</FieldLabel>
                <div className="flex gap-2">
                    <Input
                        value={newEmail}
                        onChange={(e) => setNewEmail(e.target.value)}
                        type="email"
                        placeholder="you@example.com"
                        className="flex-1"
                        onKeyDown={(e) => {
                            if (e.key === "Enter" && newEmail) {
                                addEmail({ email: newEmail });
                            }
                        }}
                    />
                    <button
                        onClick={() => {
                            if (newEmail) {
                                addEmail({ email: newEmail });
                            }
                        }}
                        disabled={adding || !newEmail}
                        className="inline-flex items-center gap-2 px-4 h-9 bg-foreground text-background text-sm font-medium rounded-md hover:opacity-90 transition-opacity shrink-0 disabled:opacity-50"
                    >
                        {adding ? <Loader2 className="h-4 w-4 animate-spin" /> : <Plus className="h-4 w-4" />}
                        Add
                    </button>
                </div>
                <p className="text-xs text-muted-foreground mt-1.5">A verification email will be sent to this address.</p>
            </div>
        </div>
    );
}

function PasskeyRow({ item: pk, index }: { item: PasskeyItem; index: number }) {
    const { trigger: deletePasskey, isMutating: deleting } = useSWRMutation(`/api/auth/passkey/${pk.id}`, deleteFetcher, {
        onSuccess: () => mutate(PASSKEYS_KEY),
    });

    return (
        <div className={`flex items-center gap-3 px-4 py-3 ${index > 0 ? "border-t border-border" : ""}`}>
            <Smartphone className="h-4 w-4 text-muted-foreground shrink-0" />
            <div className="flex-1 min-w-0">
                <p className="text-sm font-medium">{pk.name}</p>
                <p className="text-xs text-muted-foreground">Added {formatDistanceToNow(uuidv7ToDate(pk.id), { addSuffix: true })}</p>
            </div>
            <button
                onClick={() => deletePasskey()}
                disabled={deleting}
                className="text-muted-foreground hover:text-destructive transition-colors disabled:opacity-50"
            >
                {deleting ? <Loader2 className="h-4 w-4 animate-spin" /> : <Trash2 className="h-4 w-4" />}
            </button>
        </div>
    );
}

function AuthenticationTab() {
    const [showCurrent, setShowCurrent] = useState(false);
    const [showNew, setShowNew] = useState(false);
    const [showConfirm, setShowConfirm] = useState(false);

    const { data: passkeys, isLoading: passkeysLoading } = useSWR<PasskeyItem[]>(PASSKEYS_KEY);

    const { trigger: startRegister, isMutating: registering } = useSWRMutation(
        "/api/auth/passkey/register/start",
        postEmptyFetcher<RegisterStartResponse>
    );

    const { trigger: finishRegister } = useSWRMutation(
        "/api/auth/passkey/register/finish",
        postJsonVoidFetcher<{ challenge_id: string; credential: object }>
    );

    async function handleAddPasskey() {
        try {
            const startResult = await startRegister();
            const { challenge_id, options } = startResult!;
            const credential = await startRegistration({ optionsJSON: options.publicKey });
            await finishRegister({ challenge_id, credential });
            await mutate(PASSKEYS_KEY);
        } catch (err) {
            toast.error(err instanceof Error ? err.message : "Failed to register passkey");
        }
    }

    return (
        <div>
            <SectionHeader title="Authentication" />

            {/* Change password — WIP */}
            <div className="flex items-center gap-2 mb-4">
                <h3 className="text-sm font-semibold">Password</h3>
                <WipTag />
            </div>
            <div className="space-y-4 mb-6 opacity-50 pointer-events-none select-none">
                <div>
                    <FieldLabel>Current password</FieldLabel>
                    <div className="relative">
                        <Input type={showCurrent ? "text" : "password"} placeholder="Current password" className="pr-10" />
                        <button
                            onClick={() => setShowCurrent((v) => !v)}
                            className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground transition-colors"
                        >
                            {showCurrent ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                        </button>
                    </div>
                </div>
                <div className="grid grid-cols-2 gap-4">
                    <div>
                        <FieldLabel>New password</FieldLabel>
                        <div className="relative">
                            <Input type={showNew ? "text" : "password"} placeholder="New password" className="pr-10" />
                            <button
                                onClick={() => setShowNew((v) => !v)}
                                className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground transition-colors"
                            >
                                {showNew ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                            </button>
                        </div>
                    </div>
                    <div>
                        <FieldLabel>Confirm new password</FieldLabel>
                        <div className="relative">
                            <Input type={showConfirm ? "text" : "password"} placeholder="Confirm password" className="pr-10" />
                            <button
                                onClick={() => setShowConfirm((v) => !v)}
                                className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground transition-colors"
                            >
                                {showConfirm ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                            </button>
                        </div>
                    </div>
                </div>
            </div>
            <SaveButton>Update password</SaveButton>

            <Divider />

            {/* Passkeys */}
            <div className="flex items-center justify-between mb-4">
                <div>
                    <h3 className="text-sm font-semibold">Passkeys</h3>
                    <p className="text-xs text-muted-foreground mt-0.5">Sign in without a password using biometrics or a hardware key.</p>
                </div>
                <button
                    onClick={handleAddPasskey}
                    disabled={registering}
                    className="inline-flex items-center gap-2 px-3 h-8 text-sm border border-border rounded-md hover:bg-accent/50 transition-colors shrink-0 disabled:opacity-50"
                >
                    {registering ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Plus className="h-3.5 w-3.5" />}
                    Add passkey
                </button>
            </div>

            <div className="border border-border rounded-md overflow-hidden mb-0">
                {passkeysLoading &&
                    [0, 1].map((i) => (
                        <div key={i} className={`flex items-center gap-3 px-4 py-4 ${i > 0 ? "border-t border-border" : ""}`}>
                            <div className="h-4 w-4 bg-muted animate-pulse rounded shrink-0" />
                            <div className="flex-1 min-w-0 space-y-1.5">
                                <div className="h-4 w-40 bg-muted animate-pulse rounded" />
                                <div className="h-3 w-24 bg-muted animate-pulse rounded" />
                            </div>
                            <div className="h-7 w-16 bg-muted animate-pulse rounded shrink-0" />
                        </div>
                    ))}
                {!passkeysLoading &&
                    passkeys &&
                    passkeys.length > 0 &&
                    passkeys.map((pk, i) => <PasskeyRow key={pk.id} item={pk} index={i} />)}
                {!passkeysLoading && (!passkeys || passkeys.length === 0) && (
                    <div className="px-4 py-8 text-center text-sm text-muted-foreground">No passkeys configured.</div>
                )}
            </div>

            <Divider />

            {/* 2FA — WIP */}
            <div className="flex items-start gap-4">
                <div className="flex-1">
                    <div className="flex items-center gap-2 mb-0.5">
                        <h3 className="text-sm font-semibold">Two-factor authentication</h3>
                        <WipTag />
                    </div>
                    <p className="text-xs text-muted-foreground leading-relaxed">
                        Add an extra layer of security to your account. When enabled you will be prompted for a one-time code in addition to
                        your password.
                    </p>
                </div>
                <div className="shrink-0 mt-0.5 opacity-40 pointer-events-none">
                    <button className="relative inline-flex h-5 w-9 items-center rounded-full bg-secondary border border-border">
                        <span className="inline-block h-3.5 w-3.5 transform rounded-full bg-background border border-border/50 translate-x-1" />
                    </button>
                </div>
            </div>
        </div>
    );
}

function SessionsTab() {
    const { data: sessions, isLoading } = useSWR<SessionResponse[]>(SESSIONS_KEY);

    const { trigger: revokeSession, isMutating: revoking } = useSWRMutation(
        SESSIONS_KEY,
        (_url: string, { arg }: { arg: string }) => deleteFetcher(`/api/sessions/${arg}`),
        {
            onSuccess: () => mutate(SESSIONS_KEY),
            onError: (err: Error) => toast.error(err.message),
        }
    );

    const { trigger: revokeAll, isMutating: revokingAll } = useSWRMutation(SESSIONS_KEY, deleteFetcher, {
        onSuccess: () => mutate(SESSIONS_KEY),
        onError: (err: Error) => toast.error(err.message),
    });

    const hasOtherSessions = sessions ? sessions.some((s) => !s.isCurrent) : false;

    return (
        <div>
            <SectionHeader title="Sessions" description="Devices and clients currently signed in to your account." />

            <div className="border border-border rounded-md overflow-hidden mb-6">
                {isLoading &&
                    [0, 1].map((i) => (
                        <div key={i} className={`flex items-start gap-3 px-4 py-4 ${i > 0 ? "border-t border-border" : ""}`}>
                            <div className="h-4 w-4 bg-muted animate-pulse rounded shrink-0 mt-0.5" />
                            <div className="flex-1 min-w-0 space-y-1.5">
                                <div className="h-4 w-36 bg-muted animate-pulse rounded" />
                                <div className="h-3 w-56 bg-muted animate-pulse rounded" />
                            </div>
                        </div>
                    ))}
                {!isLoading && sessions && sessions.length === 0 && (
                    <div className="px-4 py-8 text-center text-sm text-muted-foreground">No active sessions.</div>
                )}
                {!isLoading &&
                    sessions &&
                    sessions.map((session, i) => {
                        const device = parseDevice(session.userAgent);
                        const location = [session.city, session.country].filter(Boolean).join(", ") || "Unknown location";
                        const lastActive = formatDistanceToNow(new Date(session.updatedAt), { addSuffix: true });
                        const DeviceIcon = device.isMobile ? Smartphone : Monitor;

                        return (
                            <div
                                key={session.hash}
                                className={`flex items-start gap-3 px-4 py-4 ${i > 0 ? "border-t border-border" : ""} ${session.isCurrent ? "bg-secondary/40" : ""}`}
                            >
                                <DeviceIcon className="h-4 w-4 text-muted-foreground shrink-0 mt-0.5" />
                                <div className="flex-1 min-w-0">
                                    <div className="flex items-center gap-2 mb-0.5">
                                        <span className="text-sm font-medium">{device.label}</span>
                                        {session.isCurrent && <Tag>Current session</Tag>}
                                    </div>
                                    <p className="text-xs text-muted-foreground">
                                        {session.ipAddress} · {location} · Last active {lastActive}
                                    </p>
                                </div>
                                {!session.isCurrent && (
                                    <button
                                        onClick={() => revokeSession(session.hash)}
                                        disabled={revoking}
                                        className="text-muted-foreground hover:text-destructive transition-colors text-xs flex items-center gap-1 shrink-0 mt-0.5 disabled:opacity-50"
                                    >
                                        <LogOut className="h-3.5 w-3.5" />
                                        Revoke
                                    </button>
                                )}
                            </div>
                        );
                    })}
            </div>

            {hasOtherSessions && (
                <button
                    onClick={() => revokeAll()}
                    disabled={revokingAll}
                    className="inline-flex items-center gap-2 px-4 h-9 border border-destructive/50 text-destructive text-sm font-medium rounded-md hover:bg-destructive/10 transition-colors disabled:opacity-50"
                >
                    {revokingAll ? <Loader2 className="h-4 w-4 animate-spin" /> : <LogOut className="h-4 w-4" />}
                    Revoke all other sessions
                </button>
            )}

            <div className="flex items-start gap-3 p-4 border border-blue-500/30 bg-blue-500/5 rounded-md mt-4">
                <Info className="h-4 w-4 text-blue-500 shrink-0 mt-0.5" />
                <p className="text-xs text-muted-foreground leading-relaxed">
                    Geolocation data provided by{" "}
                    <a href="https://www.maxmind.com" target="_blank" rel="noopener noreferrer" className="underline hover:text-foreground">
                        MaxMind
                    </a>
                    .
                </p>
            </div>
        </div>
    );
}

function KeysTab() {
    const { data: sshKeys, isLoading: sshLoading } = useSWR<SshKeyResponse[]>(SSH_KEYS_KEY);
    const [newSSHTitle, setNewSSHTitle] = useState("");
    const [newSSHKey, setNewSSHKey] = useState("");

    const { trigger: addSSHKey, isMutating: addingKey } = useSWRMutation(
        "/api/ssh-key",
        putJsonFetcher<{ title: string; key: string }, { id: number; fingerprint: string }>,
        {
            onSuccess: () => {
                setNewSSHTitle("");
                setNewSSHKey("");
                mutate(SSH_KEYS_KEY);
                toast.success("SSH key added");
            },
            onError: (err: Error) => toast.error(err.message),
        }
    );

    const { trigger: deleteSSHKey } = useSWRMutation(
        SSH_KEYS_KEY,
        (_url: string, { arg }: { arg: number }) => deleteFetcher(`/api/ssh-keys/${arg}`),
        {
            onSuccess: () => mutate(SSH_KEYS_KEY),
            onError: (err: Error) => toast.error(err.message),
        }
    );

    return (
        <div>
            {/* SSH Keys */}
            <SectionHeader title="SSH Keys" description="SSH keys are used to authenticate your Git operations over SSH." />

            <div className="border border-border rounded-md overflow-hidden mb-4">
                {sshLoading &&
                    [0, 1].map((i) => (
                        <div key={i} className={`flex items-start gap-3 px-4 py-4 ${i > 0 ? "border-t border-border" : ""}`}>
                            <div className="h-4 w-4 bg-muted animate-pulse rounded shrink-0 mt-0.5" />
                            <div className="flex-1 min-w-0 space-y-1.5">
                                <div className="h-4 w-32 bg-muted animate-pulse rounded" />
                                <div className="h-3 w-64 bg-muted animate-pulse rounded font-mono" />
                                <div className="h-3 w-28 bg-muted animate-pulse rounded" />
                            </div>
                            <div className="h-4 w-4 bg-muted animate-pulse rounded shrink-0" />
                        </div>
                    ))}
                {!sshLoading && sshKeys && sshKeys.length === 0 && (
                    <div className="px-4 py-8 text-center text-sm text-muted-foreground">No SSH keys added yet.</div>
                )}
                {!sshLoading &&
                    sshKeys &&
                    sshKeys.map((key, i) => (
                        <div key={key.id} className={`flex items-start gap-3 px-4 py-4 ${i > 0 ? "border-t border-border" : ""}`}>
                            <Key className="h-4 w-4 text-muted-foreground shrink-0 mt-0.5" />
                            <div className="flex-1 min-w-0">
                                <p className="text-sm font-medium mb-0.5">{key.title}</p>
                                <Tooltip>
                                    <TooltipTrigger asChild>
                                        <p className="font-mono text-xs text-muted-foreground truncate mb-0.5 cursor-default">
                                            SHA256:{key.fingerprint}
                                        </p>
                                    </TooltipTrigger>
                                    <TooltipContent>
                                        <span className="font-mono">
                                            {key.algorithm} {key.pubkey}
                                        </span>
                                    </TooltipContent>
                                </Tooltip>
                                <p className="text-xs text-muted-foreground">
                                    Added {formatDistanceToNow(new Date(key.createdAt), { addSuffix: true })}
                                    {key.expiresAt && ` · Expires ${new Date(key.expiresAt).toLocaleDateString()}`}
                                </p>
                            </div>
                            <button
                                onClick={() => deleteSSHKey(key.id)}
                                className="text-muted-foreground hover:text-destructive transition-colors"
                            >
                                <Trash2 className="h-4 w-4" />
                            </button>
                        </div>
                    ))}
            </div>

            {/* Add SSH key */}
            <div className="space-y-4 mb-0">
                <div>
                    <FieldLabel>Title</FieldLabel>
                    <Input value={newSSHTitle} onChange={(e) => setNewSSHTitle(e.target.value)} placeholder="e.g. Personal MacBook" />
                </div>
                <div>
                    <FieldLabel>Key</FieldLabel>
                    <Textarea
                        rows={4}
                        value={newSSHKey}
                        onChange={(e) => setNewSSHKey(e.target.value)}
                        placeholder="Paste your public key here — begins with ssh-rsa, ssh-ed25519, etc."
                        className="font-mono text-xs"
                    />
                </div>
                <button
                    onClick={() => {
                        if (newSSHTitle && newSSHKey) {
                            addSSHKey({ title: newSSHTitle, key: newSSHKey });
                        }
                    }}
                    disabled={addingKey || !newSSHTitle || !newSSHKey}
                    className="inline-flex items-center gap-2 px-4 h-9 bg-foreground text-background text-sm font-medium rounded-md hover:opacity-90 transition-opacity disabled:opacity-50"
                >
                    {addingKey ? <Loader2 className="h-4 w-4 animate-spin" /> : <Plus className="h-4 w-4" />}
                    Add SSH key
                </button>
            </div>

            <Divider />

            {/* GPG Keys — WIP */}
            <div className="flex items-center gap-2 mb-1">
                <h2 className="text-lg font-semibold">GPG Keys</h2>
                <WipTag />
            </div>
            <p className="text-sm text-muted-foreground mb-4">GPG key support is coming soon.</p>
        </div>
    );
}

function RepositoriesTab() {
    return (
        <div>
            <div className="flex items-center gap-2 mb-1">
                <h2 className="text-lg font-semibold">Repository Settings</h2>
                <WipTag />
            </div>
            <p className="text-sm text-muted-foreground">Repository default settings are coming soon.</p>
        </div>
    );
}

function APIKeysTab() {
    return (
        <div>
            <div className="flex items-center gap-2 mb-1">
                <h2 className="text-lg font-semibold">API Keys</h2>
                <WipTag />
            </div>
            <p className="text-sm text-muted-foreground mb-6">API key management is coming soon.</p>

            <div className="flex items-start gap-3 p-4 border border-amber-500/30 bg-amber-500/5 rounded-md">
                <AlertCircle className="h-4 w-4 text-amber-500 shrink-0 mt-0.5" />
                <p className="text-xs text-muted-foreground leading-relaxed">
                    Treat your API keys like passwords. Do not share them or include them in version-controlled code.
                </p>
            </div>
        </div>
    );
}

function AppearanceTab() {
    const [allLanguages, setAllLanguages] = useLocalStorage<boolean>("gitarena:all-languages", false);

    return (
        <div>
            <SectionHeader title="Appearance" description="Customize how GitArena looks and behaves for you." />

            <div className="space-y-6">
                <div className="flex items-center justify-between gap-4">
                    <div>
                        <p className="text-sm font-medium">All languages</p>
                        <p className="text-xs text-muted-foreground mt-0.5 leading-relaxed">
                            Show all detected languages in repository language bars, including data and prose files. When off, only
                            programming and markup languages are shown, with everything else grouped into &quot;Other&quot;.
                        </p>
                    </div>
                    <Switch checked={allLanguages} onCheckedChange={setAllLanguages} />
                </div>
            </div>
        </div>
    );
}

// ── Page ───────────────────────────────────────────────────────────────────────

export default function SettingsPage() {
    const [activeTab, setActiveTab] = useState<Tab>("profile");
    const { user: me } = useAuth();

    const username = me?.username ?? "Settings";

    const tabContent: Record<Tab, React.ReactNode> = {
        profile: <ProfileTab />,
        appearance: <AppearanceTab />,
        emails: <EmailsTab />,
        authentication: <AuthenticationTab />,
        sessions: <SessionsTab />,
        keys: <KeysTab />,
        repositories: <RepositoriesTab />,
        "api-keys": <APIKeysTab />,
    };

    return (
        <div className="flex flex-col h-screen overflow-hidden bg-background text-foreground font-sans">
            <TopBar breadcrumb={[{ label: username, href: `/${username}` }, { label: "Settings" }]} hasNotifications />

            <div className="flex-1 flex overflow-hidden">
                {/* Left sidebar nav */}
                <aside className="w-60 border-r border-border shrink-0 overflow-y-auto">
                    <div className="p-3">
                        <h3 className="px-3 mb-2 mt-2 text-xs font-medium uppercase tracking-widest text-muted-foreground">Settings</h3>
                        <nav className="space-y-0.5">
                            {navItems.map((item) => (
                                <button
                                    key={item.id}
                                    onClick={() => setActiveTab(item.id)}
                                    className={`w-full flex items-center gap-2.5 px-3 py-2 text-sm rounded-md transition-colors text-left ${
                                        activeTab === item.id
                                            ? "bg-accent text-foreground"
                                            : "text-muted-foreground hover:text-foreground hover:bg-accent/50"
                                    }`}
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
                    <div className="max-w-2xl mx-auto px-8 py-8">{tabContent[activeTab]}</div>
                </main>
            </div>
        </div>
    );
}

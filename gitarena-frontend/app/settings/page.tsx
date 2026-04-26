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
    Check,
    X,
    Eye,
    EyeOff,
    Copy,
    ShieldCheck,
    Smartphone,
    LogOut,
    Globe,
    Lock,
    AlertCircle,
    Loader2,
} from "lucide-react";
import { TopBar } from "@/components/top-bar";
import useSWR, { mutate } from "swr";
import useSWRMutation from "swr/mutation";
import { postEmptyFetcher, postJsonVoidFetcher, deleteFetcher } from "@/lib/fetchers";
import { startRegistration } from "@simplewebauthn/browser";
import type { PublicKeyCredentialCreationOptionsJSON } from "@simplewebauthn/browser";
import { formatDistanceToNow } from "date-fns";
import { toast } from "sonner";

// ── Types ──────────────────────────────────────────────────────────────────────

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

function uuidv7ToDate(uuid: string): Date {
    const ms = parseInt(uuid.replace(/-/g, "").slice(0, 12), 16);
    return new Date(ms);
}

type Tab = "profile" | "emails" | "authentication" | "sessions" | "keys" | "repositories" | "api-keys";

// ── Mock data ──────────────────────────────────────────────────────────────────

const currentUser = {
    username: "mellowagain",
    displayName: "Mari",
    bio: "Building open-source tools for developers. Creator of GitArena and pastemyst.",
    website: "https://mariari.dev",
    location: "Vienna, Austria",
    avatarUrl: null as string | null,
};

const mockEmails = [
    { id: "1", address: "mari@mariari.dev", primary: true, verified: true, notification: true, publiclyVisible: false },
    { id: "2", address: "mari@gitarena.dev", primary: false, verified: true, notification: false, publiclyVisible: true },
    { id: "3", address: "noreply@example.com", primary: false, verified: false, notification: false, publiclyVisible: false },
];

const mockSessions = [
    { id: "1", device: "Chrome on macOS", ip: "91.115.23.4", location: "Vienna, Austria", lastActive: "Active now", current: true },
    { id: "2", device: "Firefox on Windows", ip: "185.220.101.12", location: "Berlin, Germany", lastActive: "2 hours ago", current: false },
    { id: "3", device: "Safari on iOS", ip: "91.115.23.4", location: "Vienna, Austria", lastActive: "1 day ago", current: false },
    { id: "4", device: "VS Code Extension", ip: "91.115.23.4", location: "Vienna, Austria", lastActive: "3 days ago", current: false },
];

const mockSSHKeys = [
    {
        id: "1",
        title: "Personal MacBook",
        fingerprint: "SHA256:AbCdEfGhIjKlMnOpQrStUvWxYz1234567890ABCD",
        addedAt: "6 months ago",
        lastUsed: "1 hour ago",
    },
    {
        id: "2",
        title: "Work Desktop",
        fingerprint: "SHA256:ZyXwVuTsRqPoNmLkJiHgFeDcBa0987654321ZYXW",
        addedAt: "2 months ago",
        lastUsed: "5 days ago",
    },
];

const mockGPGKeys = [
    { id: "1", keyId: "A1B2C3D4E5F60001", name: "Mari <mari@mariari.dev>", createdAt: "1 year ago", expiresAt: "2026-01-01", subkeys: 2 },
];

const mockAPIKeys = [
    {
        id: "1",
        name: "CI/CD Pipeline",
        scopes: ["repo:read", "repo:write"],
        createdAt: "1 month ago",
        lastUsed: "2 hours ago",
        expiresAt: "2025-12-31",
    },
    { id: "2", name: "VS Code Extension", scopes: ["repo:read"], createdAt: "3 months ago", lastUsed: "1 hour ago", expiresAt: null },
    {
        id: "3",
        name: "Personal Scripts",
        scopes: ["repo:read", "user:read", "issue:write"],
        createdAt: "6 months ago",
        lastUsed: "3 days ago",
        expiresAt: null,
    },
];

// ── Nav items ──────────────────────────────────────────────────────────────────

const navItems: { id: Tab; label: string; icon: React.ElementType }[] = [
    { id: "profile", label: "Profile", icon: User },
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
            className="inline-flex items-center gap-2 px-3 h-8 text-sm border border-destructive/50 text-destructive rounded-md hover:bg-destructive/10 transition-colors"
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

function Divider() {
    return <div className="border-t border-border my-8" />;
}

// ── Tab panels ─────────────────────────────────────────────────────────────────

function ProfileTab() {
    const [bio, setBio] = useState(currentUser.bio);

    return (
        <div className="space-y-0">
            <SectionHeader title="Profile" description="This information will be visible to other users on GitArena." />

            {/* Avatar */}
            <div className="mb-6">
                <FieldLabel>Avatar</FieldLabel>
                <div className="flex items-center gap-4">
                    <div className="h-16 w-16 rounded-full bg-secondary border border-border flex items-center justify-center text-xl font-semibold">
                        {currentUser.displayName[0].toUpperCase()}
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

            {/* Name fields */}
            <div className="grid grid-cols-2 gap-4 mb-4">
                <div>
                    <FieldLabel>Display name</FieldLabel>
                    <Input defaultValue={currentUser.displayName} placeholder="Display name" />
                </div>
                <div>
                    <FieldLabel>Username</FieldLabel>
                    <Input defaultValue={currentUser.username} placeholder="username" />
                    <p className="text-xs text-muted-foreground mt-1.5">Changing your username will also change your profile URL.</p>
                </div>
            </div>

            <div className="mb-4">
                <FieldLabel optional>Bio</FieldLabel>
                <Textarea rows={3} value={bio} onChange={(e) => setBio(e.target.value)} placeholder="Tell others a little about yourself" />
                <p className="text-xs text-muted-foreground mt-1.5">{bio.length} / 255</p>
            </div>

            <div className="grid grid-cols-2 gap-4 mb-6">
                <div>
                    <FieldLabel optional>Website</FieldLabel>
                    <Input defaultValue={currentUser.website} placeholder="https://example.com" type="url" />
                </div>
                <div>
                    <FieldLabel optional>Location</FieldLabel>
                    <Input defaultValue={currentUser.location} placeholder="City, Country" />
                </div>
            </div>

            <SaveButton />

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
    const [emails, setEmails] = useState(mockEmails);
    const [newEmail, setNewEmail] = useState("");

    function removeEmail(id: string) {
        setEmails((prev) => prev.filter((e) => e.id !== id));
    }

    return (
        <div>
            <SectionHeader title="Emails" description="Manage email addresses associated with your account." />

            <div className="border border-border rounded-md overflow-hidden mb-6">
                {emails.map((email, i) => (
                    <div key={email.id} className={`flex items-start gap-3 px-4 py-4 ${i > 0 ? "border-t border-border" : ""}`}>
                        {/* Status dot */}
                        <div
                            className={`mt-1 h-2 w-2 rounded-full shrink-0 ${email.verified ? "bg-green-500" : "bg-muted-foreground/40"}`}
                        />

                        <div className="flex-1 min-w-0">
                            <div className="flex items-center gap-2 flex-wrap mb-1.5">
                                <span className="text-sm font-medium">{email.address}</span>
                                {email.primary && <Tag>Primary</Tag>}
                                {email.verified && <Tag>Verified</Tag>}
                                {!email.verified && <Tag>Unverified</Tag>}
                                {email.notification && <Tag>Notifications</Tag>}
                                {email.publiclyVisible && <Tag>Public</Tag>}
                            </div>
                            <div className="flex items-center gap-2 flex-wrap">
                                {!email.primary && (
                                    <button className="text-xs text-muted-foreground hover:text-foreground transition-colors underline underline-offset-2">
                                        Make primary
                                    </button>
                                )}
                                {!email.verified && (
                                    <button className="text-xs text-muted-foreground hover:text-foreground transition-colors underline underline-offset-2">
                                        Resend verification
                                    </button>
                                )}
                                <button className="text-xs text-muted-foreground hover:text-foreground transition-colors underline underline-offset-2">
                                    {email.notification ? "Disable notifications" : "Enable notifications"}
                                </button>
                                <button className="text-xs text-muted-foreground hover:text-foreground transition-colors underline underline-offset-2">
                                    {email.publiclyVisible ? "Make private" : "Make public"}
                                </button>
                            </div>
                        </div>

                        {!email.primary && (
                            <button
                                onClick={() => removeEmail(email.id)}
                                className="text-muted-foreground hover:text-destructive transition-colors mt-0.5"
                                title="Remove email"
                            >
                                <X className="h-4 w-4" />
                            </button>
                        )}
                    </div>
                ))}
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
                    />
                    <button className="inline-flex items-center gap-2 px-4 h-9 bg-foreground text-background text-sm font-medium rounded-md hover:opacity-90 transition-opacity shrink-0">
                        <Plus className="h-4 w-4" />
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
    const [twoFAEnabled, setTwoFAEnabled] = useState(false);

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

            {/* Change password */}
            <h3 className="text-sm font-semibold mb-4">Password</h3>
            <div className="space-y-4 mb-6">
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
                {passkeysLoading && (
                    <div className="px-4 py-8 flex justify-center">
                        <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
                    </div>
                )}
                {!passkeysLoading &&
                    passkeys &&
                    passkeys.length > 0 &&
                    passkeys.map((pk, i) => <PasskeyRow key={pk.id} item={pk} index={i} />)}
                {!passkeysLoading && (!passkeys || passkeys.length === 0) && (
                    <div className="px-4 py-8 text-center text-sm text-muted-foreground">No passkeys configured.</div>
                )}
            </div>

            <Divider />

            {/* 2FA */}
            <div className="flex items-start gap-4">
                <div className="flex-1">
                    <h3 className="text-sm font-semibold mb-0.5">Two-factor authentication</h3>
                    <p className="text-xs text-muted-foreground leading-relaxed">
                        Add an extra layer of security to your account. When enabled you will be prompted for a one-time code in addition to
                        your password.
                    </p>
                </div>
                <div className="shrink-0 mt-0.5">
                    <button
                        onClick={() => setTwoFAEnabled((v) => !v)}
                        className={`relative inline-flex h-5 w-9 items-center rounded-full transition-colors focus:outline-none ${twoFAEnabled ? "bg-foreground" : "bg-secondary border border-border"}`}
                    >
                        <span
                            className={`inline-block h-3.5 w-3.5 transform rounded-full bg-background transition-transform border border-border/50 ${twoFAEnabled ? "translate-x-4" : "translate-x-1"}`}
                        />
                    </button>
                </div>
            </div>
            {twoFAEnabled && (
                <div className="mt-4 p-4 border border-border rounded-md bg-secondary/40">
                    <div className="flex items-center gap-2 text-sm font-medium mb-1">
                        <ShieldCheck className="h-4 w-4 text-green-500" />
                        Two-factor authentication is enabled
                    </div>
                    <p className="text-xs text-muted-foreground">Use an authenticator app (TOTP) to generate one-time codes.</p>
                    <button className="mt-3 text-xs underline underline-offset-2 text-muted-foreground hover:text-foreground transition-colors">
                        View recovery codes
                    </button>
                </div>
            )}
        </div>
    );
}

function SessionsTab() {
    const [sessions, setSessions] = useState(mockSessions);

    return (
        <div>
            <SectionHeader title="Sessions" description="Devices and clients currently signed in to your account." />

            <div className="border border-border rounded-md overflow-hidden mb-6">
                {sessions.map((session, i) => (
                    <div
                        key={session.id}
                        className={`flex items-start gap-3 px-4 py-4 ${i > 0 ? "border-t border-border" : ""} ${session.current ? "bg-secondary/40" : ""}`}
                    >
                        <Monitor className="h-4 w-4 text-muted-foreground shrink-0 mt-0.5" />
                        <div className="flex-1 min-w-0">
                            <div className="flex items-center gap-2 mb-0.5">
                                <span className="text-sm font-medium">{session.device}</span>
                                {session.current && <Tag>Current session</Tag>}
                            </div>
                            <p className="text-xs text-muted-foreground">
                                {session.ip} · {session.location} · {session.lastActive}
                            </p>
                        </div>
                        {!session.current && (
                            <button
                                onClick={() => setSessions((prev) => prev.filter((s) => s.id !== session.id))}
                                className="text-muted-foreground hover:text-destructive transition-colors text-xs flex items-center gap-1 shrink-0 mt-0.5"
                            >
                                <LogOut className="h-3.5 w-3.5" />
                                Revoke
                            </button>
                        )}
                    </div>
                ))}
            </div>

            <button
                onClick={() => setSessions((prev) => prev.filter((s) => s.current))}
                className="inline-flex items-center gap-2 px-4 h-9 border border-destructive/50 text-destructive text-sm font-medium rounded-md hover:bg-destructive/10 transition-colors"
            >
                <LogOut className="h-4 w-4" />
                Revoke all other sessions
            </button>
        </div>
    );
}

function KeysTab() {
    const [sshKeys, setSSHKeys] = useState(mockSSHKeys);
    const [gpgKeys, setGPGKeys] = useState(mockGPGKeys);
    const [newSSHTitle, setNewSSHTitle] = useState("");
    const [newSSHKey, setNewSSHKey] = useState("");

    return (
        <div>
            {/* SSH Keys */}
            <SectionHeader title="SSH Keys" description="SSH keys are used to authenticate your Git operations over SSH." />

            <div className="border border-border rounded-md overflow-hidden mb-4">
                {sshKeys.map((key, i) => (
                    <div key={key.id} className={`flex items-start gap-3 px-4 py-4 ${i > 0 ? "border-t border-border" : ""}`}>
                        <Key className="h-4 w-4 text-muted-foreground shrink-0 mt-0.5" />
                        <div className="flex-1 min-w-0">
                            <p className="text-sm font-medium mb-0.5">{key.title}</p>
                            <p className="font-mono text-xs text-muted-foreground truncate mb-0.5">{key.fingerprint}</p>
                            <p className="text-xs text-muted-foreground">
                                Added {key.addedAt} · Last used {key.lastUsed}
                            </p>
                        </div>
                        <button
                            onClick={() => setSSHKeys((prev) => prev.filter((k) => k.id !== key.id))}
                            className="text-muted-foreground hover:text-destructive transition-colors"
                        >
                            <Trash2 className="h-4 w-4" />
                        </button>
                    </div>
                ))}
                {sshKeys.length === 0 && <div className="px-4 py-8 text-center text-sm text-muted-foreground">No SSH keys added yet.</div>}
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
                <button className="inline-flex items-center gap-2 px-4 h-9 bg-foreground text-background text-sm font-medium rounded-md hover:opacity-90 transition-opacity">
                    <Plus className="h-4 w-4" />
                    Add SSH key
                </button>
            </div>

            <Divider />

            {/* GPG Keys */}
            <SectionHeader title="GPG Keys" description="GPG keys are used to verify that commits and tags come from a trusted source." />

            <div className="border border-border rounded-md overflow-hidden mb-4">
                {gpgKeys.map((key, i) => (
                    <div key={key.id} className={`flex items-start gap-3 px-4 py-4 ${i > 0 ? "border-t border-border" : ""}`}>
                        <ShieldCheck className="h-4 w-4 text-muted-foreground shrink-0 mt-0.5" />
                        <div className="flex-1 min-w-0">
                            <p className="text-sm font-medium mb-0.5">{key.name}</p>
                            <div className="flex items-center gap-3 text-xs text-muted-foreground">
                                <span className="font-mono">{key.keyId}</span>
                                <span>·</span>
                                <span>{key.subkeys} subkeys</span>
                                <span>·</span>
                                <span>Added {key.createdAt}</span>
                                <span>·</span>
                                <span>Expires {key.expiresAt}</span>
                            </div>
                        </div>
                        <button
                            onClick={() => setGPGKeys((prev) => prev.filter((k) => k.id !== key.id))}
                            className="text-muted-foreground hover:text-destructive transition-colors"
                        >
                            <Trash2 className="h-4 w-4" />
                        </button>
                    </div>
                ))}
                {gpgKeys.length === 0 && <div className="px-4 py-8 text-center text-sm text-muted-foreground">No GPG keys added yet.</div>}
            </div>

            <div>
                <FieldLabel>Add GPG key</FieldLabel>
                <Textarea
                    rows={5}
                    placeholder="Paste your ASCII-armored GPG key here — begins with -----BEGIN PGP PUBLIC KEY BLOCK-----"
                    className="font-mono text-xs mb-3"
                />
                <button className="inline-flex items-center gap-2 px-4 h-9 bg-foreground text-background text-sm font-medium rounded-md hover:opacity-90 transition-opacity">
                    <Plus className="h-4 w-4" />
                    Add GPG key
                </button>
            </div>
        </div>
    );
}

function RepositoriesTab() {
    const [defaultBranch, setDefaultBranch] = useState("main");
    const [defaultVisibility, setDefaultVisibility] = useState<"public" | "private">("private");
    const [defaultInit, setDefaultInit] = useState(true);
    const [defaultLicense, setDefaultLicense] = useState("MIT");
    const [mergeMethods, setMergeMethods] = useState({ merge: true, squash: true, rebase: false });

    return (
        <div>
            <SectionHeader title="Repository Settings" description="Default settings applied when creating new repositories." />

            <div className="space-y-6">
                <div className="grid grid-cols-2 gap-4">
                    <div>
                        <FieldLabel>Default branch name</FieldLabel>
                        <Input value={defaultBranch} onChange={(e) => setDefaultBranch(e.target.value)} placeholder="main" />
                        <p className="text-xs text-muted-foreground mt-1.5">Used when initializing new repositories.</p>
                    </div>
                    <div>
                        <FieldLabel>Default visibility</FieldLabel>
                        <div className="flex gap-2">
                            {(["public", "private"] as const).map((v) => (
                                <button
                                    key={v}
                                    onClick={() => setDefaultVisibility(v)}
                                    className={`flex-1 flex items-center justify-center gap-2 h-9 border rounded-md text-sm transition-colors capitalize ${
                                        defaultVisibility === v
                                            ? "border-foreground bg-foreground text-background"
                                            : "border-border hover:bg-accent/50"
                                    }`}
                                >
                                    {v === "public" ? <Globe className="h-3.5 w-3.5" /> : <Lock className="h-3.5 w-3.5" />}
                                    {v}
                                </button>
                            ))}
                        </div>
                    </div>
                </div>

                <div>
                    <FieldLabel>Default license</FieldLabel>
                    <select
                        value={defaultLicense}
                        onChange={(e) => setDefaultLicense(e.target.value)}
                        className="w-full h-9 px-3 bg-card border border-border rounded-md text-sm focus:outline-none focus:ring-1 focus:ring-ring"
                    >
                        <option value="">No license</option>
                        <option value="MIT">MIT License</option>
                        <option value="Apache-2.0">Apache License 2.0</option>
                        <option value="GPL-3.0">GNU GPLv3</option>
                        <option value="BSD-3-Clause">BSD 3-Clause</option>
                        <option value="MPL-2.0">Mozilla Public License 2.0</option>
                    </select>
                </div>

                <div>
                    <FieldLabel>Initialize with README</FieldLabel>
                    <div className="flex items-center gap-3">
                        <button
                            onClick={() => setDefaultInit((v) => !v)}
                            className={`relative inline-flex h-5 w-9 items-center rounded-full transition-colors focus:outline-none ${defaultInit ? "bg-foreground" : "bg-secondary border border-border"}`}
                        >
                            <span
                                className={`inline-block h-3.5 w-3.5 transform rounded-full bg-background transition-transform border border-border/50 ${defaultInit ? "translate-x-4" : "translate-x-1"}`}
                            />
                        </button>
                        <span className="text-sm text-muted-foreground">Auto-initialize new repositories with a README file</span>
                    </div>
                </div>

                <div>
                    <FieldLabel>Allowed merge methods</FieldLabel>
                    <p className="text-xs text-muted-foreground mb-3">
                        Control which merge strategies are available for merge requests across your repositories.
                    </p>
                    <div className="space-y-2">
                        {(
                            [
                                { key: "merge", label: "Merge commit", desc: "Preserve all commits from the source branch" },
                                { key: "squash", label: "Squash and merge", desc: "Combine all commits into a single commit" },
                                { key: "rebase", label: "Rebase and merge", desc: "Rebase commits onto the target branch" },
                            ] as const
                        ).map(({ key, label, desc }) => (
                            <div key={key} className="flex items-start gap-3 p-3 border border-border rounded-md">
                                <button
                                    onClick={() => setMergeMethods((prev) => ({ ...prev, [key]: !prev[key] }))}
                                    className={`relative mt-0.5 inline-flex h-4 w-4 shrink-0 items-center justify-center rounded border transition-colors ${mergeMethods[key] ? "bg-foreground border-foreground" : "border-border"}`}
                                >
                                    {mergeMethods[key] && <Check className="h-2.5 w-2.5 text-background" />}
                                </button>
                                <div>
                                    <p className="text-sm font-medium">{label}</p>
                                    <p className="text-xs text-muted-foreground">{desc}</p>
                                </div>
                            </div>
                        ))}
                    </div>
                </div>
            </div>

            <div className="mt-6">
                <SaveButton />
            </div>
        </div>
    );
}

function APIKeysTab() {
    const [keys, setKeys] = useState(mockAPIKeys);
    const [newName, setNewName] = useState("");
    const [revealed, setRevealed] = useState<string | null>(null);
    const [copied, setCopied] = useState<string | null>(null);

    function copyKey(id: string) {
        setCopied(id);
        setTimeout(() => setCopied(null), 2000);
    }

    return (
        <div>
            <SectionHeader title="API Keys" description="Personal API keys are used to authenticate requests to the GitArena API." />

            <div className="border border-border rounded-md overflow-hidden mb-6">
                {keys.map((key, i) => (
                    <div key={key.id} className={`flex items-start gap-3 px-4 py-4 ${i > 0 ? "border-t border-border" : ""}`}>
                        <Code2 className="h-4 w-4 text-muted-foreground shrink-0 mt-0.5" />
                        <div className="flex-1 min-w-0">
                            <div className="flex items-center gap-2 mb-1">
                                <span className="text-sm font-medium">{key.name}</span>
                                {key.expiresAt && <Tag>Expires {key.expiresAt}</Tag>}
                            </div>
                            <div className="flex items-center gap-1.5 mb-2 flex-wrap">
                                {key.scopes.map((scope) => (
                                    <Tag key={scope}>{scope}</Tag>
                                ))}
                            </div>
                            <p className="text-xs text-muted-foreground">
                                Created {key.createdAt} · Last used {key.lastUsed}
                            </p>

                            {/* Revealed key area */}
                            {revealed === key.id && (
                                <div className="mt-2 flex items-center gap-2">
                                    <code className="flex-1 text-xs font-mono bg-secondary border border-border rounded px-2 py-1 truncate">
                                        ga_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
                                    </code>
                                    <button
                                        onClick={() => copyKey(key.id)}
                                        className="text-muted-foreground hover:text-foreground transition-colors shrink-0"
                                    >
                                        {copied === key.id ? <Check className="h-4 w-4 text-green-500" /> : <Copy className="h-4 w-4" />}
                                    </button>
                                </div>
                            )}
                        </div>

                        <div className="flex items-center gap-2 shrink-0">
                            <button
                                onClick={() => setRevealed((v) => (v === key.id ? null : key.id))}
                                className="text-muted-foreground hover:text-foreground transition-colors"
                                title={revealed === key.id ? "Hide key" : "Reveal key"}
                            >
                                {revealed === key.id ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                            </button>
                            <button
                                onClick={() => setKeys((prev) => prev.filter((k) => k.id !== key.id))}
                                className="text-muted-foreground hover:text-destructive transition-colors"
                                title="Revoke key"
                            >
                                <Trash2 className="h-4 w-4" />
                            </button>
                        </div>
                    </div>
                ))}
                {keys.length === 0 && <div className="px-4 py-8 text-center text-sm text-muted-foreground">No API keys yet.</div>}
            </div>

            {/* New key */}
            <div className="space-y-4">
                <div>
                    <FieldLabel>Token name</FieldLabel>
                    <Input value={newName} onChange={(e) => setNewName(e.target.value)} placeholder="e.g. CI/CD Pipeline" />
                </div>
                <div>
                    <FieldLabel>Expiration</FieldLabel>
                    <select className="w-full h-9 px-3 bg-card border border-border rounded-md text-sm focus:outline-none focus:ring-1 focus:ring-ring">
                        <option value="">No expiration</option>
                        <option value="30">30 days</option>
                        <option value="60">60 days</option>
                        <option value="90">90 days</option>
                        <option value="custom">Custom date</option>
                    </select>
                </div>
                <div>
                    <FieldLabel>Scopes</FieldLabel>
                    <div className="space-y-2">
                        {["repo:read", "repo:write", "issue:read", "issue:write", "user:read", "user:write"].map((scope) => (
                            <label key={scope} className="flex items-center gap-2.5 cursor-pointer">
                                <input type="checkbox" className="rounded border-border" defaultChecked={scope === "repo:read"} />
                                <code className="text-xs font-mono">{scope}</code>
                            </label>
                        ))}
                    </div>
                </div>
                <button className="inline-flex items-center gap-2 px-4 h-9 bg-foreground text-background text-sm font-medium rounded-md hover:opacity-90 transition-opacity">
                    <Plus className="h-4 w-4" />
                    Generate token
                </button>
            </div>

            <Divider />

            <div className="flex items-start gap-3 p-4 border border-amber-500/30 bg-amber-500/5 rounded-md">
                <AlertCircle className="h-4 w-4 text-amber-500 shrink-0 mt-0.5" />
                <p className="text-xs text-muted-foreground leading-relaxed">
                    Treat your API keys like passwords. Do not share them or include them in version-controlled code. Revoke any keys you no
                    longer need.
                </p>
            </div>
        </div>
    );
}

// ── Page ───────────────────────────────────────────────────────────────────────

export default function SettingsPage() {
    const [activeTab, setActiveTab] = useState<Tab>("profile");

    const tabContent: Record<Tab, React.ReactNode> = {
        profile: <ProfileTab />,
        emails: <EmailsTab />,
        authentication: <AuthenticationTab />,
        sessions: <SessionsTab />,
        keys: <KeysTab />,
        repositories: <RepositoriesTab />,
        "api-keys": <APIKeysTab />,
    };

    return (
        <div className="flex flex-col h-screen overflow-hidden bg-background text-foreground font-sans">
            <TopBar
                breadcrumb={[{ label: currentUser.username, href: `/${currentUser.username}` }, { label: "Settings" }]}
                hasNotifications
            />

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

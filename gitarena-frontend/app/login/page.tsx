"use client";

import { useState, useEffect, Suspense } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import Link from "next/link";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { ErrorDisplay } from "@/components/error-display";
import { Github, Lock, Eye, EyeOff, Fingerprint, ArrowRight, Compass, GitMerge, KeyRound } from "lucide-react";
import { useAuth } from "@/hooks/use-auth";
import type { AuthUser } from "@/hooks/use-auth";
import useSWR from "swr";
import useSWRMutation from "swr/mutation";
import { startAuthentication } from "@simplewebauthn/browser";
import type { PublicKeyCredentialRequestOptionsJSON } from "@simplewebauthn/browser";
import { postJsonFetcher } from "@/lib/fetchers";
import { toast } from "sonner";

// SSO Provider icons
function GitLabIcon({ className }: { className?: string }) {
    return (
        <svg className={className} viewBox="0 0 24 24" fill="currentColor">
            <path d="M22.65 14.39L12 22.13 1.35 14.39a.84.84 0 0 1-.3-.94l1.22-3.78 2.44-7.51A.42.42 0 0 1 4.82 2a.43.43 0 0 1 .58 0 .42.42 0 0 1 .11.18l2.44 7.49h8.1l2.44-7.51A.42.42 0 0 1 18.6 2a.43.43 0 0 1 .58 0 .42.42 0 0 1 .11.18l2.44 7.51L23 13.45a.84.84 0 0 1-.35.94z" />
        </svg>
    );
}

function BitbucketIcon({ className }: { className?: string }) {
    return (
        <svg className={className} viewBox="0 0 24 24" fill="currentColor">
            <path d="M.778 1.213a.768.768 0 0 0-.768.892l3.263 19.81c.084.5.515.868 1.022.873H19.95a.772.772 0 0 0 .77-.646l3.27-20.03a.768.768 0 0 0-.768-.891zM14.52 15.53H9.522L8.17 8.466h7.561z" />
        </svg>
    );
}

interface SSOProviders {
    github: boolean;
    gitlab: boolean;
    bitbucket: boolean;
}

interface PasskeyLoginStartResponse {
    challenge_id: string;
    options: { publicKey: PublicKeyCredentialRequestOptionsJSON };
}

interface PasskeyLoginFinishRequest {
    challenge_id: string;
    credential: object;
}

export default function LoginPage() {
    return (
        <Suspense>
            <LoginContent />
        </Suspense>
    );
}

function LoginContent() {
    const router = useRouter();
    const searchParams = useSearchParams();
    const { isAuthenticated, isLoading: authLoading, isLoggingIn, loginError, login, mutate: mutateAuth } = useAuth();

    const [showPassword, setShowPassword] = useState(false);
    const [authMethod, setAuthMethod] = useState<"password" | "passkey">("password");
    const [form, setForm] = useState({ identifier: "", password: "" });
    const { data, isLoading, error } = useSWR<SSOProviders>("/api/sso");

    const { trigger: triggerPasskeyStart, isMutating: isPasskeyStarting } = useSWRMutation<
        PasskeyLoginStartResponse,
        Error,
        string,
        Record<string, never>
    >("/api/auth/passkey/login/start", postJsonFetcher);

    const { trigger: triggerPasskeyFinish, isMutating: isPasskeyFinishing } = useSWRMutation<
        AuthUser,
        Error,
        string,
        PasskeyLoginFinishRequest
    >("/api/auth/passkey/login/finish", postJsonFetcher, {
        onSuccess: (user) => mutateAuth(user, { revalidate: false }),
    });

    const isPasskeyLoading = isPasskeyStarting || isPasskeyFinishing;

    const rawRedirect = searchParams.get("redirect") ?? "/";
    // Reject absolute URLs and protocol-relative URLs to prevent open redirects.
    const redirect = rawRedirect.startsWith("/") && !rawRedirect.startsWith("//") ? rawRedirect : "/";

    useEffect(() => {
        if (!authLoading && isAuthenticated) {
            router.replace(redirect);
        }
    }, [isAuthenticated, authLoading, redirect, router]);

    async function handleSubmit(e: React.FormEvent) {
        e.preventDefault();
        const user = await login(form.identifier, form.password).catch(() => null);
        if (user) {
            router.push(redirect);
        }
    }

    async function handlePasskeyLogin() {
        try {
            const startResponse = await triggerPasskeyStart({} as Record<string, never>);
            const credential = await startAuthentication({ optionsJSON: startResponse.options.publicKey });
            const user = await triggerPasskeyFinish({ challenge_id: startResponse.challenge_id, credential });
            if (user) {
                router.push(redirect);
            }
        } catch (err) {
            toast.error(err instanceof Error ? err.message : "Passkey authentication failed");
        }
    }

    const anySsoEnabled = !!data && (data.github || data.gitlab || data.bitbucket);

    const showSsoSection = isLoading || error || anySsoEnabled;

    return (
        <div className="min-h-screen bg-background flex flex-col">
            <header className="border-b border-border shrink-0">
                <div className="flex h-14 items-center justify-between px-5 gap-5">
                    <div className="flex items-center gap-3">
                        <Link href="/" className="text-base font-semibold tracking-tight">
                            GITARENA
                        </Link>
                    </div>

                    <nav className="flex items-center gap-2">
                        <Link href="/explore">
                            <Button variant="ghost" size="sm" className="text-muted-foreground h-10 px-4 gap-2.5 text-base">
                                <Compass className="h-[18px] w-[18px]" />
                                <span className="hidden sm:inline">Explore</span>
                            </Button>
                        </Link>
                        <Link href="#">
                            <Button variant="ghost" size="sm" className="text-muted-foreground h-10 px-4 gap-2.5 text-base">
                                <GitMerge className="h-[18px] w-[18px]" />
                                <span className="hidden sm:inline">Merge Requests</span>
                            </Button>
                        </Link>
                        <div className="w-px h-7 bg-border mx-3" />
                        <Link href="/register">
                            <Button variant="secondary" size="sm" className="h-10 px-4 text-base">
                                Sign up
                            </Button>
                        </Link>
                    </nav>
                </div>
            </header>

            <main className="flex-1 flex">
                <aside className="w-80 border-r border-border p-8 hidden lg:flex flex-col justify-between">
                    <div className="space-y-8">
                        <div>
                            <h2 className="text-xl font-semibold mb-3">Welcome back</h2>
                            <p className="text-muted-foreground">
                                Sign in to access your repositories, collaborate with your team, and manage your projects.
                            </p>
                        </div>

                        <div className="space-y-4">
                            <div className="flex items-start gap-3">
                                <div className="flex items-center justify-center h-8 w-8 rounded-lg bg-card border border-border shrink-0">
                                    <Lock className="h-4 w-4 text-muted-foreground" />
                                </div>
                                <div>
                                    <p className="font-medium">Secure by default</p>
                                    <p className="text-sm text-muted-foreground">2FA, passkeys, and SSO support</p>
                                </div>
                            </div>
                            <div className="flex items-start gap-3">
                                <div className="flex items-center justify-center h-8 w-8 rounded-lg bg-card border border-border shrink-0">
                                    <Fingerprint className="h-4 w-4 text-muted-foreground" />
                                </div>
                                <div>
                                    <p className="font-medium">Passwordless login</p>
                                    <p className="text-sm text-muted-foreground">Use passkeys for instant access</p>
                                </div>
                            </div>
                        </div>
                    </div>

                    <div className="pt-6 border-t border-border">
                        <p className="text-sm text-muted-foreground">
                            New to GitArena?{" "}
                            <Link href="/register" className="text-foreground hover:underline">
                                Create an account
                            </Link>
                        </p>
                    </div>
                </aside>

                <div className="flex-1 flex items-center justify-center p-8">
                    <div className="w-full max-w-md space-y-6">
                        <div className="flex items-center gap-3 mb-8">
                            <div className="flex items-center justify-center h-12 w-12 rounded-lg bg-card border border-border">
                                <KeyRound className="h-6 w-6 text-muted-foreground" />
                            </div>
                            <div>
                                <h1 className="text-2xl font-semibold">Sign in</h1>
                                <p className="text-muted-foreground">Access your GitArena account</p>
                            </div>
                        </div>

                        <div className="flex items-center gap-1 p-1 bg-card border border-border rounded-lg">
                            <button
                                onClick={() => setAuthMethod("password")}
                                className={`flex-1 flex items-center justify-center gap-2 py-3 rounded-md transition-colors ${
                                    authMethod === "password"
                                        ? "bg-background text-foreground border border-border"
                                        : "text-muted-foreground hover:text-foreground"
                                }`}
                            >
                                <Lock className="h-4 w-4" />
                                Password
                            </button>
                            <button
                                onClick={() => setAuthMethod("passkey")}
                                className={`flex-1 flex items-center justify-center gap-2 py-3 rounded-md transition-colors ${
                                    authMethod === "passkey"
                                        ? "bg-background text-foreground border border-border"
                                        : "text-muted-foreground hover:text-foreground"
                                }`}
                            >
                                <Fingerprint className="h-4 w-4" />
                                Passkey
                            </button>
                        </div>

                        {authMethod === "password" ? (
                            <form className="space-y-4" onSubmit={handleSubmit}>
                                {loginError && (
                                    <div className="px-4 py-3 rounded-md bg-destructive/10 border border-destructive/20 text-sm text-destructive">
                                        {loginError.message}
                                    </div>
                                )}

                                <div className="space-y-2">
                                    <label htmlFor="identifier" className="text-sm font-medium">
                                        Username or email
                                    </label>
                                    <input
                                        id="identifier"
                                        type="text"
                                        value={form.identifier}
                                        onChange={(e) => setForm((f) => ({ ...f, identifier: e.target.value }))}
                                        placeholder="you@example.com"
                                        autoComplete="username"
                                        required
                                        disabled={isLoggingIn}
                                        className="w-full h-11 px-4 bg-card border border-border rounded-md text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring disabled:opacity-50"
                                    />
                                </div>

                                <div className="space-y-2">
                                    <div className="flex items-center justify-between">
                                        <label htmlFor="password" className="text-sm font-medium">
                                            Password
                                        </label>
                                        <Link
                                            href="/forgot-password"
                                            className="text-sm text-muted-foreground hover:text-foreground transition-colors"
                                        >
                                            Forgot password?
                                        </Link>
                                    </div>
                                    <div className="relative">
                                        <input
                                            id="password"
                                            type={showPassword ? "text" : "password"}
                                            value={form.password}
                                            onChange={(e) => setForm((f) => ({ ...f, password: e.target.value }))}
                                            placeholder="Enter your password"
                                            autoComplete="current-password"
                                            required
                                            disabled={isLoggingIn}
                                            className="w-full h-11 px-4 pr-11 bg-card border border-border rounded-md text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring disabled:opacity-50"
                                        />
                                        <button
                                            type="button"
                                            onClick={() => setShowPassword(!showPassword)}
                                            className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground transition-colors"
                                        >
                                            {showPassword ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                                        </button>
                                    </div>
                                </div>

                                <Button type="submit" className="w-full h-11 gap-2" disabled={isLoggingIn}>
                                    {isLoggingIn ? "Signing in..." : "Sign in"}
                                    {!isLoggingIn && <ArrowRight className="h-4 w-4" />}
                                </Button>
                            </form>
                        ) : (
                            <div className="space-y-4">
                                <div className="p-6 bg-card border border-border rounded-lg text-center space-y-4">
                                    <div className="flex items-center justify-center h-16 w-16 mx-auto rounded-full bg-secondary">
                                        <Fingerprint className="h-8 w-8 text-muted-foreground" />
                                    </div>
                                    <div>
                                        <p className="font-medium">Sign in with your passkey</p>
                                        <p className="text-sm text-muted-foreground mt-1">
                                            Use your device&apos;s biometric authentication or security key
                                        </p>
                                    </div>
                                    <Button
                                        type="button"
                                        className="w-full h-11 gap-2"
                                        onClick={handlePasskeyLogin}
                                        disabled={isPasskeyLoading}
                                    >
                                        <Fingerprint className="h-5 w-5" />
                                        {isPasskeyLoading ? "Authenticating..." : "Continue with Passkey"}
                                    </Button>
                                </div>
                            </div>
                        )}

                        {showSsoSection && (
                            <>
                                <div className="relative">
                                    <div className="absolute inset-0 flex items-center">
                                        <div className="w-full border-t border-border" />
                                    </div>
                                    <div className="relative flex justify-center text-sm">
                                        <span className="bg-background px-4 text-muted-foreground">or continue with</span>
                                    </div>
                                </div>

                                {isLoading && (
                                    <div className="flex items-center justify-center gap-3">
                                        <Skeleton className="h-11 w-11 rounded-lg" />
                                        <Skeleton className="h-11 w-11 rounded-lg" />
                                        <Skeleton className="h-11 w-11 rounded-lg" />
                                    </div>
                                )}

                                {error && <ErrorDisplay failed="SSO providers" error={error} />}

                                {anySsoEnabled && (
                                    <div className="flex items-center justify-center gap-3">
                                        {data?.github && (
                                            <a
                                                href="/api/sso/github"
                                                className="flex items-center justify-center h-11 w-11 bg-card border border-border rounded-lg hover:bg-accent/50 transition-colors"
                                                title="GitHub"
                                            >
                                                <Github className="h-5 w-5" />
                                            </a>
                                        )}
                                        {data?.gitlab && (
                                            <a
                                                href="/api/sso/gitlab"
                                                className="flex items-center justify-center h-11 w-11 bg-card border border-border rounded-lg hover:bg-accent/50 transition-colors"
                                                title="GitLab"
                                            >
                                                <GitLabIcon className="h-5 w-5" />
                                            </a>
                                        )}
                                        {data?.bitbucket && (
                                            <a
                                                href="/api/sso/bitbucket"
                                                className="flex items-center justify-center h-11 w-11 bg-card border border-border rounded-lg hover:bg-accent/50 transition-colors"
                                                title="Bitbucket"
                                            >
                                                <BitbucketIcon className="h-5 w-5" />
                                            </a>
                                        )}
                                    </div>
                                )}
                            </>
                        )}

                        <p className="text-center text-sm text-muted-foreground lg:hidden">
                            New to GitArena?{" "}
                            <Link href="/register" className="text-foreground hover:underline">
                                Create an account
                            </Link>
                        </p>
                    </div>
                </div>
            </main>

            <footer className="border-t border-border p-4">
                <div className="flex items-center justify-center gap-6 text-sm text-muted-foreground">
                    <Link href="#" className="hover:text-foreground transition-colors">
                        Terms
                    </Link>
                    <Link href="#" className="hover:text-foreground transition-colors">
                        Privacy
                    </Link>
                    <Link href="#" className="hover:text-foreground transition-colors">
                        Contact
                    </Link>
                </div>
            </footer>
        </div>
    );
}

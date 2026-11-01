"use client";

import { useState, useEffect, useCallback, Suspense } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import Link from "next/link";
import { Button } from "@/components/ui/button";
import { Lock, Eye, EyeOff, Fingerprint, ArrowRight, Compass, GitMerge, KeyRound } from "lucide-react";
import { SsoProviders } from "@/components/sso-providers";
import { useAuth } from "@/hooks/use-auth";
import type { AuthUser } from "@/hooks/use-auth";
import useSWRMutation from "swr/mutation";
import { browserSupportsWebAuthnAutofill, startAuthentication, WebAuthnAbortService, WebAuthnError } from "@simplewebauthn/browser";
import type { PublicKeyCredentialRequestOptionsJSON } from "@simplewebauthn/browser";
import { postJsonFetcher } from "@/lib/fetchers";
import { toast } from "sonner";
import { Input } from "@/components/ui/input";
import { InputGroup, InputGroupAddon, InputGroupButton, InputGroupInput } from "@/components/ui/input-group";
import { Separator } from "@/components/ui/separator";
import { Alert, AlertDescription } from "@/components/ui/alert";

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
    const [passkeyRound, setPasskeyRound] = useState(0);
    const [form, setForm] = useState({ identifier: "", password: "" });

    const { trigger: triggerPasskeyStart } = useSWRMutation<PasskeyLoginStartResponse, Error, string, Record<string, never>>(
        "/api/auth/passkey/login/start",
        postJsonFetcher
    );

    const { trigger: triggerPasskeyFinish, isMutating: isPasskeyFinishing } = useSWRMutation<
        AuthUser,
        Error,
        string,
        PasskeyLoginFinishRequest
    >("/api/auth/passkey/login/finish", postJsonFetcher, {
        onSuccess: (user) => mutateAuth(user, { revalidate: false }),
    });

    const rawRedirect = searchParams.get("redirect") ?? "/";
    // Reject absolute URLs and protocol-relative URLs to prevent open redirects.
    const redirect = rawRedirect.startsWith("/") && !rawRedirect.startsWith("//") ? rawRedirect : "/";

    useEffect(() => {
        if (!authLoading && isAuthenticated) {
            router.replace(redirect);
        }
    }, [isAuthenticated, authLoading, redirect, router]);

    const loginWithPasskey = useCallback(
        async (useBrowserAutofill: boolean, isCancelled: () => boolean = () => false) => {
            try {
                const startResponse = await triggerPasskeyStart({} as Record<string, never>);
                if (isCancelled()) {
                    return;
                }
                const credential = await startAuthentication({ optionsJSON: startResponse.options.publicKey, useBrowserAutofill });
                const user = await triggerPasskeyFinish({ challenge_id: startResponse.challenge_id, credential });
                if (user) {
                    router.push(redirect);
                }
            } catch (err) {
                if (err instanceof WebAuthnError && err.code === "ERROR_CEREMONY_ABORTED") {
                    return;
                }
                toast.error(err instanceof Error ? err.message : "Passkey authentication failed");
                // Only re-arm autofill after user interaction, otherwise errors that happen without any
                // (backend down, bad RP ID) would retry in a loop.
                if (!useBrowserAutofill || (err instanceof Error && err.name === "NotAllowedError")) {
                    setPasskeyRound((round) => round + 1);
                }
            }
        },
        [triggerPasskeyStart, triggerPasskeyFinish, router, redirect]
    );

    useEffect(() => {
        if (authLoading || isAuthenticated) {
            return;
        }

        let cancelled = false;
        browserSupportsWebAuthnAutofill().then((supported) => {
            if (supported && !cancelled) {
                void loginWithPasskey(true, () => cancelled);
            }
        });

        // Login challenges expire after 5 minutes on the backend, so grab a fresh one before that.
        const refresh = setTimeout(() => setPasskeyRound((round) => round + 1), 4 * 60 * 1000);

        return () => {
            cancelled = true;
            clearTimeout(refresh);
            WebAuthnAbortService.cancelCeremony();
        };
    }, [authLoading, isAuthenticated, loginWithPasskey, passkeyRound]);

    async function handleSubmit(e: React.FormEvent) {
        e.preventDefault();
        const user = await login(form.identifier, form.password).catch(() => null);
        if (user) {
            router.push(redirect);
        }
    }

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
                        <Separator orientation="vertical" className="mx-3 data-[orientation=vertical]:h-7" />
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

                        <form className="space-y-4" onSubmit={handleSubmit}>
                            {loginError && (
                                <Alert variant="destructive">
                                    <AlertDescription>{loginError.message}</AlertDescription>
                                </Alert>
                            )}

                            <div className="space-y-2">
                                <label htmlFor="identifier" className="text-sm font-medium">
                                    Username or email
                                </label>
                                <Input
                                    id="identifier"
                                    value={form.identifier}
                                    onChange={(e) => setForm((f) => ({ ...f, identifier: e.target.value }))}
                                    placeholder="you@example.com"
                                    autoComplete="username webauthn"
                                    required
                                    disabled={isLoggingIn}
                                    className="h-11 px-4 md:text-base"
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
                                <InputGroup className="h-11">
                                    <InputGroupInput
                                        id="password"
                                        type={showPassword ? "text" : "password"}
                                        value={form.password}
                                        onChange={(e) => setForm((f) => ({ ...f, password: e.target.value }))}
                                        placeholder="Enter your password"
                                        autoComplete="current-password"
                                        required
                                        disabled={isLoggingIn}
                                        className="md:text-base"
                                    />
                                    <InputGroupAddon align="inline-end">
                                        <InputGroupButton size="icon-xs" onClick={() => setShowPassword(!showPassword)}>
                                            {showPassword ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                                        </InputGroupButton>
                                    </InputGroupAddon>
                                </InputGroup>
                            </div>

                            <Button type="submit" className="w-full h-11 gap-2" disabled={isLoggingIn}>
                                {isLoggingIn ? "Signing in..." : "Sign in"}
                                {!isLoggingIn && <ArrowRight className="h-4 w-4" />}
                            </Button>

                            <Button
                                type="button"
                                variant="outline"
                                className="w-full h-11 gap-2"
                                onClick={() => loginWithPasskey(false)}
                                disabled={isPasskeyFinishing}
                            >
                                <Fingerprint className="h-4 w-4" />
                                {isPasskeyFinishing ? "Authenticating..." : "Sign in with a passkey"}
                            </Button>
                        </form>
                        <SsoProviders />

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

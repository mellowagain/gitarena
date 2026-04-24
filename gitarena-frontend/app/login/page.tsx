"use client";

import { useState, useEffect, Suspense } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import Link from "next/link";
import { Button } from "@/components/ui/button";
import { Github, Lock, Eye, EyeOff, Fingerprint, ArrowRight, Compass, GitMerge, KeyRound } from "lucide-react";
import { useAuth } from "@/hooks/use-auth";

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

function GoogleIcon({ className }: { className?: string }) {
    return (
        <svg className={className} viewBox="0 0 24 24" fill="currentColor">
            <path d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z" />
            <path d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z" />
            <path d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z" />
            <path d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z" />
        </svg>
    );
}

function MicrosoftIcon({ className }: { className?: string }) {
    return (
        <svg className={className} viewBox="0 0 24 24" fill="currentColor">
            <path d="M11.4 24H0V12.6h11.4V24zM24 24H12.6V12.6H24V24zM11.4 11.4H0V0h11.4v11.4zm12.6 0H12.6V0H24v11.4z" />
        </svg>
    );
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
    const { isAuthenticated, isLoading, isLoggingIn, loginError, login } = useAuth();

    const [showPassword, setShowPassword] = useState(false);
    const [authMethod, setAuthMethod] = useState<"password" | "passkey">("password");
    const [form, setForm] = useState({ identifier: "", password: "" });

    const rawRedirect = searchParams.get("redirect") ?? "/";
    // Reject absolute URLs and protocol-relative URLs to prevent open redirects.
    const redirect = rawRedirect.startsWith("/") && !rawRedirect.startsWith("//") ? rawRedirect : "/";

    useEffect(() => {
        if (!isLoading && isAuthenticated) {
            router.replace(redirect);
        }
    }, [isAuthenticated, isLoading, redirect, router]);

    async function handleSubmit(e: React.FormEvent) {
        e.preventDefault();
        const user = await login(form.identifier, form.password).catch(() => null);
        if (user) { router.push(redirect); }
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
                                        onChange={(e) => setForm(f => ({ ...f, identifier: e.target.value }))}
                                        placeholder="you@example.com"
                                        autoComplete="username"
                                        required
                                        disabled={isSubmitting}
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
                                            onChange={(e) => setForm(f => ({ ...f, password: e.target.value }))}
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
                                    <Button type="button" className="w-full h-11 gap-2">
                                        <Fingerprint className="h-5 w-5" />
                                        Continue with Passkey
                                    </Button>
                                </div>
                            </div>
                        )}

                        <div className="relative">
                            <div className="absolute inset-0 flex items-center">
                                <div className="w-full border-t border-border" />
                            </div>
                            <div className="relative flex justify-center text-sm">
                                <span className="bg-background px-4 text-muted-foreground">or continue with</span>
                            </div>
                        </div>

                        <div className="flex items-center justify-center gap-3">
                            <button
                                className="flex items-center justify-center h-11 w-11 bg-card border border-border rounded-lg hover:bg-accent/50 transition-colors"
                                title="GitHub"
                            >
                                <Github className="h-5 w-5" />
                            </button>
                            <button
                                className="flex items-center justify-center h-11 w-11 bg-card border border-border rounded-lg hover:bg-accent/50 transition-colors"
                                title="GitLab"
                            >
                                <GitLabIcon className="h-5 w-5" />
                            </button>
                            <button
                                className="flex items-center justify-center h-11 w-11 bg-card border border-border rounded-lg hover:bg-accent/50 transition-colors"
                                title="Bitbucket"
                            >
                                <BitbucketIcon className="h-5 w-5" />
                            </button>
                            <button
                                className="flex items-center justify-center h-11 w-11 bg-card border border-border rounded-lg hover:bg-accent/50 transition-colors"
                                title="Google"
                            >
                                <GoogleIcon className="h-5 w-5" />
                            </button>
                            <button
                                className="flex items-center justify-center h-11 w-11 bg-card border border-border rounded-lg hover:bg-accent/50 transition-colors"
                                title="Microsoft"
                            >
                                <MicrosoftIcon className="h-5 w-5" />
                            </button>
                        </div>

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

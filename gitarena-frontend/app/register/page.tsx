"use client";

import { useState, useEffect } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";
import { Button } from "@/components/ui/button";
import { Github, Mail, Lock, Eye, EyeOff, User, ArrowRight, Check, Compass, GitMerge, UserPlus } from "lucide-react";
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

function PasswordStrength({ password }: { password: string }) {
    const checks = [
        { label: "8+ chars", met: password.length >= 8 },
        { label: "Uppercase", met: /[A-Z]/.test(password) },
        { label: "Number", met: /[0-9]/.test(password) },
        { label: "Special", met: /[^A-Za-z0-9]/.test(password) },
    ];

    const strength = checks.filter((c) => c.met).length;

    return (
        <div className="space-y-2 pt-1">
            <div className="flex gap-1">
                {[1, 2, 3, 4].map((level) => (
                    <div
                        key={level}
                        className={`h-1.5 flex-1 rounded-full transition-colors ${
                            strength >= level
                                ? strength <= 1
                                    ? "bg-red-500"
                                    : strength === 2
                                      ? "bg-orange-500"
                                      : strength === 3
                                        ? "bg-yellow-500"
                                        : "bg-green-500"
                                : "bg-secondary"
                        }`}
                    />
                ))}
            </div>
            <div className="flex flex-wrap gap-x-4 gap-y-1">
                {checks.map((check) => (
                    <div key={check.label} className="flex items-center gap-1.5 text-xs">
                        <div
                            className={`flex items-center justify-center h-4 w-4 rounded-full ${
                                check.met ? "bg-green-500/20 text-green-500" : "bg-secondary text-muted-foreground"
                            }`}
                        >
                            {check.met && <Check className="h-2.5 w-2.5" />}
                        </div>
                        <span className={check.met ? "text-foreground" : "text-muted-foreground"}>{check.label}</span>
                    </div>
                ))}
            </div>
        </div>
    );
}

export default function RegisterPage() {
    const router = useRouter();
    const { isAuthenticated, isLoading, isRegistering, registerError, register } = useAuth();

    const [showPassword, setShowPassword] = useState(false);
    const [form, setForm] = useState({ username: "", email: "", password: "" });

    useEffect(() => {
        if (!isLoading && isAuthenticated) {
            router.replace("/");
        }
    }, [isAuthenticated, isLoading, router]);

    async function handleSubmit(e: React.FormEvent) {
        e.preventDefault();
        const user = await register(form.username, form.email, form.password).catch(() => null);
        if (user) {
            router.push("/");
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
                        <div className="w-px h-7 bg-border mx-3" />
                        <Link href="/login">
                            <Button variant="secondary" size="sm" className="h-10 px-4 text-base">
                                Sign in
                            </Button>
                        </Link>
                    </nav>
                </div>
            </header>

            <main className="flex-1 flex">
                <aside className="w-80 border-r border-border p-8 hidden lg:flex flex-col justify-between">
                    <div className="space-y-8">
                        <div>
                            <h2 className="text-xl font-semibold mb-3">Join GitArena</h2>
                            <p className="text-muted-foreground">
                                Create an account to start hosting your code, collaborating with others, and building amazing projects.
                            </p>
                        </div>

                        <div className="space-y-4">
                            <div className="flex items-start gap-3">
                                <div className="flex items-center justify-center h-5 w-5 rounded-full bg-green-500/20 shrink-0 mt-0.5">
                                    <Check className="h-3 w-3 text-green-500" />
                                </div>
                                <div>
                                    <p className="font-medium">Unlimited repositories</p>
                                    <p className="text-sm text-muted-foreground">Public and private, no limits</p>
                                </div>
                            </div>
                            <div className="flex items-start gap-3">
                                <div className="flex items-center justify-center h-5 w-5 rounded-full bg-green-500/20 shrink-0 mt-0.5">
                                    <Check className="h-3 w-3 text-green-500" />
                                </div>
                                <div>
                                    <p className="font-medium">Built-in CI/CD</p>
                                    <p className="text-sm text-muted-foreground">Automated builds and deployments</p>
                                </div>
                            </div>
                            <div className="flex items-start gap-3">
                                <div className="flex items-center justify-center h-5 w-5 rounded-full bg-green-500/20 shrink-0 mt-0.5">
                                    <Check className="h-3 w-3 text-green-500" />
                                </div>
                                <div>
                                    <p className="font-medium">Issue tracking</p>
                                    <p className="text-sm text-muted-foreground">Manage tasks and bugs</p>
                                </div>
                            </div>
                            <div className="flex items-start gap-3">
                                <div className="flex items-center justify-center h-5 w-5 rounded-full bg-green-500/20 shrink-0 mt-0.5">
                                    <Check className="h-3 w-3 text-green-500" />
                                </div>
                                <div>
                                    <p className="font-medium">Self-hostable</p>
                                    <p className="text-sm text-muted-foreground">Run on your own infrastructure</p>
                                </div>
                            </div>
                        </div>
                    </div>

                    <div className="pt-6 border-t border-border">
                        <p className="text-sm text-muted-foreground">
                            Already have an account?{" "}
                            <Link href="/login" className="text-foreground hover:underline">
                                Sign in
                            </Link>
                        </p>
                    </div>
                </aside>

                <div className="flex-1 flex items-center justify-center p-8">
                    <div className="w-full max-w-md space-y-6">
                        <div className="flex items-center gap-3 mb-8">
                            <div className="flex items-center justify-center h-12 w-12 rounded-lg bg-card border border-border">
                                <UserPlus className="h-6 w-6 text-muted-foreground" />
                            </div>
                            <div>
                                <h1 className="text-2xl font-semibold">Create account</h1>
                                <p className="text-muted-foreground">Get started with GitArena</p>
                            </div>
                        </div>

                        <form className="space-y-4" onSubmit={handleSubmit}>
                            {registerError && (
                                <div className="px-4 py-3 rounded-md bg-destructive/10 border border-destructive/20 text-sm text-destructive">
                                    {registerError.message}
                                </div>
                            )}

                            <div className="space-y-2">
                                <label htmlFor="username" className="text-sm font-medium">
                                    Username
                                </label>
                                <div className="relative">
                                    <User className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                                    <input
                                        id="username"
                                        type="text"
                                        value={form.username}
                                        onChange={(e) => setForm((f) => ({ ...f, username: e.target.value }))}
                                        placeholder="johndoe"
                                        autoComplete="username"
                                        required
                                        disabled={isRegistering}
                                        className="w-full h-11 pl-10 pr-4 bg-card border border-border rounded-md text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring disabled:opacity-50"
                                    />
                                </div>
                                {form.username && (
                                    <p className="text-xs text-muted-foreground">
                                        Your profile: <span className="text-foreground font-mono">gitarena.dev/{form.username}</span>
                                    </p>
                                )}
                            </div>

                            <div className="space-y-2">
                                <label htmlFor="email" className="text-sm font-medium">
                                    Email
                                </label>
                                <div className="relative">
                                    <Mail className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                                    <input
                                        id="email"
                                        type="email"
                                        value={form.email}
                                        onChange={(e) => setForm((f) => ({ ...f, email: e.target.value }))}
                                        placeholder="you@example.com"
                                        autoComplete="email"
                                        required
                                        disabled={isRegistering}
                                        className="w-full h-11 pl-10 pr-4 bg-card border border-border rounded-md text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring disabled:opacity-50"
                                    />
                                </div>
                            </div>

                            <div className="space-y-2">
                                <label htmlFor="password" className="text-sm font-medium">
                                    Password
                                </label>
                                <div className="relative">
                                    <Lock className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                                    <input
                                        id="password"
                                        type={showPassword ? "text" : "password"}
                                        value={form.password}
                                        onChange={(e) => setForm((f) => ({ ...f, password: e.target.value }))}
                                        placeholder="Create a password"
                                        autoComplete="new-password"
                                        required
                                        disabled={isRegistering}
                                        className="w-full h-11 pl-10 pr-11 bg-card border border-border rounded-md text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring disabled:opacity-50"
                                    />
                                    <button
                                        type="button"
                                        onClick={() => setShowPassword(!showPassword)}
                                        className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground transition-colors"
                                    >
                                        {showPassword ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                                    </button>
                                </div>
                                {form.password && <PasswordStrength password={form.password} />}
                            </div>

                            <Button type="submit" className="w-full h-11 gap-2" disabled={isRegistering}>
                                {isRegistering ? "Creating account..." : "Create account"}
                                {!isRegistering && <ArrowRight className="h-4 w-4" />}
                            </Button>

                            <p className="text-center text-xs text-muted-foreground">
                                By creating an account, you agree to our{" "}
                                <Link href="#" className="text-foreground hover:underline">
                                    Terms
                                </Link>{" "}
                                and{" "}
                                <Link href="#" className="text-foreground hover:underline">
                                    Privacy Policy
                                </Link>
                            </p>
                        </form>

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
                            Already have an account?{" "}
                            <Link href="/login" className="text-foreground hover:underline">
                                Sign in
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

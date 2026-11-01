"use client";

import { useState, useEffect } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";
import { Button } from "@/components/ui/button";
import { Mail, Lock, Eye, EyeOff, User, ArrowRight, Check, Compass, GitMerge, UserPlus } from "lucide-react";
import { SsoProviders } from "@/components/sso-providers";
import { useAuth } from "@/hooks/use-auth";
import { InputGroup, InputGroupAddon, InputGroupButton, InputGroupInput } from "@/components/ui/input-group";
import { Separator } from "@/components/ui/separator";
import { Alert, AlertDescription } from "@/components/ui/alert";

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
    const { isAuthenticated, isLoading: authLoading, isRegistering, registerError, register } = useAuth();

    const [showPassword, setShowPassword] = useState(false);
    const [form, setForm] = useState({ username: "", email: "", password: "" });

    useEffect(() => {
        if (!authLoading && isAuthenticated) {
            router.replace("/");
        }
    }, [isAuthenticated, authLoading, router]);

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
                        <Separator orientation="vertical" className="mx-3 data-[orientation=vertical]:h-7" />
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
                                <Alert variant="destructive">
                                    <AlertDescription>{registerError.message}</AlertDescription>
                                </Alert>
                            )}

                            <div className="space-y-2">
                                <label htmlFor="username" className="text-sm font-medium">
                                    Username
                                </label>
                                <InputGroup className="h-11">
                                    <InputGroupAddon>
                                        <User />
                                    </InputGroupAddon>
                                    <InputGroupInput
                                        id="username"
                                        value={form.username}
                                        onChange={(e) => setForm((f) => ({ ...f, username: e.target.value }))}
                                        placeholder="johndoe"
                                        autoComplete="username"
                                        required
                                        disabled={isRegistering}
                                        className="md:text-base"
                                    />
                                </InputGroup>
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
                                <InputGroup className="h-11">
                                    <InputGroupAddon>
                                        <Mail />
                                    </InputGroupAddon>
                                    <InputGroupInput
                                        id="email"
                                        type="email"
                                        value={form.email}
                                        onChange={(e) => setForm((f) => ({ ...f, email: e.target.value }))}
                                        placeholder="you@example.com"
                                        autoComplete="email"
                                        required
                                        disabled={isRegistering}
                                        className="md:text-base"
                                    />
                                </InputGroup>
                            </div>

                            <div className="space-y-2">
                                <label htmlFor="password" className="text-sm font-medium">
                                    Password
                                </label>
                                <InputGroup className="h-11">
                                    <InputGroupAddon>
                                        <Lock />
                                    </InputGroupAddon>
                                    <InputGroupInput
                                        id="password"
                                        type={showPassword ? "text" : "password"}
                                        value={form.password}
                                        onChange={(e) => setForm((f) => ({ ...f, password: e.target.value }))}
                                        placeholder="Create a password"
                                        autoComplete="new-password"
                                        required
                                        disabled={isRegistering}
                                        className="md:text-base"
                                    />
                                    <InputGroupAddon align="inline-end">
                                        <InputGroupButton size="icon-xs" onClick={() => setShowPassword(!showPassword)}>
                                            {showPassword ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                                        </InputGroupButton>
                                    </InputGroupAddon>
                                </InputGroup>
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

                        <SsoProviders />

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

"use client";

import { useState, useEffect } from "react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import useSWR from "swr";
import useSWRMutation from "swr/mutation";
import { toast } from "sonner";
import { TopBar } from "@/components/top-bar";
import { Button } from "@/components/ui/button";
import {
    Globe,
    Users,
    Lock,
    Compass,
    GitMerge,
    FolderGit2,
    Download,
    Github,
    Link as LinkIcon,
    ArrowRight,
    CheckCircle2,
    AlertCircle,
    Loader2,
} from "lucide-react";
import { useAuth } from "@/hooks/use-auth";
import { postJsonFetcher, validationFetcher } from "@/lib/fetchers";
import { isValidUrl, extractRepoNameFromUrl } from "@/lib/repo-validation";

// SSO Provider icons
function GitLabIcon({ className }: { className?: string }) {
    return (
        <svg
            className={className}
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
        >
            <path
                d="M22.65 14.39L12 22.13 1.35 14.39a.84.84 0 0 1-.3-.94l1.22-3.78 2.44-7.51A.42.42 0 0 1 4.82 2a.43.43 0 0 1 .58 0 .42.42 0 0 1 .11.18l2.44 7.49h8.1l2.44-7.51A.42.42 0 0 1 18.6 2a.43.43 0 0 1 .58 0 .42.42 0 0 1 .11.18l2.44 7.51L23 13.45a.84.84 0 0 1-.35.94z"
                fill="currentColor"
            />
        </svg>
    );
}

function BitbucketIcon({ className }: { className?: string }) {
    return (
        <svg className={className} viewBox="0 0 24 24" fill="none">
            <path d="M2 4h20l-2.5 16H4.5L2 4z" stroke="currentColor" strokeWidth="2" strokeLinejoin="round" />
            <path d="M9 12h6l-1 4h-4l-1-4z" fill="currentColor" />
        </svg>
    );
}

type ImportSource = "url" | "github" | "gitlab" | "bitbucket";
type Visibility = "public" | "internal" | "private";

interface ImportRepoRequest {
    name: string;
    description: string;
    url: string;
    visibility: Visibility;
    username?: string;
    password?: string;
}

interface ImportRepoResponse {
    id: string;
    url: string;
}

export default function ImportRepositoryPage() {
    const router = useRouter();
    const { user } = useAuth();

    const [source, setSource] = useState<ImportSource>("url");
    const [repoUrl, setRepoUrl] = useState("");
    const [repoName, setRepoName] = useState("");
    const [description, setDescription] = useState("");
    const [visibility, setVisibility] = useState<Visibility>("private");
    const [importUsername, setImportUsername] = useState("");
    const [importPassword, setImportPassword] = useState("");

    // Debounced values for the validate endpoint
    const [debouncedName, setDebouncedName] = useState("");
    const [debouncedDescription, setDebouncedDescription] = useState("");

    useEffect(() => {
        const t = setTimeout(() => {
            setDebouncedName(repoName);
            setDebouncedDescription(description);
        }, 400);
        return () => clearTimeout(t);
    }, [repoName, description]);

    // Client-side URL validation (mirrors url::Url::parse behaviour)
    const urlValid = repoUrl.length > 0 && isValidUrl(repoUrl);
    const urlInvalid = repoUrl.length > 0 && !urlValid;

    const handleUrlChange = (url: string) => {
        setRepoUrl(url);

        // Auto-fill repo name from URL when the field is still empty
        if (isValidUrl(url)) {
            const extracted = extractRepoNameFromUrl(url);
            if (extracted && !repoName) {
                setRepoName(extracted);
            }
        }
    };

    // Single validate call covers name rules, reserved names, duplicate check, and description length
    const validateUrl =
        user && debouncedName
            ? `/api/repo/validate?name=${encodeURIComponent(debouncedName)}&description=${encodeURIComponent(debouncedDescription)}`
            : null;

    const { data: validation, isLoading: isValidating } = useSWR(validateUrl, validationFetcher, {
        shouldRetryOnError: false,
        revalidateOnFocus: false,
    });

    const namePending = (!!repoName && repoName !== debouncedName) || isValidating;
    const descPending = description !== debouncedDescription || isValidating;
    const isPending = namePending || descPending;
    const nameValid = !!repoName && !isPending && validation?.valid === true;
    const nameError = !!repoName && !isPending ? (validation?.name ?? null) : null;
    const descriptionError = !isPending ? (validation?.description ?? null) : null;

    const { trigger, isMutating } = useSWRMutation<ImportRepoResponse, Error, string, ImportRepoRequest>(
        "/api/repo/import",
        postJsonFetcher
    );

    const canSubmit = !!user && !!repoName && nameValid && !descriptionError && !isPending && (source !== "url" || urlValid) && !isMutating;

    async function handleSubmit(e: React.FormEvent) {
        e.preventDefault();
        if (!canSubmit || !user) {
            return;
        }

        try {
            await trigger({
                name: repoName,
                description,
                url: repoUrl,
                visibility,
                ...(importUsername ? { username: importUsername } : {}),
                ...(importPassword ? { password: importPassword } : {}),
            });
            router.push(`/${user.username}/${repoName}`);
        } catch (err) {
            toast.error(err instanceof Error ? err.message : "Failed to import repository");
        }
    }

    const sourceOptions = [
        { id: "url" as ImportSource, label: "Clone URL", icon: LinkIcon, desc: "Import from any Git URL" },
        { id: "github" as ImportSource, label: "GitHub", icon: Github, desc: "Connect your GitHub account" },
        { id: "gitlab" as ImportSource, label: "GitLab", icon: GitLabIcon, desc: "Connect your GitLab account" },
        { id: "bitbucket" as ImportSource, label: "Bitbucket", icon: BitbucketIcon, desc: "Connect your Bitbucket account" },
    ];

    return (
        <div className="min-h-screen bg-background flex flex-col">
            <TopBar
                breadcrumb={[{ label: "Import repository" }]}
                navLinks={[
                    { label: "Explore", href: "/explore", icon: <Compass className="h-[18px] w-[18px]" /> },
                    { label: "Merge Requests", href: "#", icon: <GitMerge className="h-[18px] w-[18px]" /> },
                ]}
            />

            <main className="flex-1 flex">
                <aside className="w-64 border-r border-border p-5 hidden lg:block">
                    <div className="space-y-6">
                        <div>
                            <h3 className="text-sm font-medium text-muted-foreground mb-3">Quick actions</h3>
                            <div className="space-y-1">
                                <Link
                                    href="/new"
                                    className="flex items-center gap-3 px-3 py-2 text-sm text-muted-foreground hover:text-foreground hover:bg-accent/30 rounded-md transition-colors"
                                >
                                    <FolderGit2 className="h-4 w-4" />
                                    New repository
                                </Link>
                                <Link
                                    href="#"
                                    className="flex items-center gap-3 px-3 py-2 text-sm text-muted-foreground hover:text-foreground hover:bg-accent/30 rounded-md transition-colors"
                                >
                                    <Users className="h-4 w-4" />
                                    New organization
                                </Link>
                                <Link
                                    href="/import"
                                    className="flex items-center gap-3 px-3 py-2 text-sm bg-accent/50 text-foreground rounded-md"
                                >
                                    <Download className="h-4 w-4" />
                                    Import repository
                                </Link>
                            </div>
                        </div>

                        <div>
                            <h3 className="text-sm font-medium text-muted-foreground mb-3">Recent imports</h3>
                            <div className="space-y-1 text-sm text-muted-foreground">
                                <p className="px-3 py-2">No recent imports</p>
                            </div>
                        </div>
                    </div>
                </aside>

                <div className="flex-1 overflow-auto">
                    <div className="max-w-2xl mx-auto p-8 lg:p-12">
                        <div className="flex items-center gap-3 mb-8">
                            <div className="flex items-center justify-center h-12 w-12 rounded-lg bg-card border border-border">
                                <Download className="h-6 w-6 text-muted-foreground" />
                            </div>
                            <div>
                                <h1 className="text-2xl font-semibold">Import a repository</h1>
                                <p className="text-muted-foreground">Migrate your code from another platform or import from a Git URL.</p>
                            </div>
                        </div>

                        <form onSubmit={handleSubmit} className="space-y-8">
                            <div className="space-y-3">
                                <label className="text-sm font-medium">Import source</label>
                                <div className="grid grid-cols-2 gap-3">
                                    {sourceOptions.map((option) => {
                                        const Icon = option.icon;
                                        return (
                                            <button
                                                key={option.id}
                                                type="button"
                                                onClick={() => setSource(option.id)}
                                                className={`flex items-center gap-3 p-4 rounded-lg border text-left transition-colors ${
                                                    source === option.id
                                                        ? "border-foreground bg-accent/30"
                                                        : "border-border hover:bg-accent/20"
                                                }`}
                                            >
                                                <Icon
                                                    className={`h-5 w-5 shrink-0 ${source === option.id ? "text-foreground" : "text-muted-foreground"}`}
                                                />
                                                <div className="min-w-0">
                                                    <span
                                                        className={`text-sm font-medium block ${source === option.id ? "text-foreground" : "text-muted-foreground"}`}
                                                    >
                                                        {option.label}
                                                    </span>
                                                    <span className="text-xs text-muted-foreground">{option.desc}</span>
                                                </div>
                                            </button>
                                        );
                                    })}
                                </div>
                            </div>

                            {source === "url" ? (
                                <div className="space-y-3">
                                    <label className="text-sm font-medium">
                                        Repository URL <span className="text-red-500">*</span>
                                    </label>
                                    <div className="relative">
                                        <input
                                            type="url"
                                            value={repoUrl}
                                            onChange={(e) => handleUrlChange(e.target.value)}
                                            placeholder="https://github.com/user/repo.git"
                                            className="w-full h-11 px-4 pr-10 bg-card border border-border rounded-md text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
                                        />
                                        <div className="absolute right-3 top-1/2 -translate-y-1/2">
                                            {urlValid && <CheckCircle2 className="h-4 w-4 text-green-500" />}
                                            {urlInvalid && <AlertCircle className="h-4 w-4 text-red-500" />}
                                        </div>
                                    </div>
                                    {urlValid && <p className="text-sm text-green-500">Valid URL</p>}
                                    {urlInvalid && <p className="text-sm text-red-500">Please enter a valid URL</p>}

                                    <div className="pt-4 border-t border-border space-y-3">
                                        <p className="text-sm text-muted-foreground">For private repositories, provide authentication:</p>
                                        <div className="grid grid-cols-2 gap-3">
                                            <input
                                                type="text"
                                                value={importUsername}
                                                onChange={(e) => setImportUsername(e.target.value)}
                                                placeholder="Username (optional)"
                                                className="h-10 px-3 bg-card border border-border rounded-md text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
                                            />
                                            <input
                                                type="password"
                                                value={importPassword}
                                                onChange={(e) => setImportPassword(e.target.value)}
                                                placeholder="Token or password"
                                                className="h-10 px-3 bg-card border border-border rounded-md text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
                                            />
                                        </div>
                                    </div>
                                </div>
                            ) : (
                                <div className="p-8 border border-dashed border-border rounded-lg text-center space-y-4">
                                    <div className="flex justify-center">
                                        {source === "github" && <Github className="h-12 w-12 text-muted-foreground" />}
                                        {source === "gitlab" && <GitLabIcon className="h-12 w-12 text-muted-foreground" />}
                                        {source === "bitbucket" && <BitbucketIcon className="h-12 w-12 text-muted-foreground" />}
                                    </div>
                                    <div>
                                        <p className="font-medium">
                                            Connect your {sourceOptions.find((s) => s.id === source)?.label} account
                                        </p>
                                        <p className="text-sm text-muted-foreground mt-1">Authorize GitArena to access your repositories</p>
                                    </div>
                                    <Button type="button" className="gap-2">
                                        Connect to {sourceOptions.find((s) => s.id === source)?.label}
                                        <ArrowRight className="h-4 w-4" />
                                    </Button>
                                </div>
                            )}

                            <div className="space-y-3">
                                <label className="text-sm font-medium">
                                    Import to <span className="text-red-500">*</span>
                                </label>
                                <div className="flex items-center gap-3">
                                    {/* Owner is always the current user — no org dropdown for now */}
                                    <div className="flex items-center gap-3 h-11 px-4 bg-card border border-border rounded-md">
                                        <div className="flex items-center justify-center h-6 w-6 rounded bg-secondary text-xs font-medium">
                                            {user?.username?.[0]?.toUpperCase() ?? "?"}
                                        </div>
                                        <span>{user?.username ?? "…"}</span>
                                    </div>

                                    <span className="text-2xl text-muted-foreground">/</span>

                                    <div className="relative flex-1">
                                        <input
                                            type="text"
                                            value={repoName}
                                            onChange={(e) => setRepoName(e.target.value)}
                                            placeholder="repository-name"
                                            className="w-full h-11 px-4 pr-10 bg-card border border-border rounded-md text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
                                        />
                                        <div className="absolute right-3 top-1/2 -translate-y-1/2">
                                            {namePending && <Loader2 className="h-4 w-4 text-muted-foreground animate-spin" />}
                                            {nameValid && <CheckCircle2 className="h-4 w-4 text-green-500" />}
                                            {nameError && <AlertCircle className="h-4 w-4 text-red-500" />}
                                        </div>
                                    </div>
                                </div>
                                {nameError && <p className="text-sm text-red-500">{nameError}</p>}
                                {nameValid && user && (
                                    <p className="text-sm text-muted-foreground">
                                        Will be imported to{" "}
                                        <span className="text-foreground font-mono">
                                            {user.username}/{repoName}
                                        </span>
                                    </p>
                                )}
                            </div>

                            <div className="space-y-3">
                                <label className="text-sm font-medium flex items-center gap-2">
                                    Description <span className="text-muted-foreground font-normal">(optional)</span>
                                    {descPending && <Loader2 className="h-3.5 w-3.5 text-muted-foreground animate-spin" />}
                                </label>
                                <textarea
                                    value={description}
                                    onChange={(e) => setDescription(e.target.value)}
                                    placeholder="A short description of your repository"
                                    rows={2}
                                    className="w-full px-4 py-3 bg-card border border-border rounded-md text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring resize-none"
                                />
                                {descriptionError && <p className="text-sm text-red-500">{descriptionError}</p>}
                            </div>

                            <div className="space-y-3">
                                <label className="text-sm font-medium">
                                    Visibility <span className="text-red-500">*</span>
                                </label>
                                <div className="grid grid-cols-3 gap-3">
                                    {[
                                        { value: "public" as Visibility, label: "Public", icon: Globe, desc: "Anyone can see" },
                                        { value: "internal" as Visibility, label: "Internal", icon: Users, desc: "Logged-in users" },
                                        { value: "private" as Visibility, label: "Private", icon: Lock, desc: "Only you and invites" },
                                    ].map((option) => {
                                        const Icon = option.icon;
                                        return (
                                            <button
                                                key={option.value}
                                                type="button"
                                                onClick={() => setVisibility(option.value)}
                                                className={`flex flex-col items-center gap-2 p-4 rounded-lg border transition-colors ${
                                                    visibility === option.value
                                                        ? "border-foreground bg-accent/30"
                                                        : "border-border hover:bg-accent/20"
                                                }`}
                                            >
                                                <Icon
                                                    className={`h-5 w-5 ${visibility === option.value ? "text-foreground" : "text-muted-foreground"}`}
                                                />
                                                <span
                                                    className={`text-sm font-medium ${visibility === option.value ? "text-foreground" : "text-muted-foreground"}`}
                                                >
                                                    {option.label}
                                                </span>
                                                <span className="text-xs text-muted-foreground">{option.desc}</span>
                                            </button>
                                        );
                                    })}
                                </div>
                            </div>

                            <div className="space-y-3">
                                <label className="text-sm font-medium">Options</label>
                                <div
                                    className="flex items-center gap-3 p-4 rounded-lg border border-border text-left w-full opacity-50 cursor-not-allowed"
                                    title="Mirror support is coming soon"
                                >
                                    <div className="flex items-center justify-center h-5 w-5 rounded border-2 border-muted-foreground" />
                                    <div>
                                        <span className="font-medium">Mirror repository</span>
                                        <p className="text-xs text-muted-foreground mt-0.5">
                                            Keep the repository in sync with the source — coming soon
                                        </p>
                                    </div>
                                </div>
                            </div>

                            <div className="pt-4">
                                <Button type="submit" className="w-full h-12 text-base gap-2" disabled={!canSubmit}>
                                    {isMutating ? (
                                        <>
                                            <Loader2 className="h-5 w-5 animate-spin" />
                                            Importing…
                                        </>
                                    ) : (
                                        <>
                                            <Download className="h-5 w-5" />
                                            Import repository
                                        </>
                                    )}
                                </Button>
                            </div>
                        </form>
                    </div>
                </div>
            </main>
        </div>
    );
}

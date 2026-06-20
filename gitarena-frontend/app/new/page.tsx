"use client";

import { Suspense, useState } from "react";
import Link from "next/link";
import { useRouter, useSearchParams } from "next/navigation";
import useSWR from "swr";
import useSWRMutation from "swr/mutation";
import { toast } from "sonner";
import { TopBar } from "@/components/top-bar";
import { useEffect } from "react";
import { Button } from "@/components/ui/button";
import {
    ChevronDown,
    Globe,
    Users,
    Lock,
    FileText,
    GitBranch,
    Scale,
    Compass,
    GitMerge,
    Code,
    FolderGit2,
    Sparkles,
    CheckCircle2,
    AlertCircle,
    Loader2,
    Building2,
    BookOpen,
} from "lucide-react";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { useAuth } from "@/hooks/use-auth";
import { jsonFetcher, postJsonFetcher, validationFetcher } from "@/lib/fetchers";

const licenses = [
    { id: "none", name: "None" },
    { id: "mit", name: "MIT" },
    { id: "apache-2.0", name: "Apache 2.0" },
    { id: "gpl-3.0", name: "GPL 3.0" },
    { id: "bsd-3-clause", name: "BSD 3-Clause" },
];

const gitignoreTemplates = [
    { id: "none", name: "None" },
    { id: "rust", name: "Rust" },
    { id: "node", name: "Node" },
    { id: "python", name: "Python" },
    { id: "go", name: "Go" },
];

type Visibility = "public" | "internal" | "private";

interface UserOrgEntry {
    id: string;
    name: string;
}

interface CreateRepoRequest {
    name: string;
    description: string;
    visibility: Visibility;
    namespace: string;
    readme?: boolean;
    defaultBranch: string;
    license?: string;
    gitignore?: string;
}

interface CreateRepoResponse {
    id: string;
    url: string;
}

export default function NewRepositoryPage() {
    return (
        <Suspense>
            <NewRepositoryForm />
        </Suspense>
    );
}

function NewRepositoryForm() {
    const router = useRouter();
    const searchParams = useSearchParams();
    const { user } = useAuth();

    const { data: userOrgs } = useSWR<UserOrgEntry[]>(user ? `/api/users/${user.username}/orgs` : null, jsonFetcher, {
        shouldRetryOnError: false,
    });

    // Namespace: default to user, or org if passed via query param.
    // Track an explicit override — when null, fall back to the query param or the logged-in user.
    const initialNamespace = searchParams.get("namespace") ?? "";
    const [namespaceOverride, setNamespaceOverride] = useState<string | null>(null);
    const selectedNamespace = ((namespaceOverride ?? initialNamespace) || user?.username) ?? "";

    const namespaceOptions: Array<{ value: string; label: string; isOrg: boolean }> = [
        ...(user ? [{ value: user.username, label: user.username, isOrg: false }] : []),
        ...(userOrgs ?? []).map((org) => ({ value: org.name, label: org.name, isOrg: true })),
    ];

    const [repoName, setRepoName] = useState("");
    const [description, setDescription] = useState("");
    const [visibility, setVisibility] = useState<Visibility>("public");
    const [createReadme, setCreateReadme] = useState(false);
    const [selectedLicense, setSelectedLicense] = useState("none");
    const [selectedGitignore, setSelectedGitignore] = useState("none");
    const [defaultBranch, setDefaultBranch] = useState("main");

    // Debounced values sent to the validate endpoint
    const [debouncedName, setDebouncedName] = useState("");
    const [debouncedDescription, setDebouncedDescription] = useState("");

    useEffect(() => {
        const t = setTimeout(() => {
            setDebouncedName(repoName);
            setDebouncedDescription(description);
        }, 400);
        return () => clearTimeout(t);
    }, [repoName, description]);

    // Single validate call covers name rules, reserved names, duplicate check, and description length
    const validateUrl =
        user && debouncedName
            ? `/api/repo/validate?namespace=${encodeURIComponent(selectedNamespace)}&name=${encodeURIComponent(debouncedName)}&description=${encodeURIComponent(debouncedDescription)}`
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

    const { trigger, isMutating } = useSWRMutation<CreateRepoResponse, Error, string, CreateRepoRequest>("/api/repo", postJsonFetcher);

    const canSubmit = !!user && !!repoName && !!selectedNamespace && nameValid && !descriptionError && !isPending && !isMutating;

    async function handleSubmit(e: React.FormEvent) {
        e.preventDefault();
        if (!canSubmit || !user) {
            return;
        }

        try {
            await trigger({
                name: repoName,
                description,
                visibility,
                namespace: selectedNamespace,
                defaultBranch,
                ...(createReadme ? { readme: true } : {}),
                ...(selectedLicense !== "none" ? { license: selectedLicense } : {}),
                ...(selectedGitignore !== "none" ? { gitignore: selectedGitignore } : {}),
            });
            router.push(`/${selectedNamespace}/${repoName}`);
        } catch (err) {
            toast.error(err instanceof Error ? err.message : "Failed to create repository");
        }
    }

    const selectedOption = namespaceOptions.find((o) => o.value === selectedNamespace);

    return (
        <div className="min-h-screen bg-background flex flex-col">
            <TopBar
                breadcrumb={[{ label: "New repository" }]}
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
                                    className="flex items-center gap-3 px-3 py-2 text-sm bg-accent/50 text-foreground rounded-md"
                                >
                                    <FolderGit2 className="h-4 w-4" />
                                    New repository
                                </Link>
                                <Link
                                    href="/orgs/new"
                                    className="flex items-center gap-3 px-3 py-2 text-sm text-muted-foreground hover:text-foreground hover:bg-accent/30 rounded-md transition-colors"
                                >
                                    <Users className="h-4 w-4" />
                                    New organization
                                </Link>
                                <Link
                                    href="/import"
                                    className="flex items-center gap-3 px-3 py-2 text-sm text-muted-foreground hover:text-foreground hover:bg-accent/30 rounded-md transition-colors"
                                >
                                    <Code className="h-4 w-4" />
                                    Import repository
                                </Link>
                            </div>
                        </div>

                        <div>
                            <h3 className="text-sm font-medium text-muted-foreground mb-3">Recent repositories</h3>
                            <div className="space-y-1 text-sm text-muted-foreground">
                                <p className="px-3 py-2">No recent repositories</p>
                            </div>
                        </div>
                    </div>
                </aside>

                <div className="flex-1 overflow-auto">
                    <div className="max-w-2xl mx-auto p-8 lg:p-12">
                        <div className="flex items-center gap-3 mb-8">
                            <div className="flex items-center justify-center h-12 w-12 rounded-lg bg-card border border-border">
                                <Sparkles className="h-6 w-6 text-muted-foreground" />
                            </div>
                            <div>
                                <h1 className="text-2xl font-semibold">Create a new repository</h1>
                                <p className="text-muted-foreground">
                                    A repository contains all project files, including the revision history.
                                </p>
                            </div>
                        </div>

                        <form onSubmit={handleSubmit} className="space-y-8">
                            <div className="space-y-3">
                                <label className="text-sm font-medium">
                                    Owner <span className="text-red-500">*</span>
                                </label>
                                <div className="flex items-center gap-3">
                                    {namespaceOptions.length > 1 ? (
                                        <DropdownMenu>
                                            <DropdownMenuTrigger asChild>
                                                <button className="flex items-center gap-3 h-11 px-4 bg-card border border-border rounded-md hover:bg-accent/50 transition-colors">
                                                    <div className="flex items-center justify-center h-6 w-6 rounded bg-secondary text-xs font-medium shrink-0">
                                                        {selectedOption?.isOrg ? (
                                                            <Building2 className="h-3.5 w-3.5" />
                                                        ) : (
                                                            (selectedOption?.label?.[0]?.toUpperCase() ?? "?")
                                                        )}
                                                    </div>
                                                    <span>{selectedNamespace || "Select owner"}</span>
                                                    <ChevronDown className="h-3.5 w-3.5 text-muted-foreground ml-1" />
                                                </button>
                                            </DropdownMenuTrigger>
                                            <DropdownMenuContent align="start" className="w-52">
                                                {namespaceOptions.map((opt) => (
                                                    <DropdownMenuItem key={opt.value} onClick={() => setNamespaceOverride(opt.value)}>
                                                        <div className="flex items-center gap-2">
                                                            <div className="flex items-center justify-center h-5 w-5 rounded bg-secondary text-xs font-medium shrink-0">
                                                                {opt.isOrg ? <Building2 className="h-3 w-3" /> : opt.label[0].toUpperCase()}
                                                            </div>
                                                            {opt.label}
                                                            {opt.isOrg && (
                                                                <span className="ml-auto text-[10px] text-muted-foreground">org</span>
                                                            )}
                                                        </div>
                                                    </DropdownMenuItem>
                                                ))}
                                            </DropdownMenuContent>
                                        </DropdownMenu>
                                    ) : (
                                        <div className="flex items-center gap-3 h-11 px-4 bg-card border border-border rounded-md">
                                            <div className="flex items-center justify-center h-6 w-6 rounded bg-secondary text-xs font-medium">
                                                {user?.username?.[0]?.toUpperCase() ?? "?"}
                                            </div>
                                            <span>{user?.username ?? "…"}</span>
                                        </div>
                                    )}

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
                                {nameValid && selectedNamespace && (
                                    <p className="text-sm text-muted-foreground">
                                        Your repository will be available at{" "}
                                        <span className="text-foreground font-mono">
                                            {selectedNamespace}/{repoName}
                                        </span>
                                    </p>
                                )}
                                {nameValid && selectedNamespace && repoName === selectedNamespace && (
                                    <div className="flex items-start gap-3 p-3 rounded-md bg-blue-500/10 border border-blue-500/30">
                                        <BookOpen className="h-4 w-4 text-blue-500 mt-0.5 shrink-0" />
                                        <p className="text-sm text-blue-600 dark:text-blue-400">
                                            <span className="font-medium">
                                                {selectedNamespace}/{repoName}
                                            </span>{" "}
                                            is a special repository — its README will appear on your profile page.
                                        </p>
                                    </div>
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
                                <label className="text-sm font-medium">Initialize repository</label>
                                <div className="grid grid-cols-2 gap-4">
                                    <button
                                        type="button"
                                        onClick={() => setCreateReadme(!createReadme)}
                                        className={`flex items-center gap-3 p-4 rounded-lg border text-left transition-colors ${
                                            createReadme ? "border-foreground bg-accent/30" : "border-border hover:bg-accent/20"
                                        }`}
                                    >
                                        <div
                                            className={`flex items-center justify-center h-5 w-5 rounded border-2 transition-colors ${
                                                createReadme ? "border-foreground bg-foreground" : "border-muted-foreground"
                                            }`}
                                        >
                                            {createReadme && (
                                                <svg
                                                    className="h-3 w-3 text-background"
                                                    viewBox="0 0 12 12"
                                                    fill="none"
                                                    stroke="currentColor"
                                                    strokeWidth="2"
                                                >
                                                    <path d="M2 6l3 3 5-6" />
                                                </svg>
                                            )}
                                        </div>
                                        <div>
                                            <div className="flex items-center gap-2">
                                                <FileText className="h-4 w-4 text-muted-foreground" />
                                                <span className="font-medium">README</span>
                                            </div>
                                            <p className="text-xs text-muted-foreground mt-0.5">Add a README file</p>
                                        </div>
                                    </button>

                                    <div className="p-4 rounded-lg border border-border space-y-2">
                                        <div className="flex items-center gap-2">
                                            <GitBranch className="h-4 w-4 text-muted-foreground" />
                                            <span className="font-medium">Default branch</span>
                                        </div>
                                        <input
                                            type="text"
                                            value={defaultBranch}
                                            onChange={(e) => setDefaultBranch(e.target.value)}
                                            className="w-full h-9 px-3 bg-background border border-border rounded-md text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
                                        />
                                    </div>

                                    <div className="p-4 rounded-lg border border-border space-y-2">
                                        <div className="flex items-center gap-2">
                                            <Scale className="h-4 w-4 text-muted-foreground" />
                                            <span className="font-medium">License</span>
                                        </div>
                                        <DropdownMenu>
                                            <DropdownMenuTrigger asChild>
                                                <button className="flex items-center justify-between w-full h-9 px-3 bg-background border border-border rounded-md text-sm hover:bg-accent/50 transition-colors">
                                                    <span className={selectedLicense === "none" ? "text-muted-foreground" : ""}>
                                                        {licenses.find((l) => l.id === selectedLicense)?.name}
                                                    </span>
                                                    <ChevronDown className="h-3.5 w-3.5 text-muted-foreground" />
                                                </button>
                                            </DropdownMenuTrigger>
                                            <DropdownMenuContent align="start" className="w-48">
                                                {licenses.map((license) => (
                                                    <DropdownMenuItem key={license.id} onClick={() => setSelectedLicense(license.id)}>
                                                        {license.name}
                                                    </DropdownMenuItem>
                                                ))}
                                            </DropdownMenuContent>
                                        </DropdownMenu>
                                    </div>

                                    <div className="p-4 rounded-lg border border-border space-y-2">
                                        <div className="flex items-center gap-2">
                                            <FileText className="h-4 w-4 text-muted-foreground" />
                                            <span className="font-medium">.gitignore</span>
                                        </div>
                                        <DropdownMenu>
                                            <DropdownMenuTrigger asChild>
                                                <button className="flex items-center justify-between w-full h-9 px-3 bg-background border border-border rounded-md text-sm hover:bg-accent/50 transition-colors">
                                                    <span className={selectedGitignore === "none" ? "text-muted-foreground" : ""}>
                                                        {gitignoreTemplates.find((t) => t.id === selectedGitignore)?.name}
                                                    </span>
                                                    <ChevronDown className="h-3.5 w-3.5 text-muted-foreground" />
                                                </button>
                                            </DropdownMenuTrigger>
                                            <DropdownMenuContent align="start" className="w-48">
                                                {gitignoreTemplates.map((template) => (
                                                    <DropdownMenuItem key={template.id} onClick={() => setSelectedGitignore(template.id)}>
                                                        {template.name}
                                                    </DropdownMenuItem>
                                                ))}
                                            </DropdownMenuContent>
                                        </DropdownMenu>
                                    </div>
                                </div>
                            </div>

                            <div className="pt-4">
                                <Button type="submit" className="w-full h-12 text-base" disabled={!canSubmit}>
                                    {isMutating ? (
                                        <>
                                            <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                                            Creating…
                                        </>
                                    ) : (
                                        "Create repository"
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

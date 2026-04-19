"use client";

import { useState } from "react";
import Link from "next/link";
import { TopBar } from "@/components/top-bar";
import { Button } from "@/components/ui/button";
import {
    ChevronDown,
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
    Loader2,
    CheckCircle2,
    AlertCircle,
} from "lucide-react";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";

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

const currentUser = {
    name: "Mari",
    username: "mellowagain",
    orgs: [
        { name: "mellowagain", type: "user" },
        { name: "gitarena", type: "org" },
        { name: "acme-corp", type: "org" },
    ],
};

type ImportSource = "url" | "github" | "gitlab" | "bitbucket";
type Visibility = "public" | "internal" | "private";
type ImportStatus = "idle" | "validating" | "valid" | "invalid";

export default function ImportRepositoryPage() {
    const [source, setSource] = useState<ImportSource>("url");
    const [repoUrl, setRepoUrl] = useState("");
    const [namespace, setNamespace] = useState(currentUser.orgs[0].name);
    const [repoName, setRepoName] = useState("");
    const [visibility, setVisibility] = useState<Visibility>("private");
    const [importStatus, setImportStatus] = useState<ImportStatus>("idle");
    const [mirror, setMirror] = useState(false);

    const baseUrl = "git.mari.zip";

    const handleUrlChange = (url: string) => {
        setRepoUrl(url);
        if (url.length > 10) {
            setImportStatus("validating");
            setTimeout(() => {
                if (url.includes("github.com") || url.includes("gitlab.com") || url.includes("bitbucket.org") || url.endsWith(".git")) {
                    setImportStatus("valid");
                    // Auto-extract repo name from URL
                    const parts = url.split("/");
                    const name = parts[parts.length - 1]?.replace(".git", "") || "";
                    if (name && !repoName) {
                        setRepoName(name);
                    }
                } else {
                    setImportStatus("invalid");
                }
            }, 800);
        } else {
            setImportStatus("idle");
        }
    };

    const sourceOptions = [
        { id: "url" as ImportSource, label: "Clone URL", icon: LinkIcon, desc: "Import from any Git URL" },
        { id: "github" as ImportSource, label: "GitHub", icon: Github, desc: "Connect your GitHub account" },
        { id: "gitlab" as ImportSource, label: "GitLab", icon: GitLabIcon, desc: "Connect your GitLab account" },
        { id: "bitbucket" as ImportSource, label: "Bitbucket", icon: BitbucketIcon, desc: "Connect your Bitbucket account" },
    ];

    return (
        <div className="min-h-screen bg-background flex flex-col">
            <TopBar
                breadcrumb={[{ label: "Mirror repository" }]}
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

                        <div className="space-y-8">
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
                                            {importStatus === "validating" && (
                                                <Loader2 className="h-4 w-4 text-muted-foreground animate-spin" />
                                            )}
                                            {importStatus === "valid" && <CheckCircle2 className="h-4 w-4 text-green-500" />}
                                            {importStatus === "invalid" && <AlertCircle className="h-4 w-4 text-red-500" />}
                                        </div>
                                    </div>
                                    {importStatus === "valid" && <p className="text-sm text-green-500">Repository found and accessible</p>}
                                    {importStatus === "invalid" && (
                                        <p className="text-sm text-red-500">
                                            Could not access repository. Check the URL or ensure it&apos;s public.
                                        </p>
                                    )}

                                    <div className="pt-4 border-t border-border space-y-3">
                                        <p className="text-sm text-muted-foreground">For private repositories, provide authentication:</p>
                                        <div className="grid grid-cols-2 gap-3">
                                            <input
                                                type="text"
                                                placeholder="Username (optional)"
                                                className="h-10 px-3 bg-card border border-border rounded-md text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
                                            />
                                            <input
                                                type="password"
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
                                    <Button className="gap-2">
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
                                    <DropdownMenu>
                                        <DropdownMenuTrigger asChild>
                                            <button className="flex items-center gap-3 h-11 px-4 bg-card border border-border rounded-md hover:bg-accent/50 transition-colors">
                                                <div className="flex items-center justify-center h-6 w-6 rounded bg-secondary text-xs font-medium">
                                                    {namespace[0].toUpperCase()}
                                                </div>
                                                <span>{namespace}</span>
                                                <ChevronDown className="h-4 w-4 text-muted-foreground" />
                                            </button>
                                        </DropdownMenuTrigger>
                                        <DropdownMenuContent align="start" className="w-56">
                                            {currentUser.orgs.map((org) => (
                                                <DropdownMenuItem
                                                    key={org.name}
                                                    onClick={() => setNamespace(org.name)}
                                                    className="flex items-center gap-3"
                                                >
                                                    <div className="flex items-center justify-center h-6 w-6 rounded bg-secondary text-xs font-medium">
                                                        {org.name[0].toUpperCase()}
                                                    </div>
                                                    {org.name}
                                                    {org.type === "user" && (
                                                        <span className="text-xs text-muted-foreground ml-auto">you</span>
                                                    )}
                                                </DropdownMenuItem>
                                            ))}
                                        </DropdownMenuContent>
                                    </DropdownMenu>

                                    <span className="text-2xl text-muted-foreground">/</span>

                                    <input
                                        type="text"
                                        value={repoName}
                                        onChange={(e) => setRepoName(e.target.value)}
                                        placeholder="repository-name"
                                        className="flex-1 h-11 px-4 bg-card border border-border rounded-md text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
                                    />
                                </div>
                                {repoName && (
                                    <p className="text-sm text-muted-foreground">
                                        Will be imported to{" "}
                                        <span className="text-foreground font-mono">
                                            {baseUrl}/{namespace}/{repoName}
                                        </span>
                                    </p>
                                )}
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
                                <button
                                    type="button"
                                    onClick={() => setMirror(!mirror)}
                                    className={`flex items-center gap-3 p-4 rounded-lg border text-left w-full transition-colors ${
                                        mirror ? "border-foreground bg-accent/30" : "border-border hover:bg-accent/20"
                                    }`}
                                >
                                    <div
                                        className={`flex items-center justify-center h-5 w-5 rounded border-2 transition-colors ${
                                            mirror ? "border-foreground bg-foreground" : "border-muted-foreground"
                                        }`}
                                    >
                                        {mirror && (
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
                                        <span className="font-medium">Mirror repository</span>
                                        <p className="text-xs text-muted-foreground mt-0.5">
                                            Keep the repository in sync with the source. Changes will be pulled automatically.
                                        </p>
                                    </div>
                                </button>
                            </div>

                            <div className="pt-4">
                                <Button
                                    type="submit"
                                    className="w-full h-12 text-base gap-2"
                                    disabled={!repoName || (source === "url" && importStatus !== "valid")}
                                >
                                    <Download className="h-5 w-5" />
                                    Import repository
                                </Button>
                            </div>
                        </div>
                    </div>
                </div>
            </main>
        </div>
    );
}

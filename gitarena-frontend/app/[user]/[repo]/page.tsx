"use client";

import { FileText, AlertCircle, GitMerge, ExternalLink, Code, BookOpen, Edit3, WrapText } from "lucide-react";
import { use, useState } from "react";
import { Button } from "@/components/ui/button";
import { TopBar } from "@/components/top-bar";
import { RepoFileSidebar, RepoFileSidebarSkeleton } from "@/components/repo-file-sidebar";
import { RepoSidebar, RepoSidebarSkeleton } from "@/components/repo-sidebar";
import useSWR from "swr";
import { ErrorDisplay } from "@/components/error-display";
import { ReadmeView, isMarkdown } from "@/components/readme-view";
import { CodeBlock } from "@/components/code-block";

// Mock data
const repoData = {
    org: "mellowagain",
    name: "test",
    description: "A lightweight git hosting solution built for speed and simplicity.",
    projectId: 1,
    size: "1.81 kB",
    stars: 12,
    forks: 3,
    watchers: 5,
    license: "MIT",
    websiteUrl: "https://gitarena.dev",
    visibility: "public" as "public" | "internal" | "private",
    defaultBranch: "main",
    branches: ["main", "develop", "feature/auth"],
    topics: ["git", "self-hosted", "rust"],
    createdAt: "Jan 15, 2024",
    languages: [
        { name: "Rust", percentage: 68.4, color: "#dea584" },
        { name: "TOML", percentage: 18.2, color: "#9c4221" },
        { name: "Shell", percentage: 13.4, color: "#89e051" },
    ],
    latestCommit: {
        hash: "9bf39d9",
        message: "init",
        author: "Mari",
        avatarUrl: null,
        date: "19 hours ago",
        totalCommits: 1,
        ciStatus: "passed" as "pending" | "passed" | "failed" | "cancelled",
    },
    latestRelease: {
        tag: "v0.1.0",
        name: "Initial Release",
        date: "2 days ago",
    },
    contributors: [
        { name: "Mari", commits: 24, avatarUrl: null },
        { name: "Alex", commits: 12, avatarUrl: null },
        { name: "Jordan", commits: 8, avatarUrl: null },
    ],
};

interface RepoMetadata {
    id: number;

    owner: number;
    name: string;
    description: string;

    visibility: string;
    defaultBranch: string;

    license?: string;
    languages: Record<string, number>;

    forkedFrom?: number;
    mirroredFrom?: string;

    archived: boolean;
    disabled: boolean;
}

function RepoTopBar({ user, repo }: { user: string; repo: string }) {
    return (
        <TopBar
            breadcrumb={[{ label: user, href: `/${user}` }, { label: repo }]}
            search={{ placeholder: "Search files, commits, issues..." }}
            navLinks={[
                {
                    label: "Code",
                    href: `/${user}/${repo}`,
                    icon: <Code className="h-[18px] w-[18px]" />,
                    active: true,
                },
                {
                    label: "Issues",
                    href: `/${user}/${repo}/issues`,
                    icon: <AlertCircle className="h-[18px] w-[18px]" />,
                },
                {
                    label: "Merge Requests",
                    href: `/${user}/${repo}/merge-requests`,
                    icon: <GitMerge className="h-[18px] w-[18px]" />,
                },
            ]}
        />
    );
}

export default function RepoPage({ params }: { params: Promise<{ user: string; repo: string }> }) {
    const { user, repo } = use(params);

    const [selectedFile, setSelectedFile] = useState<string | null>(null);
    const [showReadmeSource, setShowReadmeSource] = useState(false);
    const [wrapLines, setWrapLines] = useState(false);
    const [branch, setBranch] = useState<string | null>(null);

    const { data, error, isLoading } = useSWR<RepoMetadata>(`http://localhost:8080/api/repos/${user}/${repo}`);

    const effectiveBranch = branch ?? data?.defaultBranch ?? null;

    const { data: readmeData } = useSWR<{ file_name: string; content: string }>(
        effectiveBranch ? `http://localhost:8080/api/repo/${user}/${repo}/tree/${effectiveBranch}/readme` : null
    );

    const displayFile = selectedFile ?? readmeData?.file_name ?? null;

    if (isLoading) {
        return <RepoPageSkeleton user={user} repo={repo} />;
    }

    if (error || !data) {
        return <ErrorDisplay failed={"repo"} error={error} />;
    }

    return (
        <div className="min-h-screen bg-background flex flex-col">
            <RepoTopBar user={user} repo={repo} />

            <div className="flex flex-1 overflow-hidden">
                <RepoFileSidebar
                    user={user}
                    repo={repo}
                    selectedFile={selectedFile}
                    setSelectedFile={setSelectedFile}
                    defaultBranch={data.defaultBranch}
                    onBranchChange={setBranch}
                    branches={repoData.branches}
                    latestCommit={repoData.latestCommit}
                />

                <main className="flex-1 flex flex-col min-w-0 overflow-hidden">
                    <div className="flex items-center justify-between px-5 py-3 border-b border-border shrink-0">
                        <div className="flex items-center gap-2.5">
                            <FileText className="h-[18px] w-[18px] text-muted-foreground" />
                            <span className="font-medium">{displayFile ?? "README"}</span>
                            {((displayFile && !isMarkdown(displayFile)) || showReadmeSource) && (
                                <span className="text-sm text-muted-foreground">2.4 KB</span>
                            )}
                        </div>
                        <div className="flex items-center gap-2">
                            {((displayFile && !isMarkdown(displayFile)) || showReadmeSource) && (
                                <div className="flex items-center gap-1">
                                    <Button
                                        variant="ghost"
                                        size="sm"
                                        className="h-8 px-3 gap-2 text-sm text-muted-foreground hover:text-foreground"
                                    >
                                        <Edit3 className="h-3.5 w-3.5" />
                                        Edit
                                    </Button>
                                    <Button
                                        variant="ghost"
                                        size="sm"
                                        onClick={() => setWrapLines((w) => !w)}
                                        className={`h-8 px-3 gap-2 text-sm hover:text-foreground ${wrapLines ? "text-foreground" : "text-muted-foreground"}`}
                                    >
                                        <WrapText className="h-3.5 w-3.5" />
                                        Wrap
                                    </Button>
                                    <Button
                                        variant="ghost"
                                        size="sm"
                                        className="h-8 px-3 gap-2 text-sm text-muted-foreground hover:text-foreground"
                                    >
                                        <ExternalLink className="h-3.5 w-3.5" />
                                        Raw
                                    </Button>
                                </div>
                            )}
                            {displayFile && isMarkdown(displayFile) && (
                                <div className="flex items-center gap-1 p-0.5 bg-secondary rounded-md">
                                    <button
                                        onClick={() => setShowReadmeSource(false)}
                                        className={`flex items-center gap-2 px-3 py-1.5 text-sm rounded transition-colors ${
                                            !showReadmeSource
                                                ? "bg-background text-foreground"
                                                : "text-muted-foreground hover:text-foreground"
                                        }`}
                                    >
                                        <BookOpen className="h-3.5 w-3.5" />
                                        Preview
                                    </button>
                                    <button
                                        onClick={() => setShowReadmeSource(true)}
                                        className={`flex items-center gap-2 px-3 py-1.5 text-sm rounded transition-colors ${
                                            showReadmeSource
                                                ? "bg-background text-foreground"
                                                : "text-muted-foreground hover:text-foreground"
                                        }`}
                                    >
                                        <Code className="h-3.5 w-3.5" />
                                        Code
                                    </button>
                                </div>
                            )}
                        </div>
                    </div>

                    <div className="flex-1 overflow-auto">
                        {displayFile && isMarkdown(displayFile) && readmeData && (
                            <ReadmeView
                                content={readmeData.content}
                                fileName={readmeData.file_name}
                                showSource={showReadmeSource}
                                wrapLines={wrapLines}
                            />
                        )}
                        {displayFile && !isMarkdown(displayFile) && (
                            <CodeBlock user={user} repo={repo} branch={effectiveBranch} filename={displayFile} wrapLines={wrapLines} />
                        )}
                    </div>
                </main>

                <RepoSidebar
                    user={user}
                    repo={repo}
                    description={data.description}
                    projectId={data.id}
                    license={data.license}
                    //websiteUrl="idk"
                    //createdAt={"creation date"}
                    topics={[]}
                    languages={data.languages}
                    //latestRelease={repoData.latestRelease}
                    //contributors={repoData.contributors}
                />
            </div>
        </div>
    );
}

export function RepoPageSkeleton({ user, repo }: { user: string; repo: string }) {
    return (
        <div className="min-h-screen bg-background flex flex-col">
            <RepoTopBar user={user} repo={repo} />

            <div className="flex flex-1 overflow-hidden">
                <RepoFileSidebarSkeleton />

                {/* Main content */}
                <main className="flex-1 flex flex-col min-w-0 overflow-hidden animate-pulse">
                    <div className="flex items-center justify-between px-5 py-3 border-b border-border shrink-0">
                        <div className="flex items-center gap-2.5">
                            <div className="h-4 w-4 rounded bg-accent shrink-0" />
                            <div className="h-3.5 w-28 rounded bg-accent" />
                        </div>
                        <div className="h-7 w-32 rounded bg-accent" />
                    </div>
                    <div className="flex-1 overflow-hidden p-0">
                        {[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15].map((i) => (
                            <div key={i} className="flex items-center py-0.5">
                                <div className="w-14 shrink-0 flex justify-end pr-4">
                                    <div className="h-3 w-5 rounded bg-accent" />
                                </div>
                                <div className="h-3 rounded bg-accent" style={{ width: `${20 + ((i * 23 + 7) % 60)}%` }} />
                            </div>
                        ))}
                    </div>
                </main>

                <RepoSidebarSkeleton />
            </div>
        </div>
    );
}

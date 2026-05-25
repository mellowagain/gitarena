"use client";

import {
    FileText,
    AlertCircle,
    GitMerge,
    ExternalLink,
    Code,
    BookOpen,
    WrapText,
    MoreHorizontal,
    History,
    GitBranch,
    GitCommitHorizontal,
    ChevronDown,
} from "lucide-react";
import { use, useState } from "react";
import Link from "next/link";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { TopBar } from "@/components/top-bar";
import { RepoFileSidebar, RepoFileSidebarSkeleton } from "@/components/repo-file-sidebar";
import { RepoSidebar, RepoSidebarSkeleton } from "@/components/repo-sidebar";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import useSWR from "swr";
import { ErrorDisplay } from "@/components/error-display";
import { isMarkdown } from "@/components/markdown-renderer";
import { FileContent, type FileCommit } from "@/components/file-content";
import prettyBytes from "pretty-bytes";
import { formatDistanceToNowStrict } from "date-fns";
import { shortLocale } from "@/lib/utils";
import { useInstanceConfig } from "@/components/instance-config-provider";

export interface RepoMetadata {
    id: string;

    owner: string;
    name: string;
    description: string;

    visibility: string;
    defaultBranch: string;

    empty: boolean;
    readme?: string;
    license?: string;
    languages: Record<string, number>;

    forkedFrom?: string;
    mirroredFrom?: string;

    archived: boolean;
    disabled: boolean;
}

function RepoTopBar({ user, repo }: { user: string; repo: string }) {
    return (
        <TopBar
            breadcrumb={[{ label: user, href: `/${user}` }, { label: repo }]}
            search={{
                placeholder: "Search files, commits, issues...",
                scope: { label: `${user}/${repo}`, prefix: `repo:"${user}/${repo}"` },
            }}
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
                //{
                //    label: "Merge Requests",
                //    href: `/${user}/${repo}/merge-requests`,
                //    icon: <GitMerge className="h-[18px] w-[18px]" />,
                //},
            ]}
        />
    );
}

export default function RepoPage({ params }: { params: Promise<{ user: string; repo: string }> }) {
    const { user, repo } = use(params);
    const { data, error, isLoading } = useSWR<RepoMetadata>(`/api/repos/${user}/${repo}`);

    if (isLoading) {
        return <RepoPageSkeleton user={user} repo={repo} />;
    }

    if (error || !data) {
        return <ErrorDisplay failed={"repo"} error={error} />;
    }

    return data.empty ? (
        <EmptyRepoContent user={user} repo={repo} meta={data} />
    ) : (
        <RepoPageContent user={user} repo={repo} meta={data} defaultFile={data.readme} />
    );
}

function EmptyRepoContent({ user, repo, meta }: { user: string; repo: string; meta: RepoMetadata }) {
    const instanceConfig = useInstanceConfig();
    const [protocol, setProtocol] = useState<"https" | "ssh">("https");

    const apiUrl = process.env.NEXT_PUBLIC_API_URL ?? "";
    const host = apiUrl.replace(/^https?:\/\//, "");
    const sshEnabled = instanceConfig?.sshPort != null;
    const sshCloneUrl =
        instanceConfig?.sshPort === 22
            ? `git@${host}:${user}/${repo}.git`
            : `ssh://git@${host}:${instanceConfig?.sshPort}/${user}/${repo}.git`;
    const cloneUrl = protocol === "https" ? `${apiUrl}/${user}/${repo}.git` : sshCloneUrl;

    const branch = meta.defaultBranch;

    return (
        <div className="min-h-screen bg-background flex flex-col">
            <RepoTopBar user={user} repo={repo} />

            <div className="flex flex-1 overflow-hidden">
                <div className="flex flex-1 items-center justify-center">
                    <div className="flex flex-col gap-5 w-full max-w-2xl px-6">
                        <div className="flex flex-col items-center gap-3 text-center">
                            <GitBranch className="h-8 w-8 text-muted-foreground shrink-0" />
                            <div>
                                <h2 className="text-xl font-semibold">Repository is empty</h2>
                                <p className="text-sm text-muted-foreground mt-0.5">Get started by pushing your code.</p>
                            </div>
                        </div>

                        <Tabs defaultValue="create">
                            <div className="flex items-center justify-between mb-3">
                                <TabsList>
                                    <TabsTrigger value="create">Create new repository</TabsTrigger>
                                    <TabsTrigger value="push">Push existing repository</TabsTrigger>
                                </TabsList>

                                <DropdownMenu>
                                    <DropdownMenuTrigger asChild>
                                        <Button
                                            variant="ghost"
                                            size="sm"
                                            className="h-7 px-2 gap-1 text-xs uppercase tracking-wider text-muted-foreground hover:text-foreground"
                                        >
                                            {protocol}
                                            <ChevronDown className="h-3 w-3" />
                                        </Button>
                                    </DropdownMenuTrigger>
                                    <DropdownMenuContent align="end">
                                        <DropdownMenuItem onClick={() => setProtocol("https")}>HTTPS</DropdownMenuItem>
                                        {sshEnabled && <DropdownMenuItem onClick={() => setProtocol("ssh")}>SSH</DropdownMenuItem>}
                                    </DropdownMenuContent>
                                </DropdownMenu>
                            </div>

                            <TabsContent value="create">
                                <div className="rounded-md border border-border bg-muted/50 p-4 font-mono text-sm leading-relaxed space-y-0.5">
                                    <div>echo &quot;# {repo}&quot; &gt;&gt; README.md</div>
                                    <div>git init</div>
                                    <div>git add README.md</div>
                                    <div>git commit -m &quot;initial commit&quot;</div>
                                    <div>git branch -M {branch}</div>
                                    <div>git remote add origin {cloneUrl}</div>
                                    <div>git push -u origin {branch}</div>
                                </div>
                            </TabsContent>

                            <TabsContent value="push">
                                <div className="rounded-md border border-border bg-muted/50 p-4 font-mono text-sm leading-relaxed space-y-0.5">
                                    <div>git remote add origin {cloneUrl}</div>
                                    <div>git branch -M {branch}</div>
                                    <div>git push -u origin {branch}</div>
                                </div>
                            </TabsContent>
                        </Tabs>
                    </div>
                </div>

                <RepoSidebar
                    user={user}
                    repo={repo}
                    description={meta.description}
                    projectId={meta.id}
                    license={meta.license}
                    topics={[]}
                    languages={meta.languages}
                />
            </div>
        </div>
    );
}

function RepoPageContent({ user, repo, meta, defaultFile }: { user: string; repo: string; meta: RepoMetadata; defaultFile?: string }) {
    const [selectedFile, setSelectedFile] = useState<string | null>(defaultFile ?? null);
    const [showSource, setShowSource] = useState(false);
    const [showBlame, setShowBlame] = useState(false);
    const [wrapLines, setWrapLines] = useState(false);
    const [branch, setBranch] = useState(meta.defaultBranch);
    const [fileSize, setFileSize] = useState<number | null>(null);
    const [fileCommit, setFileCommit] = useState<FileCommit | null>(null);
    const [isBinary, setIsBinary] = useState(false);

    function handleSelectFile(file: string | null) {
        setSelectedFile(file);
        setFileSize(null);
        setFileCommit(null);
        setShowBlame(false);
        setIsBinary(false);
    }

    return (
        <div className="min-h-screen bg-background flex flex-col">
            <RepoTopBar user={user} repo={repo} />

            <div className="flex flex-1 overflow-hidden">
                <RepoFileSidebar
                    user={user}
                    repo={repo}
                    selectedFile={selectedFile}
                    setSelectedFile={handleSelectFile}
                    branch={branch}
                    onBranchChange={setBranch}
                    defaultBranch={meta.defaultBranch}
                />

                <main className="flex-1 flex flex-col min-w-0 overflow-hidden">
                    {selectedFile && (
                        <>
                            <div className="flex items-center justify-between px-5 py-3 border-b border-border shrink-0">
                                <div className="flex items-center gap-2.5 min-w-0">
                                    <FileText className="h-[18px] w-[18px] text-muted-foreground shrink-0" />
                                    <div className="flex items-center gap-2 shrink-0 group/filename peer/filename">
                                        <span className="font-medium">
                                            <span className="group-hover/filename:hidden">{selectedFile.split("/").pop()}</span>
                                            <span className="hidden group-hover/filename:inline text-muted-foreground">{selectedFile}</span>
                                        </span>
                                        {(!isMarkdown(selectedFile) || showSource) && fileSize !== null && (
                                            <span className="text-sm text-muted-foreground">{prettyBytes(fileSize)}</span>
                                        )}
                                    </div>
                                    {fileCommit && (
                                        <div className="flex flex-col border-l border-border pl-3 ml-1 min-w-0 peer-hover/filename:invisible">
                                            <div className="flex items-center gap-1.5">
                                                {(() => {
                                                    const lines = fileCommit.message.split("\n").filter((l) => l.trim() !== "");
                                                    const firstLine = lines[0] ?? "";
                                                    const hasMore = lines.length > 1;
                                                    const msg = <span className="text-sm text-foreground truncate">{firstLine}</span>;
                                                    return hasMore ? (
                                                        <Tooltip>
                                                            <TooltipTrigger asChild>
                                                                <span className="flex items-center gap-1 truncate cursor-default">
                                                                    <span className="text-sm text-foreground truncate">{firstLine}</span>
                                                                    <MoreHorizontal className="h-3.5 w-3.5 shrink-0 text-muted-foreground/50" />
                                                                </span>
                                                            </TooltipTrigger>
                                                            <TooltipContent className="max-w-sm whitespace-pre-wrap">
                                                                {fileCommit.message.trim()}
                                                            </TooltipContent>
                                                        </Tooltip>
                                                    ) : (
                                                        msg
                                                    );
                                                })()}
                                                <Tooltip>
                                                    <TooltipTrigger asChild>
                                                        <span className="text-xs text-muted-foreground/60 shrink-0 cursor-default">
                                                            {formatDistanceToNowStrict(new Date(fileCommit.time * 1000), {
                                                                locale: shortLocale,
                                                            })}
                                                        </span>
                                                    </TooltipTrigger>
                                                    <TooltipContent>
                                                        {new Date(fileCommit.time * 1000).toLocaleString(undefined, {
                                                            year: "numeric",
                                                            month: "long",
                                                            day: "numeric",
                                                            hour: "2-digit",
                                                            minute: "2-digit",
                                                            second: "2-digit",
                                                        })}
                                                    </TooltipContent>
                                                </Tooltip>
                                            </div>
                                            <div className="flex items-center gap-1.5 text-xs text-muted-foreground mt-0.5">
                                                <div className="flex h-4 w-4 shrink-0 items-center justify-center rounded-full bg-secondary text-[9px] font-medium">
                                                    {fileCommit.authorName[0].toUpperCase()}
                                                </div>
                                                <span className="font-medium text-foreground/80 shrink-0">{fileCommit.authorName}</span>
                                                <span className="text-muted-foreground/40">·</span>
                                                <span className="font-mono text-muted-foreground/60 shrink-0">
                                                    {fileCommit.sha1.slice(0, 7)}
                                                </span>
                                            </div>
                                        </div>
                                    )}
                                </div>
                                <div className="flex items-center gap-2">
                                    {(!isMarkdown(selectedFile) || showSource) && (
                                        <div className="flex items-center gap-1">
                                            <Button
                                                variant="ghost"
                                                size="sm"
                                                onClick={() => setWrapLines((w) => !w)}
                                                className={`h-8 px-3 gap-2 text-sm hover:text-foreground ${wrapLines ? "text-foreground" : "text-muted-foreground"}`}
                                            >
                                                <WrapText className="h-3.5 w-3.5" />
                                                Wrap
                                            </Button>
                                            {!isBinary && (
                                                <Button
                                                    variant="ghost"
                                                    size="sm"
                                                    onClick={() => setShowBlame((b) => !b)}
                                                    className={`h-8 px-3 gap-2 text-sm hover:text-foreground ${showBlame ? "text-foreground" : "text-muted-foreground"}`}
                                                >
                                                    <GitCommitHorizontal className="h-3.5 w-3.5" />
                                                    Blame
                                                </Button>
                                            )}
                                            <Button
                                                asChild
                                                variant="ghost"
                                                size="sm"
                                                className="h-8 px-3 gap-2 text-sm text-muted-foreground hover:text-foreground"
                                            >
                                                <a
                                                    href={`http://localhost:8080/${user}/${repo}/tree/${branch}/~blob/${selectedFile}`}
                                                    target="_blank"
                                                    rel="noreferrer"
                                                >
                                                    <ExternalLink className="h-3.5 w-3.5" />
                                                    Raw
                                                </a>
                                            </Button>
                                            <Button
                                                variant="ghost"
                                                size="sm"
                                                className="h-8 px-3 gap-2 text-sm text-muted-foreground hover:text-foreground"
                                                asChild
                                            >
                                                <Link
                                                    href={`/${user}/${repo}/commits/${encodeURIComponent(branch)}?path=${encodeURIComponent(selectedFile)}`}
                                                >
                                                    <History className="h-3.5 w-3.5" />
                                                    History
                                                </Link>
                                            </Button>
                                        </div>
                                    )}
                                    {isMarkdown(selectedFile) && (
                                        <div className="flex items-center gap-1 p-0.5 bg-secondary rounded-md">
                                            <button
                                                onClick={() => setShowSource(false)}
                                                className={`flex items-center gap-2 px-3 py-1.5 text-sm rounded transition-colors ${
                                                    !showSource
                                                        ? "bg-background text-foreground"
                                                        : "text-muted-foreground hover:text-foreground"
                                                }`}
                                            >
                                                <BookOpen className="h-3.5 w-3.5" />
                                                Preview
                                            </button>
                                            <button
                                                onClick={() => setShowSource(true)}
                                                className={`flex items-center gap-2 px-3 py-1.5 text-sm rounded transition-colors ${
                                                    showSource
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
                                <FileContent
                                    user={user}
                                    repo={repo}
                                    branch={branch}
                                    filename={selectedFile}
                                    showSource={showSource}
                                    showBlame={showBlame}
                                    wrapLines={wrapLines}
                                    setFileSize={setFileSize}
                                    setCommit={setFileCommit}
                                    setIsBinary={setIsBinary}
                                />
                            </div>
                        </>
                    )}
                </main>

                <RepoSidebar
                    user={user}
                    repo={repo}
                    description={meta.description}
                    projectId={meta.id}
                    license={meta.license}
                    //websiteUrl="idk"
                    //createdAt={"creation date"}
                    topics={[]}
                    languages={meta.languages}
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
                            <div className="h-3.5 w-28 rounded bg-accent shrink-0" />
                            <div className="flex flex-col border-l border-border pl-3 ml-1 gap-1.5">
                                <div className="h-3.5 w-48 rounded bg-accent" />
                                <div className="flex items-center gap-1.5">
                                    <div className="h-4 w-4 rounded-full bg-accent shrink-0" />
                                    <div className="h-3 w-16 rounded bg-accent" />
                                    <div className="h-3 w-12 rounded bg-accent" />
                                </div>
                            </div>
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

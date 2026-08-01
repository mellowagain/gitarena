"use client";

import { useState, useRef, useEffect } from "react";
import Link from "next/link";
import { useParams } from "next/navigation";
import useSWR from "swr";
import { format } from "date-fns";
import { Prism as SyntaxHighlighter } from "react-syntax-highlighter";
import { TopBar } from "@/components/top-bar";
import { Skeleton } from "@/components/ui/skeleton";
import { ErrorDisplay } from "@/components/error-display";
import { HashCopy } from "@/components/hash-copy";
import { jsonFetcher } from "@/lib/fetchers";
import { gitarenaTheme, detectLanguage } from "@/components/code-block";
import {
    ArrowLeft,
    GitCommit,
    ChevronDown,
    Code,
    AlertCircle,
    GitMerge,
    FileCode,
    Plus,
    Minus,
    FileText,
    FileDiff as FileDiffIcon,
    GitBranch,
    ExternalLink,
    AlertTriangle,
} from "lucide-react";
import { UserAvatar } from "@/components/ui/user-avatar";

interface SignatureInfo {
    name: string;
    email: string;
    timestamp: number;
    uid: string | null;
}

interface CommitMeta {
    oid: string;
    shortOid: string;
    message: string;
    description?: string;
    author: SignatureInfo;
    committer: SignatureInfo;
    parents: string[];
}

interface DiffStats {
    filesChanged: number;
    insertions: number;
    deletions: number;
}

interface FileStats {
    insertions: number;
    deletions: number;
}

type DiffLineKind = "context" | "addition" | "deletion";

interface DiffLineEntry {
    kind: DiffLineKind;
    oldLineNumber?: number;
    newLineNumber?: number;
    content: string;
}

interface DiffHunk {
    header: string;
    oldStart: number;
    oldLines: number;
    newStart: number;
    newLines: number;
    lines: DiffLineEntry[];
}

type DiffStatus = "added" | "deleted" | "modified" | "renamed" | "copied" | "untracked";

interface DiffFile {
    status: DiffStatus;
    path: string;
    oldPath?: string;
    binary: boolean;
    tooLarge: boolean;
    stats: FileStats;
    hunks: DiffHunk[];
}

interface CommitDetailResponse {
    commit: CommitMeta;
    branch?: string;
    stats: DiffStats;
    files: DiffFile[];
}

type AstNode =
    | { type: "text"; value: string }
    | { type: "element"; tagName: string; properties: { className?: string[] }; children: AstNode[] };

type AstElement = Extract<AstNode, { type: "element" }>;

function renderNode(node: AstNode, stylesheet: Record<string, React.CSSProperties>, i: number): React.ReactNode {
    if (node.type === "text") {
        return node.value;
    }
    const style = (node.properties?.className ?? []).reduce<React.CSSProperties>(
        (acc, cls) => ({ ...acc, ...(stylesheet[cls] ?? {}) }),
        {}
    );
    return (
        <span key={i} style={style}>
            {node.children.map((child, j) => renderNode(child, stylesheet, j))}
        </span>
    );
}

function HighlightedHunk({ hunk, language }: { hunk: DiffHunk; language: string }) {
    const content = hunk.lines.map((l) => l.content).join("\n");

    return (
        <SyntaxHighlighter
            language={language}
            style={gitarenaTheme}
            PreTag="div"
            renderer={({ rows, stylesheet }) => (
                <>
                    {(rows as AstElement[]).map((row, i) => {
                        const line = hunk.lines[i];
                        if (!line) {
                            return null;
                        }
                        const bg = line.kind === "addition" ? "bg-green-500/8" : line.kind === "deletion" ? "bg-red-500/8" : "";
                        const gutter =
                            line.kind === "addition"
                                ? "text-green-500/50"
                                : line.kind === "deletion"
                                  ? "text-red-500/50"
                                  : "text-muted-foreground/40";
                        const sigil = line.kind === "addition" ? "+" : line.kind === "deletion" ? "−" : " ";
                        return (
                            <div key={i} className={`flex font-mono text-xs leading-5 ${bg}`}>
                                <span className={`select-none w-12 px-2 py-0.5 text-right shrink-0 ${gutter} border-r border-border/50`}>
                                    {line.oldLineNumber ?? ""}
                                </span>
                                <span className={`select-none w-12 px-2 py-0.5 text-right shrink-0 ${gutter} border-r border-border/50`}>
                                    {line.newLineNumber ?? ""}
                                </span>
                                <span className={`select-none w-5 px-1 py-0.5 shrink-0 text-center ${gutter}`}>{sigil}</span>
                                <span className="px-2 py-0.5 whitespace-pre flex-1">
                                    {row.children.map((node, j) => renderNode(node, stylesheet, j))}
                                </span>
                            </div>
                        );
                    })}
                </>
            )}
        >
            {content}
        </SyntaxHighlighter>
    );
}

function fileStatusIcon(status: DiffStatus) {
    switch (status) {
        case "added":
            return <FileText className="h-[18px] w-[18px] shrink-0 text-green-500" />;
        case "deleted":
            return <FileText className="h-[18px] w-[18px] shrink-0 text-red-500" />;
        case "renamed":
        case "copied":
            return <FileDiffIcon className="h-[18px] w-[18px] shrink-0 text-blue-400" />;
        default:
            return <FileText className="h-[18px] w-[18px] shrink-0" />;
    }
}

function fileDiffId(path: string) {
    return `file-${path.replace(/[^a-zA-Z0-9]/g, "-")}`;
}

function ChangedFilesSidebar({ files }: { files: DiffFile[] }) {
    const [sidebarWidth, setSidebarWidth] = useState(240);
    const [isResizing, setIsResizing] = useState(false);
    const [activeFile, setActiveFile] = useState<string | null>(null);
    const sidebarRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        const handleMouseMove = (e: MouseEvent) => {
            if (!isResizing) {
                return;
            }
            setSidebarWidth(Math.max(180, Math.min(480, e.clientX)));
        };
        const handleMouseUp = () => setIsResizing(false);

        if (isResizing) {
            document.addEventListener("mousemove", handleMouseMove);
            document.addEventListener("mouseup", handleMouseUp);
        }

        return () => {
            document.removeEventListener("mousemove", handleMouseMove);
            document.removeEventListener("mouseup", handleMouseUp);
        };
    }, [isResizing]);

    function scrollToFile(path: string) {
        setActiveFile(path);
        const el = document.getElementById(fileDiffId(path));
        if (el) {
            el.scrollIntoView({ behavior: "smooth", block: "start" });
        }
    }

    return (
        <aside
            ref={sidebarRef}
            className="border-r border-border flex flex-col shrink-0 bg-card/30 relative"
            style={{ width: sidebarWidth }}
        >
            <div className="flex-1 overflow-y-auto py-2">
                {files.map((file) => (
                    <button
                        key={file.path}
                        onClick={() => scrollToFile(file.path)}
                        title={file.path}
                        className={`w-full flex items-center gap-2 py-1.5 px-3 hover:bg-accent/50 transition-colors ${
                            activeFile === file.path ? "bg-accent text-foreground" : "text-muted-foreground hover:text-foreground"
                        }`}
                    >
                        {fileStatusIcon(file.status)}
                        <span className="min-w-0 truncate text-left text-sm">{file.path.split("/").pop()}</span>
                    </button>
                ))}
            </div>
            <div
                className="absolute right-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-ring/50 transition-colors"
                onMouseDown={() => setIsResizing(true)}
            />
        </aside>
    );
}

function FileDiff({ file }: { file: DiffFile }) {
    const [collapsed, setCollapsed] = useState(false);
    const language = detectLanguage(file.path);

    return (
        <div id={fileDiffId(file.path)} className="border border-border rounded-md overflow-hidden scroll-mt-4">
            <div className="flex items-center gap-3 px-4 py-2.5 bg-secondary/40 border-b border-border">
                <button onClick={() => setCollapsed(!collapsed)} className="shrink-0">
                    <ChevronDown className={`h-4 w-4 text-muted-foreground transition-transform ${collapsed ? "-rotate-90" : ""}`} />
                </button>
                <FileCode className="h-4 w-4 text-muted-foreground shrink-0" />
                {file.oldPath && <span className="text-sm font-mono text-muted-foreground line-through shrink-0">{file.oldPath}</span>}
                {file.oldPath && <span className="text-muted-foreground">→</span>}
                <span className="text-sm font-mono flex-1">{file.path}</span>
                <span className="text-xs text-green-500 font-medium">+{file.stats.insertions}</span>
                <span className="text-xs text-red-500 font-medium">-{file.stats.deletions}</span>
            </div>
            {!collapsed && (
                <div className="overflow-x-auto">
                    <div className="min-w-max">
                        {file.binary && <div className="px-4 py-3 text-sm text-muted-foreground italic">Binary file</div>}
                        {file.tooLarge && <div className="px-4 py-3 text-sm text-muted-foreground italic">Diff too large to display</div>}
                        {!file.binary &&
                            !file.tooLarge &&
                            file.hunks.map((hunk, hi) => (
                                <div key={hi}>
                                    <div className="px-4 py-1 bg-blue-500/5 text-xs font-mono text-blue-400/70 border-b border-border/50">
                                        {hunk.header}
                                    </div>
                                    <HighlightedHunk hunk={hunk} language={language} />
                                </div>
                            ))}
                    </div>
                </div>
            )}
        </div>
    );
}

function CommitDetailSkeleton() {
    return (
        <div className="max-w-4xl mx-auto px-6 py-8 space-y-6">
            <Skeleton className="h-4 w-32" />
            <div className="space-y-3">
                <Skeleton className="h-8 w-2/3" />
                <div className="flex items-center gap-3">
                    <Skeleton className="h-5 w-5 rounded-full" />
                    <Skeleton className="h-4 w-24" />
                    <Skeleton className="h-4 w-20" />
                </div>
            </div>
            <div className="flex gap-4">
                <Skeleton className="h-4 w-28" />
                <Skeleton className="h-4 w-16" />
                <Skeleton className="h-4 w-16" />
            </div>
            {Array.from({ length: 3 }).map((_, i) => (
                <div key={i} className="border border-border rounded-md overflow-hidden">
                    <div className="flex items-center gap-3 px-4 py-2.5 bg-secondary/40">
                        <Skeleton className="h-4 w-4" />
                        <Skeleton className="h-4 flex-1" />
                        <Skeleton className="h-4 w-8" />
                        <Skeleton className="h-4 w-8" />
                    </div>
                    <div className="p-4 space-y-1">
                        {Array.from({ length: 6 }).map((_, j) => (
                            <Skeleton key={j} className="h-4 w-full" />
                        ))}
                    </div>
                </div>
            ))}
        </div>
    );
}

export default function CommitDetailPage() {
    const { user, repo, hash } = useParams<{ user: string; repo: string; hash: string }>();

    const { data, isLoading, error } = useSWR<CommitDetailResponse>(`/api/repos/${user}/${repo}/commits/${hash}`, jsonFetcher);

    const navLinks = [
        { label: "Code", href: `/${user}/${repo}`, icon: <Code className="h-[18px] w-[18px]" /> },
        { label: "Issues", href: `/${user}/${repo}/issues`, icon: <AlertCircle className="h-[18px] w-[18px]" /> },
        { label: "Merge Requests", href: `/${user}/${repo}/merge-requests`, icon: <GitMerge className="h-[18px] w-[18px]" /> },
    ];

    if (error) {
        return <ErrorDisplay failed="commit" error={error} />;
    }

    const shortOid = data?.commit.shortOid ?? hash.slice(0, 7);

    return (
        <div className="flex flex-col h-screen overflow-hidden bg-background text-foreground font-sans">
            <TopBar
                breadcrumb={[
                    { label: user, href: `/${user}` },
                    { label: repo, href: `/${user}/${repo}` },
                    { label: "commits", href: `/${user}/${repo}/commits` },
                    { label: shortOid },
                ]}
                navLinks={navLinks}
                hasNotifications
            />

            {isLoading && <CommitDetailSkeleton />}

            {data && (
                <div className="flex-1 flex overflow-hidden">
                    <ChangedFilesSidebar files={data.files} />
                    <main className="flex-1 overflow-y-auto">
                        <div className="max-w-4xl mx-auto px-6 py-8">
                            <Link
                                href={data.branch ? `/${user}/${repo}/commits/${data.branch}` : `/${user}/${repo}/commits`}
                                className="inline-flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors mb-6"
                            >
                                <ArrowLeft className="h-4 w-4" />
                                Back to commits
                            </Link>

                            <div className="mb-8">
                                <h1 className="text-2xl font-semibold leading-snug mb-3 text-balance">{data.commit.message}</h1>

                                <div className="flex items-center gap-2.5 text-sm text-muted-foreground flex-wrap">
                                    <div className="flex items-center gap-1.5">
                                        <UserAvatar userId={data.commit.author.uid} username={data.commit.author.name} size="sm" />
                                        <span className="font-medium text-foreground">{data.commit.author.name}</span>
                                    </div>
                                    <span className="text-muted-foreground/40">·</span>
                                    <span title={new Date(data.commit.author.timestamp * 1000).toISOString()}>
                                        {format(new Date(data.commit.author.timestamp * 1000), "MMM d, yyyy 'at' HH:mm")}
                                    </span>
                                    {data.branch && (
                                        <>
                                            <span className="text-muted-foreground/40">·</span>
                                            <span className="flex items-center gap-1">
                                                <GitBranch className="h-3.5 w-3.5" />
                                                <Link
                                                    href={`/${user}/${repo}/commits/${data.branch}`}
                                                    className="hover:text-foreground transition-colors"
                                                >
                                                    <code className="px-1.5 py-0.5 bg-secondary rounded text-xs hover:bg-accent transition-colors">
                                                        {data.branch}
                                                    </code>
                                                </Link>
                                            </span>
                                        </>
                                    )}
                                </div>

                                {data.commit.description && (
                                    <div className="mt-5 pl-5 border-l-4 border-border">
                                        <p className="text-sm text-muted-foreground leading-relaxed whitespace-pre-wrap">
                                            {data.commit.description}
                                        </p>
                                    </div>
                                )}
                            </div>

                            {!data.branch && (
                                <div className="flex gap-3 px-4 py-3 mb-4 border border-yellow-500/30 bg-yellow-500/5 rounded-md text-sm">
                                    <AlertTriangle className="h-4 w-4 text-yellow-500 shrink-0 mt-0.5" />
                                    <div className="space-y-1">
                                        <p className="text-foreground font-medium">Dangling commit</p>
                                        <p className="text-muted-foreground">
                                            This commit is not reachable from any branch and may be garbage collected. If this is
                                            unintended, you can rescue it with{" "}
                                            <code className="px-1 py-0.5 bg-secondary rounded text-xs">
                                                git branch rescue {data.commit.shortOid}
                                            </code>
                                            .
                                        </p>
                                    </div>
                                </div>
                            )}

                            <div className="space-y-4">
                                {data.files.map((file) => (
                                    <FileDiff key={file.path} file={file} />
                                ))}
                            </div>
                        </div>
                    </main>

                    <aside className="w-72 border-l border-border shrink-0 overflow-y-auto p-5 space-y-6">
                        <div className="grid grid-cols-3 gap-2">
                            <div className="text-center p-2.5 border border-border rounded-md">
                                <p className="text-base font-semibold">{data.stats.filesChanged}</p>
                                <p className="text-xs text-muted-foreground">Files</p>
                            </div>
                            <div className="text-center p-2.5 border border-border rounded-md">
                                <p className="text-base font-semibold text-green-500 flex items-center justify-center gap-0.5">
                                    <Plus className="h-3 w-3" />
                                    {data.stats.insertions}
                                </p>
                                <p className="text-xs text-muted-foreground">Added</p>
                            </div>
                            <div className="text-center p-2.5 border border-border rounded-md">
                                <p className="text-base font-semibold text-red-500 flex items-center justify-center gap-0.5">
                                    <Minus className="h-3 w-3" />
                                    {data.stats.deletions}
                                </p>
                                <p className="text-xs text-muted-foreground">Removed</p>
                            </div>
                        </div>

                        <div>
                            <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">Commit</h3>
                            <div className="space-y-2">
                                <div className="flex items-center gap-2">
                                    <GitCommit className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                                    <HashCopy shortHash={data.commit.shortOid} fullHash={data.commit.oid} />
                                </div>
                                {data.commit.parents.length > 0 && (
                                    <div className="flex items-center gap-2 text-xs text-muted-foreground">
                                        <span className="shrink-0">Parent:</span>
                                        <Link
                                            href={`/${user}/${repo}/commit/${data.commit.parents[0]}`}
                                            className="font-mono hover:text-foreground transition-colors"
                                        >
                                            {data.commit.parents[0].slice(0, 7)}
                                        </Link>
                                    </div>
                                )}
                            </div>
                        </div>

                        <div>
                            <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">Author</h3>
                            <div className="flex items-center gap-2.5 px-3 py-2.5 border border-border rounded-md">
                                <UserAvatar
                                    userId={data.commit.author.uid}
                                    username={data.commit.author.name}
                                    size="md"
                                    className="size-7"
                                />
                                <div className="min-w-0">
                                    <p className="text-sm font-medium">{data.commit.author.name}</p>
                                    <p className="text-xs text-muted-foreground truncate">{data.commit.author.email}</p>
                                </div>
                            </div>
                        </div>

                        {data.commit.committer.email !== data.commit.author.email && (
                            <div>
                                <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">Committer</h3>
                                <div className="flex items-center gap-2.5 px-3 py-2.5 border border-border rounded-md">
                                    <UserAvatar
                                        userId={data.commit.committer.uid}
                                        username={data.commit.committer.name}
                                        size="md"
                                        className="size-7"
                                    />
                                    <div className="min-w-0">
                                        <p className="text-sm font-medium">{data.commit.committer.name}</p>
                                        <p className="text-xs text-muted-foreground truncate">{data.commit.committer.email}</p>
                                    </div>
                                </div>
                            </div>
                        )}

                        <div>
                            <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">Browse</h3>
                            <Link
                                href={`/${user}/${repo}/tree/${data.commit.oid}`}
                                className="flex items-center gap-2 px-3 py-2 border border-border rounded-md text-sm hover:bg-accent/50 transition-colors text-muted-foreground hover:text-foreground"
                            >
                                <ExternalLink className="h-3.5 w-3.5 shrink-0" />
                                View repository at this commit
                            </Link>
                        </div>
                    </aside>
                </div>
            )}
        </div>
    );
}

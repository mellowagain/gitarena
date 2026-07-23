"use client";

import { GitCommit, ChevronDown, ChevronRight, Folder, FileText, FileCode, Link2, TriangleAlert } from "lucide-react";
import Link from "next/link";
import React, { useState, useRef, useEffect, useMemo } from "react";
import useSWR from "swr";
import { BranchBar } from "@/components/branch-bar";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { formatDistanceToNowStrict } from "date-fns";
import { shortLocale } from "@/lib/utils";
import { ErrorDisplay } from "@/components/error-display";

export type FileNode = {
    name: string;
    fileType: string;
    children?: FileNode[];
    lastCommit?: string;
    lastChanged?: string;
};

export type RepoFileSidebarProps = {
    user: string;
    repo: string;
    selectedFile: string | null;
    setSelectedFile: (path: string) => void;

    branch: string;
    onBranchChange: (branch: string) => void;

    defaultBranch: string;
};

interface BranchFile {
    fileType: string;
    fileName: string;
    submodule_target_oid?: string;
    commit: FileCommitInfo;
}

export interface FileCommitInfo {
    sha1: string;
    message: string;
    time: number;
    authorName: string;
    authorEmail: string;
    authorUid?: string;
}

function buildFileTree(files: BranchFile[]): FileNode[] {
    const root: FileNode[] = [];
    const nodeMap = new Map<string, FileNode>();

    for (const file of files) {
        const parts = file.fileName.split("/");
        const node: FileNode = {
            name: parts[parts.length - 1],
            fileType: file.fileType,
            lastCommit: file.commit.message.trim(),
            lastChanged: formatDistanceToNowStrict(new Date(file.commit.time * 1000), { addSuffix: false, locale: shortLocale }),
            children: file.fileType === "tree" ? [] : undefined,
        };

        nodeMap.set(file.fileName, node);

        if (parts.length === 1) {
            root.push(node);
        } else {
            const parentPath = parts.slice(0, -1).join("/");
            nodeMap.get(parentPath)?.children?.push(node);
        }
    }

    return root;
}

function FileTreeSkeleton() {
    return (
        <div className="flex-1 py-2 space-y-0.5 animate-pulse">
            {[1, 2, 3, 4, 5, 6, 7].map((i) => (
                <div key={i} className="flex items-center gap-2 px-3 py-1.5">
                    <div className="h-[18px] w-[18px] rounded bg-accent shrink-0" />
                    <div className="h-3 rounded bg-accent" style={{ width: `${40 + ((i * 17) % 45)}%` }} />
                    <div className="h-2.5 w-10 rounded bg-accent ml-auto" />
                </div>
            ))}
        </div>
    );
}

function RepoFileSidebarCommitInfoSkeleton() {
    return (
        <div className="flex items-start gap-2.5 w-full animate-pulse">
            <div className="h-6 w-6 rounded-full bg-accent shrink-0" />
            <div className="flex-1 space-y-2">
                <div className="h-3 w-3/4 rounded bg-accent" />
                <div className="h-2.5 w-1/2 rounded bg-accent" />
            </div>
        </div>
    );
}

function RepoFileSidebarCommitInfo({ user, repo, branch }: { user: string; repo: string; branch: string }) {
    const { data, error, isLoading } = useSWR<{ commits: FileCommitInfo[] }>(`/api/repos/${user}/${repo}/branch/${branch}/commits?limit=1`);

    if (isLoading) {
        return <RepoFileSidebarCommitInfoSkeleton />;
    }

    if (error || !data) {
        return <ErrorDisplay failed={"latest commit"} error={error} />;
    }

    const commit = data.commits[0];

    return (
        <>
            <div className="flex h-6 w-6 items-center justify-center rounded-full bg-secondary text-xs font-medium shrink-0">
                {commit.authorName[0].toUpperCase()}
            </div>
            <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                    <span className="font-medium text-foreground">{commit.authorName}</span>
                    <Tooltip>
                        <TooltipTrigger asChild>
                            <span className="truncate cursor-default">{commit.message}</span>
                        </TooltipTrigger>
                        <TooltipContent className="max-w-sm whitespace-pre-wrap">{commit.message.trim()}</TooltipContent>
                    </Tooltip>
                </div>
                <div className="flex items-center gap-2 mt-1 text-xs">
                    <Link
                        href={`/${user}/${repo}/commit/${commit.sha1}`}
                        className="font-mono hover:text-foreground transition-colors flex items-center gap-1"
                    >
                        <GitCommit className="h-3.5 w-3.5" />
                        {commit.sha1.slice(0, 7)}
                    </Link>
                    <span className="ml-auto">
                        {formatDistanceToNowStrict(new Date(commit.time * 1000), { addSuffix: true, locale: shortLocale })}
                    </span>
                </div>
            </div>
        </>
    );
}

function FileTooltip({ commit, date, children }: { commit: string; date: string; children: React.ReactNode }) {
    const [show, setShow] = useState(false);
    const [position, setPosition] = useState({ x: 0, y: 0 });
    const ref = useRef<HTMLDivElement>(null);

    return (
        <div
            ref={ref}
            className="relative"
            onMouseEnter={(e) => {
                const rect = e.currentTarget.getBoundingClientRect();
                setPosition({ x: rect.right + 8, y: rect.top });
                setShow(true);
            }}
            onMouseLeave={() => setShow(false)}
        >
            {children}
            {show && (
                <div
                    className="fixed z-50 px-3 py-2 text-sm bg-popover border border-border rounded-md shadow-lg whitespace-nowrap"
                    style={{ left: position.x, top: position.y }}
                >
                    <span className="text-foreground">{commit}</span>
                    <span className="text-muted-foreground ml-2">{date}</span>
                </div>
            )}
        </div>
    );
}

function FileTreeItem({
    node,
    depth = 0,
    selectedFile,
    onSelect,
    expandedFolders,
    onToggleFolder,
    path = "",
    sidebarWidth,
}: {
    node: FileNode;
    depth?: number;
    selectedFile: string | null;
    onSelect: (path: string) => void;
    expandedFolders: Set<string>;
    onToggleFolder: (path: string) => void;
    path?: string;
    sidebarWidth: number;
}) {
    const fullPath = path ? `${path}/${node.name}` : node.name;
    const isExpanded = expandedFolders.has(fullPath);
    const isSelected = selectedFile === fullPath;

    const baseWidth = 180;
    const showDate = sidebarWidth > baseWidth + 40;
    const showCommit = sidebarWidth > baseWidth + 120;

    const content = (
        <button
            onClick={() => (node.fileType === "tree" ? onToggleFolder(fullPath) : onSelect(fullPath))}
            className={`w-full flex items-center gap-2 py-1.5 px-3 hover:bg-accent/50 transition-colors ${
                isSelected ? "bg-accent text-foreground" : "text-muted-foreground hover:text-foreground"
            }`}
            style={{ paddingLeft: `${depth * 14 + (node.fileType === "tree" ? 12 : 26)}px` }}
        >
            {node.fileType === "tree" &&
                (isExpanded ? <ChevronDown className="h-4 w-4 shrink-0" /> : <ChevronRight className="h-4 w-4 shrink-0" />)}
            {node.fileType === "tree" ? (
                <Folder className="h-[18px] w-[18px] shrink-0" />
            ) : node.fileType === "link" ? (
                <Link2 className="h-[18px] w-[18px] shrink-0" />
            ) : node.fileType === "commit" ? (
                <GitCommit className="h-[18px] w-[18px] shrink-0" />
            ) : node.fileType === "blobExecutable" ? (
                <FileCode className="h-[18px] w-[18px] shrink-0" />
            ) : (
                <FileText className="h-[18px] w-[18px] shrink-0" />
            )}
            <span className="min-w-0 truncate text-left">{node.name}</span>
            {node.lastCommit && showCommit && (
                <span className="text-xs text-muted-foreground/40 truncate flex-1 text-left">{node.lastCommit}</span>
            )}
            {!showCommit && <span className="flex-1" />}
            {node.lastChanged && showDate && (
                <span className="text-xs text-muted-foreground/40 shrink-0 text-right">{node.lastChanged}</span>
            )}
        </button>
    );

    const wrappedContent =
        node.lastCommit && !showCommit ? (
            <FileTooltip commit={node.lastCommit} date={node.lastChanged || ""}>
                {content}
            </FileTooltip>
        ) : (
            content
        );

    if (node.fileType === "tree") {
        return (
            <div>
                {wrappedContent}
                {isExpanded && node.children && (
                    <div>
                        {node.children.map((child) => (
                            <FileTreeItem
                                key={child.name}
                                node={child}
                                depth={depth + 1}
                                selectedFile={selectedFile}
                                onSelect={onSelect}
                                expandedFolders={expandedFolders}
                                onToggleFolder={onToggleFolder}
                                path={fullPath}
                                sidebarWidth={sidebarWidth}
                            />
                        ))}
                    </div>
                )}
            </div>
        );
    }

    return wrappedContent;
}

export function RepoFileSidebar({
    user,
    repo,
    selectedFile,
    setSelectedFile,
    branch,
    onBranchChange,
    defaultBranch,
}: RepoFileSidebarProps) {
    const [sidebarWidth, setSidebarWidth] = useState(320);
    const [isResizing, setIsResizing] = useState(false);
    const [expandedFolders, setExpandedFolders] = useState<Set<string>>(new Set());
    const sidebarRef = useRef<HTMLDivElement>(null);

    const effectiveExpandedFolders = useMemo(() => {
        if (!selectedFile) {
            return expandedFolders;
        }
        const parts = selectedFile.split("/");
        if (parts.length <= 1) {
            return expandedFolders;
        }
        const merged = new Set(expandedFolders);
        for (let i = 1; i < parts.length; i++) {
            merged.add(parts.slice(0, i).join("/"));
        }
        return merged;
    }, [selectedFile, expandedFolders]);

    const { data, error, isLoading } = useSWR<{ files: BranchFile[]; truncated: boolean }>(
        `/api/repos/${user}/${repo}/branch/${branch}/files`
    );

    useEffect(() => {
        const handleMouseMove = (e: MouseEvent) => {
            if (!isResizing) {
                return;
            }
            setSidebarWidth(Math.max(240, Math.min(480, e.clientX)));
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

    if (error || (!data && !isLoading)) {
        return <ErrorDisplay failed={"files"} error={error} />;
    }

    const toggleFolder = (path: string) => {
        setExpandedFolders((prev) => {
            const next = new Set(prev);
            if (next.has(path)) {
                next.delete(path);
            } else {
                next.add(path);
            }
            return next;
        });
    };

    return (
        <aside
            ref={sidebarRef}
            className="w-full lg:w-[var(--repo-file-sidebar-width)] border-b lg:border-r lg:border-b-0 border-border flex flex-col shrink-0 bg-card/30 relative"
            style={{ "--repo-file-sidebar-width": `${sidebarWidth}px` } as React.CSSProperties}
        >
            <div className="p-4 border-b border-border space-y-3">
                <BranchBar user={user} repo={repo} defaultBranch={defaultBranch} selectedBranch={branch} onBranchChange={onBranchChange} />

                <div className="flex items-start gap-2.5 text-sm text-muted-foreground">
                    <RepoFileSidebarCommitInfo user={user} repo={repo} branch={branch} />
                </div>
            </div>

            {!isLoading && data?.truncated && (
                <div className="flex items-center gap-2 px-3 py-2 border-b border-border text-xs text-muted-foreground">
                    <TriangleAlert className="h-3.5 w-3.5 shrink-0 text-yellow-500" />
                    <span>Showing first 10,000 entries only</span>
                </div>
            )}
            {isLoading ? (
                <FileTreeSkeleton />
            ) : (
                <div className="flex-1 overflow-y-auto py-2">
                    {buildFileTree(data?.files ?? []).map((node) => (
                        <FileTreeItem
                            key={node.name}
                            node={node}
                            selectedFile={selectedFile}
                            onSelect={setSelectedFile}
                            expandedFolders={effectiveExpandedFolders}
                            onToggleFolder={toggleFolder}
                            sidebarWidth={sidebarWidth}
                        />
                    ))}
                </div>
            )}

            <div
                className="absolute right-0 top-0 bottom-0 hidden lg:block w-1 cursor-col-resize hover:bg-ring/50 transition-colors"
                onMouseDown={() => setIsResizing(true)}
            />
        </aside>
    );
}

export function RepoFileSidebarSkeleton() {
    return (
        <aside className="w-full lg:w-80 border-b lg:border-r lg:border-b-0 border-border flex flex-col shrink-0 bg-card/30 animate-pulse">
            <div className="p-4 border-b border-border space-y-3">
                {/* Branch dropdown + history button */}
                <div className="flex items-center gap-2">
                    <div className="h-9 flex-1 rounded bg-accent" />
                    <div className="h-9 w-14 rounded bg-accent" />
                </div>
                {/* Latest commit row */}
                <div className="flex items-start gap-2.5">
                    <div className="h-6 w-6 rounded-full bg-accent shrink-0" />
                    <div className="flex-1 space-y-2">
                        <div className="h-3 w-3/4 rounded bg-accent" />
                        <div className="h-2.5 w-1/2 rounded bg-accent" />
                    </div>
                </div>
            </div>
            {/* File tree rows */}
            <div className="flex-1 py-2 space-y-0.5">
                {[1, 2, 3, 4, 5, 6, 7].map((i) => (
                    <div key={i} className="flex items-center gap-2 px-3 py-1.5">
                        <div className="h-[18px] w-[18px] rounded bg-accent shrink-0" />
                        <div className="h-3 rounded bg-accent" style={{ width: `${40 + ((i * 17) % 45)}%` }} />
                        <div className="h-2.5 w-10 rounded bg-accent ml-auto" />
                    </div>
                ))}
            </div>
        </aside>
    );
}

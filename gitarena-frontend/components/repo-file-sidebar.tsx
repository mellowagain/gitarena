"use client";

import {
    GitBranch,
    GitCommit,
    History,
    ChevronDown,
    ChevronRight,
    Folder,
    FileText,
    CheckCircle2,
    XCircle,
    Loader2,
    Ban,
} from "lucide-react";
import Link from "next/link";
import React, { useState, useRef, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import useSWR from "swr";
import { formatDistanceToNowStrict } from "date-fns";
import { ErrorDisplay } from "@/components/error-display";
import { RepoPageSkeleton } from "@/app/[user]/[repo]/page";

export type FileNode = {
    name: string;
    type: "file" | "folder";
    children?: FileNode[];
    lastCommit?: string;
    lastChanged?: string;
};

type LatestCommit = {
    hash: string;
    message: string;
    author: string;
    date: string;
    totalCommits: number;
    ciStatus: "pending" | "passed" | "failed" | "cancelled";
};

export type RepoFileSidebarProps = {
    user: string;
    repo: string;
    selectedFile: string | null;
    setSelectedFile: (path: string) => void;

    initialBranch?: string;

    defaultBranch: string;
    branches: string[];
    latestCommit: LatestCommit;
};

interface BranchFile {
    fileType: string;
    fileName: string;
    submodule_target_oid?: string;
    commit: FileCommitInfo;
}

interface FileCommitInfo {
    sha1: string;
    message: string;
    time: number;
    authorName: string;
    authorEmail: string;
    authorUid?: number;
}

function buildFileTree(files: BranchFile[]): FileNode[] {
    const root: FileNode[] = [];
    const nodeMap = new Map<string, FileNode>();

    for (const file of files) {
        const parts = file.fileName.split("/");
        const node: FileNode = {
            name: parts[parts.length - 1],
            type: file.fileType === "tree" ? "folder" : "file",
            lastCommit: file.commit.message.trim(),
            lastChanged: formatDistanceToNowStrict(new Date(file.commit.time * 1000), { addSuffix: false }),
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

function CIStatusBadge({ status }: { status: "pending" | "passed" | "failed" | "cancelled" }) {
    const config = {
        pending: { icon: Loader2, className: "text-yellow-500 animate-spin" },
        passed: { icon: CheckCircle2, className: "text-green-500" },
        failed: { icon: XCircle, className: "text-red-500" },
        cancelled: { icon: Ban, className: "text-muted-foreground" },
    };

    const { icon: Icon, className } = config[status];
    return <Icon className={`h-4 w-4 ${className}`} />;
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
            onClick={() => (node.type === "folder" ? onToggleFolder(fullPath) : onSelect(fullPath))}
            className={`w-full flex items-center gap-2 py-1.5 px-3 hover:bg-accent/50 transition-colors ${
                isSelected ? "bg-accent text-foreground" : "text-muted-foreground hover:text-foreground"
            }`}
            style={{ paddingLeft: `${depth * 14 + (node.type === "folder" ? 12 : 26)}px` }}
        >
            {node.type === "folder" &&
                (isExpanded ? <ChevronDown className="h-4 w-4 shrink-0" /> : <ChevronRight className="h-4 w-4 shrink-0" />)}
            {node.type === "folder" ? (
                <Folder className="h-[18px] w-[18px] shrink-0" />
            ) : (
                <FileText className="h-[18px] w-[18px] shrink-0" />
            )}
            <span className="shrink-0 text-left">{node.name}</span>
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

    if (node.type === "folder") {
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
    initialBranch,
    defaultBranch,
    branches,
    latestCommit,
}: RepoFileSidebarProps) {
    const [sidebarWidth, setSidebarWidth] = useState(320);
    const [isResizing, setIsResizing] = useState(false);
    const [expandedFolders, setExpandedFolders] = useState<Set<string>>(new Set());
    const sidebarRef = useRef<HTMLDivElement>(null);

    const [branch, setBranch] = useState(initialBranch ?? defaultBranch);

    const { data, error, isLoading } = useSWR<{ files: BranchFile[] }>(
        `http://localhost:8080/api/repos/${user}/${repo}/branch/${branch}/files`
    );

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

    return (
        <aside
            ref={sidebarRef}
            className="border-r border-border flex flex-col shrink-0 bg-card/30 relative"
            style={{ width: sidebarWidth }}
        >
            <div className="p-4 border-b border-border space-y-3">
                <div className="flex items-center gap-2">
                    <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                            <Button variant="secondary" size="sm" className="flex-1 justify-between h-9">
                                <span className="flex items-center gap-2">
                                    <GitBranch className="h-4 w-4" />
                                    {branch}
                                </span>
                                <ChevronDown className="h-3.5 w-3.5 opacity-50" />
                            </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="start" className="w-56">
                            {branches.map((branch) => (
                                <DropdownMenuItem key={branch}>
                                    <GitBranch className="mr-2 h-4 w-4 opacity-50" />
                                    {branch}
                                </DropdownMenuItem>
                            ))}
                        </DropdownMenuContent>
                    </DropdownMenu>
                    <Link
                        href="#"
                        className="flex items-center gap-1.5 px-2.5 h-9 text-sm text-muted-foreground hover:text-foreground transition-colors border border-border rounded-md hover:bg-accent/50"
                    >
                        <History className="h-3.5 w-3.5" />
                        <span>{latestCommit.totalCommits}</span>
                    </Link>
                </div>

                <div className="flex items-start gap-2.5 text-sm text-muted-foreground">
                    <div className="flex h-6 w-6 items-center justify-center rounded-full bg-secondary text-xs font-medium shrink-0">
                        {latestCommit.author[0].toUpperCase()}
                    </div>
                    <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2">
                            <span className="font-medium text-foreground">{latestCommit.author}</span>
                            <span className="truncate">{latestCommit.message}</span>
                        </div>
                        <div className="flex items-center gap-2 mt-1 text-xs">
                            <CIStatusBadge status={latestCommit.ciStatus} />
                            <Link href="#" className="font-mono hover:text-foreground transition-colors flex items-center gap-1">
                                <GitCommit className="h-3.5 w-3.5" />
                                {latestCommit.hash}
                            </Link>
                            <span>{latestCommit.date}</span>
                        </div>
                    </div>
                </div>
            </div>

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
                            expandedFolders={expandedFolders}
                            onToggleFolder={toggleFolder}
                            sidebarWidth={sidebarWidth}
                        />
                    ))}
                </div>
            )}

            <div
                className="absolute right-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-ring/50 transition-colors"
                onMouseDown={() => setIsResizing(true)}
            />
        </aside>
    );
}

export function RepoFileSidebarSkeleton() {
    return (
        <aside className="w-80 border-r border-border flex flex-col shrink-0 bg-card/30 animate-pulse">
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

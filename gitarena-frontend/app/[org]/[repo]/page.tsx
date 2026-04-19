"use client";

import {
    GitBranch,
    GitCommit,
    History,
    Star,
    GitFork,
    Eye,
    Copy,
    Check,
    FileText,
    Folder,
    ChevronDown,
    ChevronRight,
    AlertCircle,
    GitMerge,
    Scale,
    Users,
    Package,
    CheckCircle2,
    XCircle,
    Loader2,
    Ban,
    ExternalLink,
    Calendar,
    Code,
    BookOpen,
    Edit3,
} from "lucide-react";
import Link from "next/link";
import { useState, useRef, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { TopBar } from "@/components/top-bar";

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

type FileNode = {
    name: string;
    type: "file" | "folder";
    children?: FileNode[];
    lastCommit?: string;
    lastChanged?: string;
};

const fileTree: FileNode[] = [
    {
        name: "src",
        type: "folder",
        lastCommit: "refactor handlers",
        lastChanged: "2d",
        children: [
            { name: "main.rs", type: "file", lastCommit: "add cli args", lastChanged: "3d" },
            { name: "lib.rs", type: "file", lastCommit: "export config", lastChanged: "5d" },
            {
                name: "handlers",
                type: "folder",
                lastCommit: "refactor handlers",
                lastChanged: "2d",
                children: [
                    { name: "auth.rs", type: "file", lastCommit: "fix auth flow", lastChanged: "2d" },
                    { name: "repo.rs", type: "file", lastCommit: "add list repos", lastChanged: "4d" },
                ],
            },
        ],
    },
    { name: ".gitignore", type: "file", lastCommit: "init", lastChanged: "19h" },
    { name: "Cargo.toml", type: "file", lastCommit: "add sqlx dep", lastChanged: "1d" },
    { name: "README.md", type: "file", lastCommit: "update readme", lastChanged: "6h" },
    { name: "LICENSE", type: "file", lastCommit: "init", lastChanged: "19h" },
];

const readmeContent = `# test

A lightweight git hosting solution built for speed and simplicity.

## Features

- Fast repository browsing
- Built-in code review
- Issue tracking
- Cross-platform support

## Getting Started

\`\`\`bash
git clone https://git.mari.zip/mellowagain/test.git
cd test
cargo build --release
\`\`\`

## License

MIT License - see LICENSE for details.`;

const fileContents: Record<string, string> = {
    "src/main.rs": `use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("GitArena v0.1.0");
    
    if args.len() > 1 {
        match args[1].as_str() {
            "serve" => start_server(),
            "init" => init_repo(),
            _ => print_help(),
        }
    }
}

fn start_server() {
    println!("Starting server on :3000...");
}

fn init_repo() {
    println!("Initializing repository...");
}

fn print_help() {
    println!("Usage: gitarena [command]");
}`,
    "src/lib.rs": `pub mod handlers;
pub mod models;
pub mod config;

pub use config::Config;`,
    "src/handlers/auth.rs": `use crate::models::User;

pub async fn login(credentials: Credentials) -> Result<Token, AuthError> {
    let user = User::find_by_email(&credentials.email).await?;
    user.verify_password(&credentials.password)?;
    
    Ok(Token::generate(&user))
}

pub async fn logout(token: Token) -> Result<(), AuthError> {
    token.invalidate().await
}`,
    "src/handlers/repo.rs": `use crate::models::Repository;

pub async fn list_repos(user_id: i64) -> Vec<Repository> {
    Repository::find_by_owner(user_id).await
}

pub async fn create_repo(name: &str, owner_id: i64) -> Result<Repository, RepoError> {
    Repository::create(name, owner_id).await
}`,
    ".gitignore": `/target
*.log
.env
.DS_Store`,
    "Cargo.toml": `[package]
name = "gitarena"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["full"] }
axum = "0.7"
serde = { version = "1", features = ["derive"] }
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio"] }`,
    LICENSE: `MIT License

Copyright (c) 2024 mellowagain

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.`,
};

function CopyButton({ text }: { text: string }) {
    const [copied, setCopied] = useState(false);

    const handleCopy = () => {
        navigator.clipboard.writeText(text);
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
    };

    return (
        <button className="p-2 text-muted-foreground hover:text-foreground transition-colors" onClick={handleCopy}>
            {copied ? <Check className="h-4 w-4" /> : <Copy className="h-4 w-4" />}
        </button>
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

// Simple syntax highlighting based on file extension
function tokenize(line: string, ext: string): React.ReactNode[] {
    const rustKeywords =
        /\b(fn|let|mut|pub|use|mod|struct|impl|enum|match|if|else|for|while|loop|return|async|await|const|static|type|where|trait|self|Self|crate|super|move|ref|in|as|break|continue|unsafe|extern|dyn)\b/g;
    const jsKeywords =
        /\b(function|const|let|var|if|else|for|while|return|import|export|from|default|class|extends|new|this|async|await|try|catch|throw|typeof|instanceof|true|false|null|undefined)\b/g;
    const tomlKeywords = /\b(true|false)\b/g;

    const keywords = ext === "rs" ? rustKeywords : ext === "toml" ? tomlKeywords : jsKeywords;
    const stringRegex = /(["'`])(?:(?!\1)[^\\]|\\.)*?\1/g;
    const commentRegex = ext === "rs" || ext === "toml" ? /\/\/.*$|#.*$/g : /\/\/.*$|\/\*[\s\S]*?\*\//g;
    const numberRegex = /\b(\d+\.?\d*)\b/g;

    const commentMatch = line.match(commentRegex);
    if ((commentMatch && line.trim().startsWith("//")) || line.trim().startsWith("#")) {
        return [
            <span key="comment" className="text-muted-foreground/50 italic">
                {line}
            </span>,
        ];
    }

    let result = line;

    result = result.replace(stringRegex, (match) => `\x00STR${match}\x00END`);
    result = result.replace(keywords, (match) => `\x00KW${match}\x00END`);
    result = result.replace(numberRegex, (match) => `\x00NUM${match}\x00END`);

    const parts = result.split(/(\x00(?:STR|KW|NUM).*?\x00END)/g);

    return parts.map((part, i) => {
        if (part.startsWith("\x00STR")) {
            const content = part.slice(4, -4);
            return (
                <span key={i} className="text-amber-400/80">
                    {content}
                </span>
            );
        }
        if (part.startsWith("\x00KW")) {
            const content = part.slice(3, -4);
            return (
                <span key={i} className="text-blue-400/90">
                    {content}
                </span>
            );
        }
        if (part.startsWith("\x00NUM")) {
            const content = part.slice(4, -4);
            return (
                <span key={i} className="text-purple-400/80">
                    {content}
                </span>
            );
        }
        return (
            <span key={i} className="text-foreground/80">
                {part}
            </span>
        );
    });
}

function CodeBlock({ content, filename }: { content: string; filename: string }) {
    const lines = content.split("\n");
    const ext = filename.split(".").pop() || "";

    return (
        <div className="font-mono text-sm leading-relaxed">
            {lines.map((line, i) => (
                <div key={i} className="flex hover:bg-accent/30 group py-0.5">
                    <span className="w-14 shrink-0 text-right pr-4 text-muted-foreground/40 select-none group-hover:text-muted-foreground/60">
                        {i + 1}
                    </span>
                    <pre className="flex-1 overflow-x-auto pr-6">
                        <code>{tokenize(line || " ", ext)}</code>
                    </pre>
                </div>
            ))}
        </div>
    );
}

function ReadmeView({ showSource }: { showSource: boolean }) {
    if (showSource) {
        return <CodeBlock content={readmeContent} filename="README.md" />;
    }

    const sections = readmeContent.split("\n\n");

    return (
        <div className="p-8 space-y-5">
            {sections.map((section, i) => {
                if (section.startsWith("# ")) {
                    return (
                        <h1 key={i} className="text-2xl font-semibold text-foreground">
                            {section.slice(2)}
                        </h1>
                    );
                }
                if (section.startsWith("## ")) {
                    return (
                        <h2 key={i} className="text-lg font-medium text-foreground pt-2">
                            {section.slice(3)}
                        </h2>
                    );
                }
                if (section.startsWith("```")) {
                    const lines = section.split("\n");
                    const code = lines.slice(1, -1).join("\n");
                    return (
                        <pre key={i} className="rounded-md bg-card border border-border p-5 font-mono text-sm overflow-x-auto">
                            <code className="text-muted-foreground">{code}</code>
                        </pre>
                    );
                }
                if (section.startsWith("- ")) {
                    return (
                        <ul key={i} className="list-disc list-inside text-muted-foreground space-y-1.5">
                            {section.split("\n").map((item, j) => (
                                <li key={j}>{item.slice(2)}</li>
                            ))}
                        </ul>
                    );
                }
                return (
                    <p key={i} className="text-muted-foreground leading-relaxed">
                        {section}
                    </p>
                );
            })}
        </div>
    );
}

function LanguageBar({ languages }: { languages: typeof repoData.languages }) {
    const [hoveredLang, setHoveredLang] = useState<string | null>(null);

    return (
        <div className="space-y-2.5">
            <div className="flex h-2.5 rounded-full overflow-hidden">
                {languages.map((lang) => (
                    <div
                        key={lang.name}
                        className="h-full transition-opacity hover:opacity-80"
                        style={{ width: `${lang.percentage}%`, backgroundColor: lang.color }}
                        onMouseEnter={() => setHoveredLang(lang.name)}
                        onMouseLeave={() => setHoveredLang(null)}
                    />
                ))}
            </div>
            <div className="flex flex-wrap gap-x-4 gap-y-1.5">
                {languages.map((lang) => (
                    <div
                        key={lang.name}
                        className={`flex items-center gap-2 text-sm transition-opacity ${
                            hoveredLang && hoveredLang !== lang.name ? "opacity-40" : ""
                        }`}
                    >
                        <span className="w-2.5 h-2.5 rounded-full" style={{ backgroundColor: lang.color }} />
                        <span className="text-foreground">{lang.name}</span>
                        <span className="text-muted-foreground">{lang.percentage}%</span>
                    </div>
                ))}
            </div>
        </div>
    );
}

export default function RepoPage() {
    const [selectedFile, setSelectedFile] = useState<string | null>(null);
    const [expandedFolders, setExpandedFolders] = useState<Set<string>>(new Set(["src"]));
    const [protocol, setProtocol] = useState<"https" | "ssh">("https");
    const [sidebarWidth, setSidebarWidth] = useState(320);
    const [showReadmeSource, setShowReadmeSource] = useState(false);
    const [isResizing, setIsResizing] = useState(false);
    const sidebarRef = useRef<HTMLDivElement>(null);

    const cloneUrl =
        protocol === "https"
            ? `https://git.mari.zip/${repoData.org}/${repoData.name}.git`
            : `git@git.mari.zip:${repoData.org}/${repoData.name}.git`;

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
            const newWidth = Math.max(240, Math.min(480, e.clientX));
            setSidebarWidth(newWidth);
        };

        const handleMouseUp = () => {
            setIsResizing(false);
        };

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
        <div className="min-h-screen bg-background flex flex-col">
            <TopBar
                breadcrumb={[{ label: repoData.org, href: `/${repoData.org}` }, { label: repoData.name }]}
                search={{ placeholder: "Search files, commits, issues..." }}
                navLinks={[
                    {
                        label: "Code",
                        href: `/${repoData.org}/${repoData.name}`,
                        icon: <Code className="h-[18px] w-[18px]" />,
                        active: true,
                    },
                    {
                        label: "Issues",
                        href: `/${repoData.org}/${repoData.name}/issues`,
                        icon: <AlertCircle className="h-[18px] w-[18px]" />,
                    },
                    {
                        label: "Merge Requests",
                        href: `/${repoData.org}/${repoData.name}/merge-requests`,
                        icon: <GitMerge className="h-[18px] w-[18px]" />,
                    },
                ]}
                hasNotifications
            />

            <div className="flex flex-1 overflow-hidden">
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
                                            {repoData.defaultBranch}
                                        </span>
                                        <ChevronDown className="h-3.5 w-3.5 opacity-50" />
                                    </Button>
                                </DropdownMenuTrigger>
                                <DropdownMenuContent align="start" className="w-56">
                                    {repoData.branches.map((branch) => (
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
                                <span>{repoData.latestCommit.totalCommits}</span>
                            </Link>
                        </div>

                        <div className="flex items-start gap-2.5 text-sm text-muted-foreground">
                            <div className="flex h-6 w-6 items-center justify-center rounded-full bg-secondary text-xs font-medium shrink-0">
                                {repoData.latestCommit.author[0].toUpperCase()}
                            </div>
                            <div className="flex-1 min-w-0">
                                <div className="flex items-center gap-2">
                                    <span className="font-medium text-foreground">{repoData.latestCommit.author}</span>
                                    <span className="truncate">{repoData.latestCommit.message}</span>
                                </div>
                                <div className="flex items-center gap-2 mt-1 text-xs">
                                    <CIStatusBadge status={repoData.latestCommit.ciStatus} />
                                    <Link href="#" className="font-mono hover:text-foreground transition-colors flex items-center gap-1">
                                        <GitCommit className="h-3.5 w-3.5" />
                                        {repoData.latestCommit.hash}
                                    </Link>
                                    <span>{repoData.latestCommit.date}</span>
                                </div>
                            </div>
                        </div>
                    </div>

                    <div className="flex-1 overflow-y-auto py-2">
                        {fileTree.map((node) => (
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

                    <div
                        className="absolute right-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-ring/50 transition-colors"
                        onMouseDown={() => setIsResizing(true)}
                    />
                </aside>

                <main className="flex-1 flex flex-col min-w-0 overflow-hidden">
                    <div className="flex items-center justify-between px-5 py-3 border-b border-border shrink-0">
                        <div className="flex items-center gap-2.5">
                            <FileText className="h-[18px] w-[18px] text-muted-foreground" />
                            <span className="font-medium">{selectedFile || "README.md"}</span>
                            {((selectedFile && !selectedFile.endsWith(".md")) || showReadmeSource) && (
                                <span className="text-sm text-muted-foreground">2.4 KB</span>
                            )}
                        </div>
                        <div className="flex items-center gap-2">
                            {(!selectedFile || selectedFile.endsWith(".md")) && (
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
                            {((selectedFile && !selectedFile.endsWith(".md")) || showReadmeSource) && (
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
                                        className="h-8 px-3 gap-2 text-sm text-muted-foreground hover:text-foreground"
                                    >
                                        <ExternalLink className="h-3.5 w-3.5" />
                                        Raw
                                    </Button>
                                </div>
                            )}
                        </div>
                    </div>

                    <div className="flex-1 overflow-auto">
                        {selectedFile && fileContents[selectedFile] ? (
                            <CodeBlock content={fileContents[selectedFile]} filename={selectedFile} />
                        ) : (
                            <ReadmeView showSource={showReadmeSource} />
                        )}
                    </div>
                </main>

                <aside className="w-[340px] border-l border-border shrink-0 overflow-y-auto">
                    <div className="p-5 space-y-5">
                        <div>
                            <div className="flex items-center justify-between mb-2.5">
                                <h3 className="text-sm font-medium uppercase tracking-wider text-muted-foreground">Clone</h3>
                                <DropdownMenu>
                                    <DropdownMenuTrigger asChild>
                                        <Button
                                            variant="ghost"
                                            size="sm"
                                            className="h-6 px-2 gap-1 text-xs uppercase tracking-wider text-muted-foreground hover:text-foreground"
                                        >
                                            {protocol}
                                            <ChevronDown className="h-3 w-3" />
                                        </Button>
                                    </DropdownMenuTrigger>
                                    <DropdownMenuContent align="end">
                                        <DropdownMenuItem onClick={() => setProtocol("https")}>HTTPS</DropdownMenuItem>
                                        <DropdownMenuItem onClick={() => setProtocol("ssh")}>SSH</DropdownMenuItem>
                                    </DropdownMenuContent>
                                </DropdownMenu>
                            </div>
                            <div className="flex items-center rounded-md bg-card border border-border">
                                <code className="flex-1 truncate px-3 py-2 text-sm font-mono text-muted-foreground">{cloneUrl}</code>
                                <CopyButton text={cloneUrl} />
                            </div>
                        </div>

                        <div className="pt-4 border-t border-border">
                            <div className="flex items-center justify-between mb-3">
                                <h3 className="text-sm font-medium uppercase tracking-wider text-muted-foreground">About</h3>
                                <div className="flex items-center gap-1.5">
                                    <button className="flex items-center gap-1.5 px-2 py-1 text-sm text-muted-foreground hover:text-foreground border border-border rounded hover:bg-accent/50 transition-colors">
                                        <Star className="h-3.5 w-3.5" />
                                        <span>{repoData.stars}</span>
                                    </button>
                                    <button className="flex items-center gap-1.5 px-2 py-1 text-sm text-muted-foreground hover:text-foreground border border-border rounded hover:bg-accent/50 transition-colors">
                                        <GitFork className="h-3.5 w-3.5" />
                                        <span>{repoData.forks}</span>
                                    </button>
                                    <button className="flex items-center gap-1.5 px-2 py-1 text-sm text-muted-foreground hover:text-foreground border border-border rounded hover:bg-accent/50 transition-colors">
                                        <Eye className="h-3.5 w-3.5" />
                                        <span>{repoData.watchers}</span>
                                    </button>
                                </div>
                            </div>
                            <p className="text-foreground leading-relaxed mb-4">{repoData.description}</p>
                            {repoData.websiteUrl && (
                                <a
                                    href={repoData.websiteUrl}
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    className="inline-flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors mb-4"
                                >
                                    <ExternalLink className="h-3.5 w-3.5" />
                                    {repoData.websiteUrl.replace(/^https?:\/\//, "")}
                                </a>
                            )}
                            {repoData.topics && repoData.topics.length > 0 && (
                                <div className="flex flex-wrap gap-2 mb-4">
                                    {repoData.topics.map((topic) => (
                                        <Link
                                            key={topic}
                                            href="#"
                                            className="px-2.5 py-1 text-xs bg-secondary text-muted-foreground hover:text-foreground rounded-full transition-colors"
                                        >
                                            {topic}
                                        </Link>
                                    ))}
                                </div>
                            )}
                            <div className="space-y-2 text-sm">
                                <div className="flex items-center justify-between text-muted-foreground">
                                    <span>Project ID</span>
                                    <span className="font-mono text-foreground">{repoData.projectId}</span>
                                </div>
                                <div className="flex items-center justify-between text-muted-foreground">
                                    <span>Size</span>
                                    <span className="text-foreground">{repoData.size}</span>
                                </div>
                                <div className="flex items-center justify-between text-muted-foreground">
                                    <span>License</span>
                                    <span className="text-foreground flex items-center gap-1.5">
                                        <Scale className="h-3.5 w-3.5" />
                                        {repoData.license}
                                    </span>
                                </div>
                                <div className="flex items-center justify-between text-muted-foreground">
                                    <span>Created</span>
                                    <span className="text-foreground flex items-center gap-1.5">
                                        <Calendar className="h-3.5 w-3.5" />
                                        {repoData.createdAt}
                                    </span>
                                </div>
                            </div>
                        </div>

                        <div className="pt-4 border-t border-border">
                            <h3 className="text-sm font-medium uppercase tracking-wider text-muted-foreground mb-3">Languages</h3>
                            <LanguageBar languages={repoData.languages} />
                        </div>

                        <div className="pt-4 border-t border-border">
                            <div className="flex items-center justify-between mb-3">
                                <h3 className="text-sm font-medium uppercase tracking-wider text-muted-foreground">Releases</h3>
                                <Link href="#" className="text-xs text-muted-foreground hover:text-foreground transition-colors">
                                    View all
                                </Link>
                            </div>
                            <Link
                                href="#"
                                className="flex items-center gap-3 p-3 -mx-3 rounded-md hover:bg-accent/50 transition-colors group"
                            >
                                <Package className="h-5 w-5 text-muted-foreground shrink-0" />
                                <div className="min-w-0 flex-1">
                                    <div className="flex items-center gap-2">
                                        <span className="font-medium text-foreground">{repoData.latestRelease.tag}</span>
                                        <span className="px-2 py-0.5 text-[10px] uppercase tracking-wider bg-secondary text-muted-foreground rounded">
                                            Latest
                                        </span>
                                    </div>
                                    <div className="text-sm text-muted-foreground truncate">
                                        {repoData.latestRelease.name} · {repoData.latestRelease.date}
                                    </div>
                                </div>
                            </Link>
                        </div>

                        <div className="pt-4 border-t border-border">
                            <Link
                                href="#"
                                className="flex items-center justify-between text-sm text-muted-foreground hover:text-foreground transition-colors"
                            >
                                <span className="flex items-center gap-2">
                                    <Users className="h-4 w-4" />
                                    Contributors
                                </span>
                                <span className="flex items-center gap-2">
                                    <div className="flex -space-x-2">
                                        {repoData.contributors.slice(0, 3).map((c) => (
                                            <div
                                                key={c.name}
                                                className="flex h-6 w-6 items-center justify-center rounded-full bg-secondary text-xs font-medium border-2 border-background"
                                            >
                                                {c.name[0].toUpperCase()}
                                            </div>
                                        ))}
                                    </div>
                                    <span>{repoData.contributors.length}</span>
                                </span>
                            </Link>
                        </div>
                    </div>
                </aside>
            </div>
        </div>
    );
}

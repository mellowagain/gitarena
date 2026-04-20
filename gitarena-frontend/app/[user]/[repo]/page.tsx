"use client";

import { FileText, AlertCircle, GitMerge, ExternalLink, Code, BookOpen, Edit3 } from "lucide-react";
import Link from "next/link";
import { use, useState } from "react";
import { Button } from "@/components/ui/button";
import { TopBar } from "@/components/top-bar";
import { RepoFileSidebar, RepoFileSidebarSkeleton } from "@/components/repo-file-sidebar";
import { RepoSidebar, RepoSidebarSkeleton } from "@/components/repo-sidebar";
import useSWR from "swr";
import { ErrorDisplay } from "@/components/error-display";

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

    const { data, error, isLoading } = useSWR<RepoMetadata>(`http://localhost:8080/api/repos/${user}/${repo}`);

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
                    branches={repoData.branches}
                    latestCommit={repoData.latestCommit}
                />

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

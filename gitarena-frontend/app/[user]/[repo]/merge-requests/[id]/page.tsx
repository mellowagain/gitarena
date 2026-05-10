"use client";

import { useState } from "react";
import Link from "next/link";
import { TopBar } from "@/components/top-bar";
import { Button } from "@/components/ui/button";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
    DropdownMenuSeparator,
} from "@/components/ui/dropdown-menu";
import {
    AlertCircle,
    GitMerge,
    ChevronDown,
    CheckCircle2,
    MessageSquare,
    MoreHorizontal,
    Code,
    ArrowLeft,
    Calendar,
    User,
    Tag,
    Link as LinkIcon,
    Edit,
    Trash2,
    Clock,
    GitBranch,
    Smile,
    FileCode,
    GitCommit,
    GitPullRequest,
    Check,
    X,
    Play,
    ChevronRight,
    Eye,
    Plus,
    Minus,
    Copy,
    XCircle,
} from "lucide-react";

const currentUser = { name: "Mari" };
const repoData = { org: "mellowagain", name: "test" };

type MRStatus = "open" | "merged" | "closed" | "draft";
type ReviewStatus = "pending" | "approved" | "changes_requested";
type CIStatus = "pending" | "running" | "passed" | "failed";

const mergeRequest = {
    id: 15,
    title: "feat: Add SSH key authentication support",
    description: `## Summary
This PR implements SSH key authentication for git operations.

## Changes
- Added SSH key parsing and validation
- Implemented key storage and fingerprint generation
- Added authentication flow for \`git-receive-pack\` and \`git-upload-pack\`
- Added user settings UI for SSH key management

## Testing
- [x] Unit tests for key parsing
- [x] Integration tests for auth flow
- [x] Manual testing with different key types

## Screenshots
N/A — backend changes only

Closes #42`,
    status: "open" as MRStatus,
    author: "mellowagain",
    createdAt: "1 day ago",
    updatedAt: "2 hours ago",
    sourceBranch: "feature/ssh-auth",
    targetBranch: "main",
    commits: 8,
    additions: 342,
    deletions: 28,
    filesChanged: 12,
    labels: [
        { name: "enhancement", color: "#a2eeef" },
        { name: "component::auth", color: "#d73a4a" },
    ],
    reviewers: [
        { name: "Mari", status: "approved" as ReviewStatus },
        { name: "contributor1", status: "pending" as ReviewStatus },
    ],
    ciStatus: "passed" as CIStatus,
    ciJobs: [
        { name: "build", status: "passed" as CIStatus },
        { name: "test", status: "passed" as CIStatus },
        { name: "lint", status: "passed" as CIStatus },
    ],
    conflicts: false,
    linkedIssue: { id: 42, title: "Add support for SSH key authentication" },
};

type ActivityItem =
    | {
          type: "comment";
          id: string;
          author: string;
          isBot?: boolean;
          content: string;
          createdAt: string;
          reviewType?: "approved" | "changes_requested";
          reactions: { emoji: string; count: number; reacted: boolean }[];
      }
    | {
          type: "diff_comment";
          id: string;
          author: string;
          content: string;
          createdAt: string;
          file: string;
          lines: { lineNo: number; type: "context" | "add" | "remove"; content: string }[];
          reactions: { emoji: string; count: number; reacted: boolean }[];
      }
    | { type: "event"; id: string; author: string; isBot?: boolean; action: string; detail?: string; createdAt: string };

const activity: ActivityItem[] = [
    {
        type: "comment",
        id: 1,
        author: "contributor1",
        content:
            "Nice work! Just a few minor suggestions on the error handling.\n\nThe `parse_key` function should probably return a `Result<SSHKey, ParseError>` rather than panicking on bad input.",
        createdAt: "1 day ago",
        reactions: [{ emoji: "👍", count: 1, reacted: false }],
    },
    {
        type: "diff_comment",
        id: 2,
        author: "contributor1",
        file: "src/auth/ssh_key.rs",
        lines: [
            { lineNo: 42, type: "context", content: "pub fn parse_key(raw: &str) -> SSHKey {" },
            { lineNo: 43, type: "remove", content: '    if raw.is_empty() { panic!("empty key"); }' },
            { lineNo: 43, type: "add", content: "    if raw.is_empty() { return Err(ParseError::EmptyKey); }" },
            { lineNo: 44, type: "context", content: "    let parts: Vec<&str> = raw.split(' ').collect();" },
        ],
        content: "This should never panic in production. Please handle the error gracefully and propagate it up to the caller.",
        createdAt: "1 day ago",
        reactions: [],
    },
    {
        type: "event",
        id: 3,
        author: "mellowagain",
        action: "pushed 2 commits to",
        detail: "feature/ssh-auth",
        createdAt: "20 hours ago",
    },
    {
        type: "event",
        id: 4,
        author: "gitarena-ci",
        isBot: true,
        action: "set status to",
        detail: "All checks passed",
        createdAt: "20 hours ago",
    },
    {
        type: "comment",
        id: 5,
        author: "gitarena-ci",
        isBot: true,
        content: "All CI checks passed.\n\n- build `passed` in 1m 24s\n- test `passed` in 3m 12s\n- lint `passed` in 0m 48s",
        createdAt: "20 hours ago",
        reactions: [],
    },
    {
        type: "comment",
        id: 6,
        author: "Mari",
        content:
            "LGTM! The implementation looks solid. The error handling is much better now.\n\n- [x] Key parsing\n- [x] Fingerprint generation\n- [x] Auth flow\n\nApproved.",
        createdAt: "2 hours ago",
        reviewType: "approved",
        reactions: [{ emoji: "🎉", count: 2, reacted: true }],
    },
    {
        type: "event",
        id: 7,
        author: "mellowagain",
        action: "reopened this merge request",
        createdAt: "1 hour ago",
    },
];

const commitHistory = [
    {
        hash: "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2",
        shortHash: "a1b2c3d",
        message: "feat: add SSH key parsing",
        author: "mellowagain",
        date: "1 day ago",
        ci: "passed" as CIStatus,
    },
    {
        hash: "e4f5g6h7i8j9e4f5g6h7i8j9e4f5g6h7i8j9e4f5",
        shortHash: "e4f5g6h",
        message: "feat: implement fingerprint generation",
        author: "mellowagain",
        date: "1 day ago",
        ci: "passed" as CIStatus,
    },
    {
        hash: "i7j8k9l0m1n2i7j8k9l0m1n2i7j8k9l0m1n2i7j8",
        shortHash: "i7j8k9l",
        message: "feat: add auth flow integration",
        author: "mellowagain",
        date: "20 hours ago",
        ci: "passed" as CIStatus,
    },
    {
        hash: "m0n1o2p3q4r5m0n1o2p3q4r5m0n1o2p3q4r5m0n1",
        shortHash: "m0n1o2p",
        message: "test: add unit tests for key parsing",
        author: "mellowagain",
        date: "18 hours ago",
        ci: "passed" as CIStatus,
    },
    {
        hash: "q3r4s5t6u7v8q3r4s5t6u7v8q3r4s5t6u7v8q3r4",
        shortHash: "q3r4s5t",
        message: "fix: handle invalid key formats gracefully",
        author: "mellowagain",
        date: "5 hours ago",
        ci: "passed" as CIStatus,
    },
];

const statusConfig: Record<MRStatus, { icon: typeof GitPullRequest; label: string; color: string; bg: string }> = {
    open: { icon: GitPullRequest, label: "Open", color: "text-green-500", bg: "bg-green-500/10" },
    draft: { icon: GitPullRequest, label: "Draft", color: "text-muted-foreground", bg: "bg-muted" },
    merged: { icon: GitMerge, label: "Merged", color: "text-purple-500", bg: "bg-purple-500/10" },
    closed: { icon: X, label: "Closed", color: "text-red-500", bg: "bg-red-500/10" },
};

const reviewStatusConfig: Record<ReviewStatus, { icon: typeof Clock; color: string }> = {
    pending: { icon: Clock, color: "text-muted-foreground" },
    approved: { icon: CheckCircle2, color: "text-green-500" },
    changes_requested: { icon: AlertCircle, color: "text-red-500" },
};

const ciStatusConfig: Record<CIStatus, { icon: typeof Clock; color: string; label: string }> = {
    pending: { icon: Clock, color: "text-muted-foreground", label: "Pending" },
    running: { icon: Play, color: "text-amber-500", label: "Running" },
    passed: { icon: CheckCircle2, color: "text-green-500", label: "Passed" },
    failed: { icon: X, color: "text-red-500", label: "Failed" },
};

function highlightCode(code: string, lang: string): React.ReactNode {
    const isRust = lang === "rust" || lang === "rs";
    if (!isRust) {
        return code;
    }

    const keywords =
        /\b(pub|fn|struct|impl|let|mut|use|mod|match|if|else|return|for|in|while|loop|enum|type|trait|where|self|Self|crate|super|async|await|move|ref|const|static|extern|unsafe|true|false|Some|None|Ok|Err)\b/g;
    const types =
        /\b(i8|i16|i32|i64|i128|u8|u16|u32|u64|u128|f32|f64|usize|isize|bool|str|String|Vec|Option|Result|Box|Arc|Rc|DateTime|Utc|ParseError|KeyType|SSHKey)\b/g;
    const strings = /"([^"\\]|\\.)*"/g;
    const comments = /\/\/.*/g;
    const numbers = /\b\d+\b/g;
    const attrs = /#\[.*?\]/g;
    const macros = /\b\w+!/g;

    const tokens: { start: number; end: number; cls: string }[] = [];

    function findAll(re: RegExp, cls: string) {
        let m: RegExpExecArray | null;
        const r = new RegExp(re.source, re.flags.includes("g") ? re.flags : re.flags + "g");
        while ((m = r.exec(code)) !== null) {
            tokens.push({ start: m.index, end: m.index + m[0].length, cls });
        }
    }

    findAll(comments, "text-muted-foreground/60 italic");
    findAll(strings, "text-green-400");
    findAll(attrs, "text-yellow-400/80");
    findAll(macros, "text-yellow-300");
    findAll(keywords, "text-blue-400 font-medium");
    findAll(types, "text-cyan-400");
    findAll(numbers, "text-orange-400");

    tokens.sort((a, b) => a.start - b.start);
    const merged: typeof tokens = [];
    let cursor = 0;
    for (const t of tokens) {
        if (t.start < cursor) {
            continue;
        }
        merged.push(t);
        cursor = t.end;
    }

    const nodes: React.ReactNode[] = [];
    let pos = 0;
    let ki = 0;
    for (const t of merged) {
        if (t.start > pos) {
            nodes.push(<span key={ki++}>{code.slice(pos, t.start)}</span>);
        }
        nodes.push(
            <span key={ki++} className={t.cls}>
                {code.slice(t.start, t.end)}
            </span>
        );
        pos = t.end;
    }
    if (pos < code.length) {
        nodes.push(<span key={ki++}>{code.slice(pos)}</span>);
    }
    return <>{nodes}</>;
}

function renderMarkdown(text: string): React.ReactNode[] {
    const lines = text.split("\n");
    const nodes: React.ReactNode[] = [];
    let i = 0;
    let keyCounter = 0;
    const k = () => keyCounter++;

    while (i < lines.length) {
        const line = lines[i];

        if (line.startsWith("```")) {
            const lang = line.slice(3).trim();
            const codeLines: string[] = [];
            i++;
            while (i < lines.length && !lines[i].startsWith("```")) {
                codeLines.push(lines[i]);
                i++;
            }
            nodes.push(
                <pre key={k()} className="my-3 rounded-md bg-secondary px-4 py-3 overflow-x-auto">
                    {lang && <div className="text-xs text-muted-foreground mb-2 font-mono">{lang}</div>}
                    <code className="text-sm font-mono leading-relaxed">{highlightCode(codeLines.join("\n"), lang)}</code>
                </pre>
            );
            i++;
            continue;
        }

        const headingMatch = line.match(/^(#{1,3})\s+(.+)/);
        if (headingMatch) {
            const level = headingMatch[1].length;
            const cls =
                level === 1
                    ? "text-lg font-semibold mt-4 mb-2"
                    : level === 2
                      ? "text-base font-semibold mt-4 mb-1.5"
                      : "text-sm font-semibold mt-3 mb-1";
            nodes.push(
                <div key={k()} className={cls}>
                    {headingMatch[2]}
                </div>
            );
            i++;
            continue;
        }

        if (line.match(/^- \[[ x]\]/)) {
            const items: React.ReactNode[] = [];
            while (i < lines.length && lines[i].match(/^- \[[ x]\]/)) {
                const checked = lines[i][3] === "x";
                items.push(
                    <li key={k()} className="flex items-start gap-2">
                        <span className={`mt-0.5 text-xs ${checked ? "text-green-500" : "text-muted-foreground"}`}>
                            {checked ? "☑" : "☐"}
                        </span>
                        <span className={checked ? "line-through text-muted-foreground/60" : ""}>{lines[i].slice(6)}</span>
                    </li>
                );
                i++;
            }
            nodes.push(
                <ul key={k()} className="space-y-1 my-2 text-sm">
                    {items}
                </ul>
            );
            continue;
        }

        if (line.startsWith("- ")) {
            const items: React.ReactNode[] = [];
            while (i < lines.length && lines[i].startsWith("- ")) {
                items.push(
                    <li key={k()} className="ml-4 list-disc">
                        {inlineMarkdown(lines[i].slice(2))}
                    </li>
                );
                i++;
            }
            nodes.push(
                <ul key={k()} className="space-y-1 my-2 text-sm">
                    {items}
                </ul>
            );
            continue;
        }

        if (line.trim() === "") {
            i++;
            continue;
        }

        nodes.push(
            <p key={k()} className="text-sm leading-relaxed my-1">
                {inlineMarkdown(line)}
            </p>
        );
        i++;
    }
    return nodes;
}

function inlineMarkdown(text: string): React.ReactNode {
    const parts = text.split(/(\*\*[^*]+\*\*|`[^`]+`|\[[^\]]+\]\([^)]+\))/);
    return parts.map((part, i) => {
        if (part.startsWith("**") && part.endsWith("**")) {
            return <strong key={i}>{part.slice(2, -2)}</strong>;
        }
        if (part.startsWith("`") && part.endsWith("`")) {
            return (
                <code key={i} className="px-1.5 py-0.5 bg-secondary rounded text-xs font-mono">
                    {part.slice(1, -1)}
                </code>
            );
        }
        const linkMatch = part.match(/^\[([^\]]+)\]\(([^)]+)\)$/);
        if (linkMatch) {
            return (
                <a key={i} href={linkMatch[2]} className="text-foreground underline underline-offset-2 hover:opacity-80">
                    {linkMatch[1]}
                </a>
            );
        }
        return part;
    });
}

function LabelBadge({ label, removable }: { label: { name: string; color: string }; removable?: boolean }) {
    const scopedIndex = label.name.indexOf("::");
    const isScoped = scopedIndex !== -1;
    const scopeKey = isScoped ? label.name.slice(0, scopedIndex) : null;
    const scopeValue = isScoped ? label.name.slice(scopedIndex + 2) : null;

    if (isScoped) {
        return (
            <span className={`group/label inline-flex items-center text-xs rounded overflow-hidden ${removable ? "" : ""}`}>
                <span className="px-2 py-0.5 font-medium" style={{ backgroundColor: `${label.color}35`, color: label.color }}>
                    {scopeKey}
                </span>
                <span className="px-2 py-0.5" style={{ backgroundColor: `${label.color}20`, color: label.color }}>
                    {scopeValue}
                </span>
                {removable && (
                    <span
                        className="grid grid-cols-[0fr] group-hover/label:grid-cols-[1fr] transition-all duration-150 self-stretch"
                        style={{ backgroundColor: `${label.color}20` }}
                    >
                        <span className="overflow-hidden flex items-center">
                            <button
                                className="ml-1.5 mr-1.5 opacity-0 group-hover/label:opacity-100 transition-opacity rounded"
                                title="Remove label"
                                style={{ color: label.color }}
                            >
                                <X className="h-2.5 w-2.5" />
                            </button>
                        </span>
                    </span>
                )}
            </span>
        );
    }

    return (
        <span
            className="group/label inline-flex items-center px-2 py-0.5 text-xs rounded"
            style={{ backgroundColor: `${label.color}20`, color: label.color }}
        >
            {label.name}
            {removable && (
                <span className="grid grid-cols-[0fr] group-hover/label:grid-cols-[1fr] transition-all duration-150">
                    <span className="overflow-hidden flex items-center">
                        <button className="ml-1.5 opacity-0 group-hover/label:opacity-100 transition-opacity rounded" title="Remove label">
                            <X className="h-2.5 w-2.5" />
                        </button>
                    </span>
                </span>
            )}
        </span>
    );
}

function AuthorAvatar({ author, isBot }: { author: string; isBot?: boolean }) {
    if (isBot) {
        return (
            <div className="flex h-5 w-5 items-center justify-center rounded-md bg-secondary border border-border shrink-0" title="Bot">
                <span className="text-[9px] font-bold text-muted-foreground">B</span>
            </div>
        );
    }
    return (
        <div className="flex h-5 w-5 items-center justify-center rounded-full bg-secondary text-[10px] font-medium shrink-0">
            {author[0].toUpperCase()}
        </div>
    );
}

function ActivityEvent({ item }: { item: Extract<ActivityItem, { type: "event" }> }) {
    return (
        <div className="flex items-center gap-2 text-sm text-muted-foreground py-1">
            <AuthorAvatar author={item.author} isBot={item.isBot} />
            <span className="font-medium text-foreground/70">{item.author}</span>
            {item.isBot && (
                <span className="text-[10px] px-1 py-0.5 rounded bg-secondary border border-border text-muted-foreground font-medium leading-none">
                    bot
                </span>
            )}
            <span>{item.action}</span>
            {item.detail && (
                <span className="px-1.5 py-0.5 bg-secondary rounded text-xs font-medium text-foreground/80">{item.detail}</span>
            )}
            <span className="ml-auto shrink-0 text-xs">{item.createdAt}</span>
        </div>
    );
}

function DiffCommentBlock({ item }: { item: Extract<ActivityItem, { type: "diff_comment" }> }) {
    return (
        <div className="group border border-border rounded-md overflow-hidden">
            <div className="flex items-center gap-2 px-3 py-1.5 bg-secondary/60 border-b border-border">
                <FileCode className="h-3.5 w-3.5 text-muted-foreground" />
                <span className="text-xs font-mono text-muted-foreground">{item.file}</span>
            </div>
            <div className="font-mono text-xs overflow-x-auto">
                {item.lines.map((line, i) => (
                    <div
                        key={i}
                        className={`flex items-start gap-3 px-3 py-0.5 ${
                            line.type === "add"
                                ? "bg-green-500/8 text-green-400"
                                : line.type === "remove"
                                  ? "bg-red-500/8 text-red-400"
                                  : "text-muted-foreground"
                        }`}
                    >
                        <span className="select-none w-6 text-right shrink-0 opacity-50">{line.lineNo}</span>
                        <span className="select-none w-3 shrink-0">{line.type === "add" ? "+" : line.type === "remove" ? "−" : " "}</span>
                        <span className="whitespace-pre">{line.content}</span>
                    </div>
                ))}
            </div>
            <div className="px-4 py-3 border-t border-border bg-card/40">
                <div className="flex items-center gap-2 mb-2">
                    <div className="flex h-5 w-5 items-center justify-center rounded-full bg-secondary text-[10px] font-medium shrink-0">
                        {item.author[0].toUpperCase()}
                    </div>
                    <span className="text-sm font-medium">{item.author}</span>
                    <span className="text-xs text-muted-foreground">{item.createdAt}</span>
                    <div className="flex items-center gap-1 ml-auto opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none group-hover:pointer-events-auto">
                        <button className="p-1 hover:bg-accent rounded transition-colors">
                            <Smile className="h-3.5 w-3.5 text-muted-foreground" />
                        </button>
                        <DropdownMenu>
                            <DropdownMenuTrigger asChild>
                                <button className="p-1 hover:bg-accent rounded transition-colors">
                                    <MoreHorizontal className="h-3.5 w-3.5 text-muted-foreground" />
                                </button>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent align="end">
                                <DropdownMenuItem>
                                    <Edit className="h-4 w-4 mr-2" />
                                    Edit
                                </DropdownMenuItem>
                                <DropdownMenuItem className="text-destructive">
                                    <Trash2 className="h-4 w-4 mr-2" />
                                    Delete
                                </DropdownMenuItem>
                            </DropdownMenuContent>
                        </DropdownMenu>
                    </div>
                </div>
                <div className="text-sm leading-relaxed text-foreground/80">{renderMarkdown(item.content)}</div>
            </div>
        </div>
    );
}

function CommentBlock({ item }: { item: Extract<ActivityItem, { type: "comment" }> }) {
    const isApproval = item.reviewType === "approved";
    const isChangesRequested = item.reviewType === "changes_requested";

    return (
        <div
            className={`group pl-4 border-l-2 transition-colors ${
                isApproval
                    ? "border-green-500/50 hover:border-green-500"
                    : isChangesRequested
                      ? "border-red-500/50 hover:border-red-500"
                      : item.isBot
                        ? "border-secondary hover:border-muted-foreground/30"
                        : "border-border hover:border-muted-foreground/40"
            }`}
        >
            <div className="flex items-center gap-2 mb-2">
                <AuthorAvatar author={item.author} isBot={item.isBot} />
                <span className="text-sm font-medium">{item.author}</span>
                {item.isBot && (
                    <span className="text-[10px] px-1 py-0.5 rounded bg-secondary border border-border text-muted-foreground font-medium leading-none">
                        bot
                    </span>
                )}
                {isApproval && (
                    <span className="flex items-center gap-1 text-xs text-green-500 bg-green-500/10 px-2 py-0.5 rounded-full">
                        <CheckCircle2 className="h-3 w-3" />
                        approved
                    </span>
                )}
                {isChangesRequested && (
                    <span className="flex items-center gap-1 text-xs text-red-500 bg-red-500/10 px-2 py-0.5 rounded-full">
                        <AlertCircle className="h-3 w-3" />
                        requested changes
                    </span>
                )}
                <span className="text-xs text-muted-foreground">{item.createdAt}</span>
                <div className="flex items-center gap-1 ml-auto opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none group-hover:pointer-events-auto">
                    <button className="p-1 hover:bg-accent rounded transition-colors">
                        <Smile className="h-3.5 w-3.5 text-muted-foreground" />
                    </button>
                    <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                            <button className="p-1 hover:bg-accent rounded transition-colors">
                                <MoreHorizontal className="h-3.5 w-3.5 text-muted-foreground" />
                            </button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                            <DropdownMenuItem>
                                <Edit className="h-4 w-4 mr-2" />
                                Edit
                            </DropdownMenuItem>
                            <DropdownMenuItem className="text-destructive">
                                <Trash2 className="h-4 w-4 mr-2" />
                                Delete
                            </DropdownMenuItem>
                        </DropdownMenuContent>
                    </DropdownMenu>
                </div>
            </div>
            <div className="text-sm leading-relaxed text-foreground/80">{renderMarkdown(item.content)}</div>
            {item.reactions.length > 0 && (
                <div className="flex items-center gap-1.5 mt-2">
                    {item.reactions.map((reaction, i) => (
                        <button
                            key={i}
                            className={`flex items-center gap-1 px-1.5 py-0.5 text-xs rounded transition-colors ${reaction.reacted ? "bg-accent text-foreground" : "bg-secondary text-muted-foreground hover:text-foreground"}`}
                        >
                            <span>{reaction.emoji}</span>
                            <span>{reaction.count}</span>
                        </button>
                    ))}
                </div>
            )}
        </div>
    );
}

function CommentComposer() {
    const [text, setText] = useState("");
    const [preview, setPreview] = useState(false);

    return (
        <div className="flex items-start gap-3">
            <div className="flex h-6 w-6 items-center justify-center rounded-full bg-secondary text-xs font-medium shrink-0 mt-1">
                {currentUser.name[0].toUpperCase()}
            </div>
            <div className="flex-1 border border-border rounded-md overflow-hidden focus-within:ring-1 focus-within:ring-ring/50">
                <div className="flex items-center border-b border-border">
                    <button
                        onClick={() => setPreview(false)}
                        className={`px-3 py-1.5 text-xs font-medium transition-colors ${!preview ? "text-foreground border-b-2 border-foreground -mb-px" : "text-muted-foreground hover:text-foreground"}`}
                    >
                        Write
                    </button>
                    <button
                        onClick={() => setPreview(true)}
                        className={`px-3 py-1.5 text-xs font-medium transition-colors flex items-center gap-1.5 ${preview ? "text-foreground border-b-2 border-foreground -mb-px" : "text-muted-foreground hover:text-foreground"}`}
                    >
                        <Eye className="h-3 w-3" />
                        Preview
                    </button>
                </div>
                {preview ? (
                    <div className="px-3 py-2 min-h-[80px] text-sm text-foreground/80">
                        {text.trim() ? renderMarkdown(text) : <span className="text-muted-foreground italic">Nothing to preview.</span>}
                    </div>
                ) : (
                    <textarea
                        value={text}
                        onChange={(e) => setText(e.target.value)}
                        placeholder="Leave a comment... Markdown supported"
                        rows={3}
                        className="w-full px-3 py-2 bg-transparent text-sm resize-none focus:outline-none"
                    />
                )}
                <div className="flex items-center justify-between px-3 py-2 border-t border-border bg-card/50">
                    <span className="text-xs text-muted-foreground font-mono">Markdown</span>
                    <div className="flex items-center gap-2">
                        <DropdownMenu>
                            <DropdownMenuTrigger asChild>
                                <Button variant="ghost" size="sm" className="gap-1.5 text-sm">
                                    Start review
                                    <ChevronDown className="h-3.5 w-3.5" />
                                </Button>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent align="end">
                                <DropdownMenuItem>
                                    <CheckCircle2 className="h-4 w-4 mr-2 text-green-500" />
                                    Approve
                                </DropdownMenuItem>
                                <DropdownMenuItem>
                                    <AlertCircle className="h-4 w-4 mr-2 text-red-500" />
                                    Request changes
                                </DropdownMenuItem>
                                <DropdownMenuItem>
                                    <MessageSquare className="h-4 w-4 mr-2" />
                                    Comment only
                                </DropdownMenuItem>
                            </DropdownMenuContent>
                        </DropdownMenu>
                        <Button size="sm" disabled={!text.trim()}>
                            Comment
                        </Button>
                    </div>
                </div>
            </div>
        </div>
    );
}

type DiffLine = { type: "context" | "add" | "remove"; lineOld: number | null; lineNew: number | null; content: string };
type Hunk = { header: string; lines: DiffLine[] };

function FileDiff({ filename, additions, deletions, hunks }: { filename: string; additions: number; deletions: number; hunks: Hunk[] }) {
    const [collapsed, setCollapsed] = useState(false);

    return (
        <div className="border border-border rounded-md overflow-hidden">
            <div className="flex items-center gap-3 px-4 py-2.5 bg-secondary/40 border-b border-border">
                <button onClick={() => setCollapsed(!collapsed)} className="shrink-0">
                    <ChevronDown className={`h-4 w-4 text-muted-foreground transition-transform ${collapsed ? "-rotate-90" : ""}`} />
                </button>
                <FileCode className="h-4 w-4 text-muted-foreground shrink-0" />
                <span className="text-sm font-mono flex-1">{filename}</span>
                <span className="text-xs text-green-500 font-medium">+{additions}</span>
                <span className="text-xs text-red-500 font-medium">-{deletions}</span>
            </div>
            {!collapsed && (
                <div className="overflow-x-auto">
                    {hunks.map((hunk, hi) => (
                        <div key={hi}>
                            <div className="px-4 py-1 bg-blue-500/5 text-xs font-mono text-blue-400/70 border-b border-border/50">
                                {hunk.header}
                            </div>
                            {hunk.lines.map((line, li) => (
                                <div
                                    key={li}
                                    className={`flex font-mono text-xs leading-5 ${
                                        line.type === "add"
                                            ? "bg-green-500/8 text-green-300"
                                            : line.type === "remove"
                                              ? "bg-red-500/8 text-red-400"
                                              : "text-muted-foreground"
                                    }`}
                                >
                                    <span className="select-none w-12 px-2 py-0.5 text-right shrink-0 text-muted-foreground/40 border-r border-border/50">
                                        {line.lineOld ?? ""}
                                    </span>
                                    <span className="select-none w-12 px-2 py-0.5 text-right shrink-0 text-muted-foreground/40 border-r border-border/50">
                                        {line.lineNew ?? ""}
                                    </span>
                                    <span className="select-none w-5 px-1 py-0.5 shrink-0 text-center">
                                        {line.type === "add" ? "+" : line.type === "remove" ? "−" : " "}
                                    </span>
                                    <span className="px-2 py-0.5 whitespace-pre flex-1">{line.content}</span>
                                </div>
                            ))}
                        </div>
                    ))}
                </div>
            )}
        </div>
    );
}

export default function MergeRequestPage() {
    const [activeTab, setActiveTab] = useState<"conversation" | "commits" | "changes">("conversation");
    const [copiedHash, setCopiedHash] = useState<string | null>(null);
    const statusInfo = statusConfig[mergeRequest.status];
    const StatusIcon = statusInfo.icon;
    const CIIcon = ciStatusConfig[mergeRequest.ciStatus].icon;

    function copyHash(hash: string) {
        navigator.clipboard.writeText(hash);
        setCopiedHash(hash);
        setTimeout(() => setCopiedHash(null), 1500);
    }

    return (
        <div className="min-h-screen bg-background flex flex-col">
            <TopBar
                breadcrumb={[
                    { label: repoData.org, href: `/${repoData.org}` },
                    { label: repoData.name, href: `/${repoData.org}/${repoData.name}` },
                    { label: "Merge Requests", href: `/${repoData.org}/${repoData.name}/merge-requests` },
                    { label: `!${mergeRequest.id}` },
                ]}
                navLinks={[
                    { label: "Code", href: `/${repoData.org}/${repoData.name}`, icon: <Code className="h-[18px] w-[18px]" /> },
                    {
                        label: "Issues",
                        href: `/${repoData.org}/${repoData.name}/issues`,
                        icon: <AlertCircle className="h-[18px] w-[18px]" />,
                    },
                    {
                        label: "Merge Requests",
                        href: `/${repoData.org}/${repoData.name}/merge-requests`,
                        icon: <GitMerge className="h-[18px] w-[18px]" />,
                        active: true,
                    },
                ]}
                hasNotifications
            />

            <div className="flex-1 flex overflow-hidden">
                <main className="flex-1 overflow-y-auto">
                    <div className="max-w-4xl mx-auto px-6 py-8">
                        <Link
                            href={`/${repoData.org}/${repoData.name}/merge-requests`}
                            className="inline-flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors mb-6"
                        >
                            <ArrowLeft className="h-4 w-4" />
                            Back to merge requests
                        </Link>

                        <div className="mb-6">
                            <div className="flex items-start gap-4 mb-3">
                                <h1 className="text-2xl font-semibold flex-1 leading-snug">
                                    <span className="text-muted-foreground font-normal">!{mergeRequest.id}</span> {mergeRequest.title}
                                </h1>
                                <DropdownMenu>
                                    <DropdownMenuTrigger asChild>
                                        <Button variant="ghost" size="sm" className="shrink-0">
                                            <MoreHorizontal className="h-5 w-5" />
                                        </Button>
                                    </DropdownMenuTrigger>
                                    <DropdownMenuContent align="end">
                                        <DropdownMenuItem>
                                            <Edit className="h-4 w-4 mr-2" />
                                            Edit
                                        </DropdownMenuItem>
                                        <DropdownMenuItem>
                                            <LinkIcon className="h-4 w-4 mr-2" />
                                            Copy link
                                        </DropdownMenuItem>
                                        <DropdownMenuSeparator />
                                        <DropdownMenuItem className="text-destructive">
                                            <X className="h-4 w-4 mr-2" />
                                            Close
                                        </DropdownMenuItem>
                                    </DropdownMenuContent>
                                </DropdownMenu>
                            </div>
                            <div className="flex items-center gap-2.5 text-sm text-muted-foreground flex-wrap">
                                <StatusIcon className={`h-4 w-4 ${statusInfo.color}`} />
                                <span className={`font-medium ${statusInfo.color}`}>{statusInfo.label}</span>
                                <span className="text-muted-foreground/40">·</span>
                                <div className="flex items-center gap-1.5">
                                    <GitBranch className="h-3.5 w-3.5" />
                                    <Link href={`/${repoData.org}/${repoData.name}?branch=${mergeRequest.sourceBranch}`}>
                                        <code className="px-2 py-0.5 bg-secondary rounded text-xs hover:bg-accent transition-colors cursor-pointer">
                                            {mergeRequest.sourceBranch}
                                        </code>
                                    </Link>
                                    <ChevronRight className="h-3.5 w-3.5" />
                                    <Link href={`/${repoData.org}/${repoData.name}?branch=${mergeRequest.targetBranch}`}>
                                        <code className="px-2 py-0.5 bg-secondary rounded text-xs hover:bg-accent transition-colors cursor-pointer">
                                            {mergeRequest.targetBranch}
                                        </code>
                                    </Link>
                                </div>
                                <span className="text-muted-foreground/40">·</span>
                                <span className="flex items-center gap-1">
                                    <MessageSquare className="h-3.5 w-3.5" />
                                    {activity.filter((a) => a.type === "comment" || a.type === "diff_comment").length} comments
                                </span>
                                {(() => {
                                    const reviewCount = activity.filter(
                                        (a) => a.type === "comment" && (a as { reviewType?: string }).reviewType
                                    ).length;
                                    return reviewCount > 0 ? (
                                        <>
                                            <span className="text-muted-foreground/40">·</span>
                                            <span className="flex items-center gap-1">
                                                <CheckCircle2 className="h-3.5 w-3.5 text-green-500" />
                                                {reviewCount} {reviewCount === 1 ? "review" : "reviews"}
                                            </span>
                                        </>
                                    ) : null;
                                })()}
                            </div>
                        </div>

                        <div className="flex items-center gap-1 border-b border-border mb-6">
                            {(["conversation", "commits", "changes"] as const).map((tab) => {
                                const icons = { conversation: MessageSquare, commits: GitCommit, changes: FileCode };
                                const labels = { conversation: "Conversation", commits: `Commits`, changes: "Changes" };
                                const counts = {
                                    conversation: activity.filter((a) => a.type === "comment" || a.type === "diff_comment").length,
                                    commits: mergeRequest.commits,
                                    changes: mergeRequest.filesChanged,
                                };
                                const Icon = icons[tab];
                                return (
                                    <button
                                        key={tab}
                                        onClick={() => setActiveTab(tab)}
                                        className={`px-4 py-2 text-sm font-medium border-b-2 -mb-px transition-colors ${activeTab === tab ? "border-foreground text-foreground" : "border-transparent text-muted-foreground hover:text-foreground"}`}
                                    >
                                        <span className="flex items-center gap-2">
                                            <Icon className="h-4 w-4" />
                                            {labels[tab]}
                                            <span className="text-xs opacity-60">{counts[tab]}</span>
                                        </span>
                                    </button>
                                );
                            })}
                        </div>

                        {activeTab === "conversation" && (
                            <>
                                <div className="group/desc mb-8 pl-5 border-l-4 border-muted-foreground/20 hover:border-muted-foreground/40 transition-colors">
                                    <div className="flex items-center gap-2 mb-3">
                                        <div className="flex h-6 w-6 items-center justify-center rounded-full bg-secondary text-xs font-medium shrink-0">
                                            {mergeRequest.author[0].toUpperCase()}
                                        </div>
                                        <span className="text-sm font-medium">{mergeRequest.author}</span>
                                        <span className="text-xs text-muted-foreground">opened {mergeRequest.createdAt}</span>
                                        <div className="ml-auto flex items-center gap-1 opacity-0 group-hover/desc:opacity-100 transition-opacity">
                                            <button className="p-1 hover:bg-accent rounded transition-colors">
                                                <Smile className="h-3.5 w-3.5 text-muted-foreground" />
                                            </button>
                                            <button className="p-1 hover:bg-accent rounded transition-colors">
                                                <Edit className="h-3.5 w-3.5 text-muted-foreground" />
                                            </button>
                                        </div>
                                    </div>
                                    <div className="text-sm text-foreground/80">{renderMarkdown(mergeRequest.description)}</div>
                                    <div className="flex items-center gap-1.5 mt-3">
                                        <button className="flex items-center gap-1 px-1.5 py-0.5 text-xs rounded bg-accent text-foreground transition-colors">
                                            <span>👍</span>
                                            <span>3</span>
                                        </button>
                                        <button className="flex items-center gap-1 px-1.5 py-0.5 text-xs rounded bg-secondary text-muted-foreground hover:text-foreground transition-colors">
                                            <span>🚀</span>
                                            <span>1</span>
                                        </button>
                                    </div>
                                </div>

                                <div className="mb-8">
                                    <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-5">Activity</h3>
                                    <div className="space-y-5">
                                        {activity.map((item) => {
                                            if (item.type === "event") {
                                                return <ActivityEvent key={item.id} item={item} />;
                                            }
                                            if (item.type === "diff_comment") {
                                                return <DiffCommentBlock key={item.id} item={item} />;
                                            }
                                            return <CommentBlock key={item.id} item={item} />;
                                        })}
                                    </div>
                                </div>

                                <div className="border border-border rounded-md overflow-hidden mb-8">
                                    <div className="flex items-center gap-3 px-4 py-3 border-b border-border bg-card/60">
                                        <CIIcon className={`h-5 w-5 ${ciStatusConfig[mergeRequest.ciStatus].color}`} />
                                        <span className="font-medium text-sm">All checks have passed</span>
                                    </div>
                                    <div className="px-4 py-3 space-y-2">
                                        {mergeRequest.ciJobs.map((job) => {
                                            const jobConfig = ciStatusConfig[job.status];
                                            const JobIcon = jobConfig.icon;
                                            return (
                                                <div key={job.name} className="flex items-center gap-3 text-sm">
                                                    <JobIcon className={`h-4 w-4 ${jobConfig.color}`} />
                                                    <span className="font-mono text-xs">{job.name}</span>
                                                    <span className="text-muted-foreground text-xs">{jobConfig.label}</span>
                                                </div>
                                            );
                                        })}
                                    </div>
                                    <div className="px-4 py-3 bg-card/60 border-t border-border flex items-center justify-between">
                                        <div className="flex items-center gap-2 text-sm">
                                            <Check className="h-4 w-4 text-green-500" />
                                            <span className="text-green-500 font-medium">Ready to merge</span>
                                            <span className="text-muted-foreground">· No conflicts</span>
                                        </div>
                                        <DropdownMenu>
                                            <DropdownMenuTrigger asChild>
                                                <Button size="sm" className="gap-2">
                                                    Merge
                                                    <ChevronDown className="h-3.5 w-3.5" />
                                                </Button>
                                            </DropdownMenuTrigger>
                                            <DropdownMenuContent align="end">
                                                <DropdownMenuItem>Create a merge commit</DropdownMenuItem>
                                                <DropdownMenuItem>Squash and merge</DropdownMenuItem>
                                                <DropdownMenuItem>Rebase and merge</DropdownMenuItem>
                                            </DropdownMenuContent>
                                        </DropdownMenu>
                                    </div>
                                </div>

                                <div className="border-t border-border pt-6">
                                    <CommentComposer />
                                </div>
                            </>
                        )}

                        {activeTab === "commits" && (
                            <div className="space-y-1.5">
                                {commitHistory.map((commit) => {
                                    const ciConf = ciStatusConfig[commit.ci];
                                    const CICommitIcon = ciConf.icon;
                                    const isCopied = copiedHash === commit.hash;
                                    return (
                                        <div
                                            key={commit.hash}
                                            className="group flex items-center gap-3 px-4 py-3 border border-border rounded-md hover:bg-accent/40 transition-colors"
                                        >
                                            <CICommitIcon className={`h-4 w-4 shrink-0 ${ciConf.color}`} title={ciConf.label} />
                                            <GitCommit className="h-4 w-4 text-muted-foreground shrink-0" />
                                            <div className="flex-1 min-w-0">
                                                <p className="truncate text-sm">{commit.message}</p>
                                                <p className="text-xs text-muted-foreground">
                                                    {commit.author} · {commit.date}
                                                </p>
                                            </div>
                                            <div className="flex items-center gap-1.5 shrink-0">
                                                <code className="px-2 py-0.5 bg-secondary rounded text-xs font-mono">
                                                    {commit.shortHash}
                                                </code>
                                                <button
                                                    onClick={() => copyHash(commit.hash)}
                                                    className="opacity-0 group-hover:opacity-100 p-1 hover:bg-accent rounded transition-all"
                                                    title="Copy full hash"
                                                >
                                                    {isCopied ? (
                                                        <Check className="h-3.5 w-3.5 text-green-500" />
                                                    ) : (
                                                        <Copy className="h-3.5 w-3.5 text-muted-foreground" />
                                                    )}
                                                </button>
                                            </div>
                                        </div>
                                    );
                                })}
                            </div>
                        )}

                        {activeTab === "changes" && (
                            <div className="space-y-4">
                                <div className="flex items-center gap-4 text-sm text-muted-foreground pb-4 border-b border-border">
                                    <span>{mergeRequest.filesChanged} files changed</span>
                                    <span className="text-green-500 font-medium">+{mergeRequest.additions}</span>
                                    <span className="text-red-500 font-medium">-{mergeRequest.deletions}</span>
                                </div>

                                <FileDiff
                                    filename="src/auth/ssh_key.rs"
                                    additions={58}
                                    deletions={4}
                                    hunks={[
                                        {
                                            header: "@@ -38,10 +38,18 @@ impl SSHKey {",
                                            lines: [
                                                { type: "context", lineOld: 38, lineNew: 38, content: "pub struct SSHKey {" },
                                                { type: "context", lineOld: 39, lineNew: 39, content: "    pub id: i64," },
                                                { type: "context", lineOld: 40, lineNew: 40, content: "    pub user_id: i64," },
                                                { type: "remove", lineOld: 41, lineNew: null, content: "    pub key: String," },
                                                { type: "add", lineOld: null, lineNew: 41, content: "    pub public_key: String," },
                                                { type: "add", lineOld: null, lineNew: 42, content: "    pub fingerprint: String," },
                                                { type: "add", lineOld: null, lineNew: 43, content: "    pub key_type: KeyType," },
                                                {
                                                    type: "context",
                                                    lineOld: 42,
                                                    lineNew: 44,
                                                    content: "    pub created_at: DateTime<Utc>,",
                                                },
                                                { type: "context", lineOld: 43, lineNew: 45, content: "}" },
                                            ],
                                        },
                                        {
                                            header: "@@ -61,6 +69,12 @@ pub fn parse_key(raw: &str) -> SSHKey {",
                                            lines: [
                                                {
                                                    type: "context",
                                                    lineOld: 61,
                                                    lineNew: 69,
                                                    content: "pub fn parse_key(raw: &str) -> Result<SSHKey, ParseError> {",
                                                },
                                                {
                                                    type: "remove",
                                                    lineOld: 62,
                                                    lineNew: null,
                                                    content: '    if raw.is_empty() { panic!("empty key"); }',
                                                },
                                                {
                                                    type: "add",
                                                    lineOld: null,
                                                    lineNew: 70,
                                                    content: "    if raw.is_empty() { return Err(ParseError::EmptyKey); }",
                                                },
                                                {
                                                    type: "context",
                                                    lineOld: 63,
                                                    lineNew: 71,
                                                    content: "    let parts: Vec<&str> = raw.split(' ').collect();",
                                                },
                                                {
                                                    type: "add",
                                                    lineOld: null,
                                                    lineNew: 72,
                                                    content: "    if parts.len() < 2 { return Err(ParseError::InvalidFormat); }",
                                                },
                                                {
                                                    type: "context",
                                                    lineOld: 64,
                                                    lineNew: 73,
                                                    content: "    let key_type = parts[0].parse::<KeyType>()?;",
                                                },
                                                {
                                                    type: "context",
                                                    lineOld: 65,
                                                    lineNew: 74,
                                                    content: "    let data = base64::decode(parts[1])?;",
                                                },
                                            ],
                                        },
                                    ]}
                                />

                                <FileDiff
                                    filename="src/auth/mod.rs"
                                    additions={12}
                                    deletions={2}
                                    hunks={[
                                        {
                                            header: "@@ -1,8 +1,18 @@",
                                            lines: [
                                                { type: "context", lineOld: 1, lineNew: 1, content: "mod error;" },
                                                { type: "add", lineOld: null, lineNew: 2, content: "mod ssh_key;" },
                                                { type: "add", lineOld: null, lineNew: 3, content: "mod fingerprint;" },
                                                { type: "context", lineOld: 2, lineNew: 4, content: "" },
                                                { type: "context", lineOld: 3, lineNew: 5, content: "pub use error::AuthError;" },
                                                { type: "remove", lineOld: 4, lineNew: null, content: "pub use key::SSHKey;" },
                                                {
                                                    type: "add",
                                                    lineOld: null,
                                                    lineNew: 6,
                                                    content: "pub use ssh_key::{SSHKey, KeyType, ParseError};",
                                                },
                                                {
                                                    type: "add",
                                                    lineOld: null,
                                                    lineNew: 7,
                                                    content: "pub use fingerprint::generate_fingerprint;",
                                                },
                                            ],
                                        },
                                    ]}
                                />
                            </div>
                        )}
                    </div>
                </main>

                <aside className="w-72 border-l border-border shrink-0 overflow-y-auto p-5 space-y-6">
                    <div>
                        <div className="grid grid-cols-3 gap-2">
                            <div className="text-center p-2.5 border border-border rounded-md">
                                <p className="text-base font-semibold">{mergeRequest.commits}</p>
                                <p className="text-xs text-muted-foreground">Commits</p>
                            </div>
                            <div className="text-center p-2.5 border border-border rounded-md">
                                <p className="text-base font-semibold text-green-500 flex items-center justify-center gap-0.5">
                                    <Plus className="h-3 w-3" />
                                    {mergeRequest.additions}
                                </p>
                                <p className="text-xs text-muted-foreground">Added</p>
                            </div>
                            <div className="text-center p-2.5 border border-border rounded-md">
                                <p className="text-base font-semibold text-red-500 flex items-center justify-center gap-0.5">
                                    <Minus className="h-3 w-3" />
                                    {mergeRequest.deletions}
                                </p>
                                <p className="text-xs text-muted-foreground">Removed</p>
                            </div>
                        </div>
                    </div>

                    <div>
                        <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">Status</h3>
                        <DropdownMenu>
                            <DropdownMenuTrigger asChild>
                                <button className="w-full flex items-center justify-between px-3 py-2 border border-border rounded-md hover:bg-accent/50 transition-colors text-sm">
                                    <div className="flex items-center gap-2">
                                        <StatusIcon className={`h-4 w-4 ${statusInfo.color}`} />
                                        {statusInfo.label}
                                    </div>
                                    <ChevronDown className="h-4 w-4 text-muted-foreground" />
                                </button>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent align="start" className="w-56">
                                <DropdownMenuItem>
                                    <GitPullRequest className="h-4 w-4 mr-2 text-blue-500" />
                                    Mark as ready
                                </DropdownMenuItem>
                                <DropdownMenuItem>
                                    <FileCode className="h-4 w-4 mr-2 text-muted-foreground" />
                                    Convert to draft
                                </DropdownMenuItem>
                                <DropdownMenuSeparator />
                                <DropdownMenuItem className="text-red-500">
                                    <XCircle className="h-4 w-4 mr-2" />
                                    Close merge request
                                </DropdownMenuItem>
                                <DropdownMenuSeparator />
                                <div className="px-2 py-1.5 text-xs text-muted-foreground leading-relaxed">
                                    To merge this MR, use the merge button at the bottom of the activity feed.
                                </div>
                            </DropdownMenuContent>
                        </DropdownMenu>
                    </div>

                    <div>
                        <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">Reviewers</h3>
                        <div className="space-y-1.5">
                            {mergeRequest.reviewers.map((reviewer) => {
                                const reviewConfig = reviewStatusConfig[reviewer.status];
                                const ReviewIcon = reviewConfig.icon;
                                return (
                                    <div
                                        key={reviewer.name}
                                        className="flex items-center justify-between px-3 py-2 border border-border rounded-md text-sm"
                                    >
                                        <div className="flex items-center gap-2">
                                            <div className="flex h-5 w-5 items-center justify-center rounded-full bg-secondary text-[10px] font-medium">
                                                {reviewer.name[0].toUpperCase()}
                                            </div>
                                            {reviewer.name}
                                        </div>
                                        <ReviewIcon className={`h-4 w-4 ${reviewConfig.color}`} />
                                    </div>
                                );
                            })}
                            <button className="w-full flex items-center justify-center gap-2 px-3 py-2 border border-dashed border-border rounded-md text-sm text-muted-foreground hover:text-foreground hover:border-solid transition-colors">
                                <User className="h-3.5 w-3.5" />
                                Add reviewer
                            </button>
                        </div>
                    </div>

                    <div>
                        <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">Labels</h3>
                        <div className="flex flex-wrap gap-2">
                            {mergeRequest.labels.map((label) => (
                                <LabelBadge key={label.name} label={label} removable />
                            ))}
                            <button className="flex items-center gap-1 px-2 py-0.5 text-xs border border-dashed border-border rounded text-muted-foreground hover:text-foreground hover:border-solid transition-colors">
                                <Tag className="h-3 w-3" />
                                Add
                            </button>
                        </div>
                    </div>

                    {mergeRequest.linkedIssue && (
                        <div>
                            <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">Linked Issue</h3>
                            <Link
                                href={`/${repoData.org}/${repoData.name}/issues/${mergeRequest.linkedIssue.id}`}
                                className="flex items-center gap-2 px-3 py-2 border border-border rounded-md hover:bg-accent/50 transition-colors text-sm"
                            >
                                <AlertCircle className="h-3.5 w-3.5 text-amber-500 shrink-0" />
                                <span className="truncate">
                                    #{mergeRequest.linkedIssue.id} {mergeRequest.linkedIssue.title}
                                </span>
                            </Link>
                        </div>
                    )}

                    <div>
                        <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">Timestamps</h3>
                        <div className="space-y-1.5 text-xs text-muted-foreground">
                            <div className="flex items-center gap-2">
                                <Calendar className="h-3.5 w-3.5" />
                                <span>Created {mergeRequest.createdAt}</span>
                            </div>
                            <div className="flex items-center gap-2">
                                <Clock className="h-3.5 w-3.5" />
                                <span>Updated {mergeRequest.updatedAt}</span>
                            </div>
                        </div>
                    </div>
                </aside>
            </div>
        </div>
    );
}

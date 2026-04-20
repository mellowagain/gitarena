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
    Circle,
    CheckCircle2,
    CircleDot,
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
    XCircle,
    X,
    Eye,
    MessageSquare,
} from "lucide-react";

const currentUser = { name: "Mari" };
const repoData = { org: "mellowagain", name: "test" };

type Status = "todo" | "in_progress" | "done" | "cancelled";
type Priority = "urgent" | "high" | "medium" | "low" | "none";

const issue = {
    id: 42,
    title: "Add support for SSH key authentication",
    description: `## Summary
We need to implement SSH key authentication to allow users to push/pull without entering their password each time.

## Requirements
- Support for RSA, ECDSA, and Ed25519 keys
- Key management UI in user settings
- Deploy keys for repositories
- SSH key fingerprint display

## Technical Details
The implementation should follow the OpenSSH format for key storage and validation.

\`\`\`rust
pub struct SSHKey {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub fingerprint: String,
    pub public_key: String,
    pub created_at: DateTime<Utc>,
}
\`\`\`

## References
- [OpenSSH Key Format](https://example.com)
- Related issue: #35`,
    status: "in_progress" as Status,
    priority: "high" as Priority,
    author: "mellowagain",
    createdAt: "2 days ago",
    updatedAt: "5 hours ago",
    labels: [
        { name: "enhancement", color: "#a2eeef" },
        { name: "component::auth", color: "#d73a4a" },
    ],
    assignees: ["Mari", "mellowagain"],
    milestone: "v1.0.0",
    linkedMR: { id: 15, title: "feat: Add SSH key authentication support" },
    reactions: [
        { emoji: "👍", count: 5, reacted: true },
        { emoji: "🎉", count: 2, reacted: false },
    ],
};

type ActivityItem =
    | {
          type: "comment";
          id: number;
          author: string;
          isBot?: boolean;
          content: string;
          createdAt: string;
          reactions: { emoji: string; count: number; reacted: boolean }[];
      }
    | { type: "event"; id: number; author: string; isBot?: boolean; action: string; detail?: string; createdAt: string };

const activity: ActivityItem[] = [
    {
        type: "comment",
        id: 1,
        author: "contributor1",
        content:
            "This would be a great addition! I've been using HTTPS but SSH keys would be much more convenient.\n\nAre we planning to support **hardware security keys** (like YubiKey) in a future iteration as well?",
        createdAt: "2 days ago",
        reactions: [{ emoji: "👍", count: 3, reacted: false }],
    },
    {
        type: "event",
        id: 2,
        author: "mellowagain",
        action: "added label",
        detail: "security",
        createdAt: "1 day ago",
    },
    {
        type: "comment",
        id: 3,
        author: "mellowagain",
        content: `I've started working on this. The basic key parsing is done, now working on the authentication flow.

Here's the current progress:
- [x] SSH key parsing
- [x] Fingerprint generation
- [ ] Auth flow integration
- [ ] User settings UI`,
        createdAt: "1 day ago",
        reactions: [{ emoji: "🚀", count: 2, reacted: true }],
    },
    {
        type: "event",
        id: 4,
        author: "mellowagain",
        action: "changed status to",
        detail: "In Progress",
        createdAt: "1 day ago",
    },
    {
        type: "event",
        id: 5,
        author: "gitarena-ci",
        isBot: true,
        action: "set status to",
        detail: "In Progress",
        createdAt: "1 day ago",
    },
    {
        type: "comment",
        id: 6,
        author: "gitarena-ci",
        isBot: true,
        content:
            "CI pipeline started for branch `feature/ssh-auth`.\n\n**Jobs running:**\n- build\n- test\n- lint\n\nResults will be posted here when complete.",
        createdAt: "1 day ago",
        reactions: [],
    },
    {
        type: "comment",
        id: 7,
        author: "Mari",
        content:
            "Looking good! Let me know when you need a review on the UI components.\n\nAlso make sure to check the `auth_service.rs` file — there might be some relevant patterns already in there.",
        createdAt: "5 hours ago",
        reactions: [],
    },
    {
        type: "event",
        id: 8,
        author: "contributor1",
        action: "deleted a comment from",
        detail: "contributor4",
        createdAt: "3 hours ago",
    },
];

const statusConfig: Record<Status, { icon: typeof Circle; label: string; color: string; bg: string }> = {
    todo: { icon: Circle, label: "Todo", color: "text-muted-foreground", bg: "bg-muted" },
    in_progress: { icon: CircleDot, label: "In Progress", color: "text-amber-500", bg: "bg-amber-500/10" },
    done: { icon: CheckCircle2, label: "Done", color: "text-green-500", bg: "bg-green-500/10" },
    cancelled: { icon: XCircle, label: "Cancelled", color: "text-muted-foreground/50", bg: "bg-muted" },
};

const priorityConfig: Record<Priority, { bars: number; color: string; label: string }> = {
    urgent: { bars: 4, color: "bg-red-500", label: "Urgent" },
    high: { bars: 3, color: "bg-orange-500", label: "High" },
    medium: { bars: 2, color: "bg-yellow-500", label: "Medium" },
    low: { bars: 1, color: "bg-blue-500", label: "Low" },
    none: { bars: 0, color: "bg-muted", label: "No priority" },
};

function highlightCode(code: string, lang: string): React.ReactNode {
    const isRust = lang === "rust" || lang === "rs";
    if (!isRust) {
        return code;
    }

    const keywords =
        /\b(pub|fn|struct|impl|let|mut|use|mod|match|if|else|return|for|in|while|loop|enum|type|trait|where|self|Self|crate|super|async|await|move|ref|const|static|extern|unsafe|true|false|Some|None|Ok|Err)\b/g;
    const types =
        /\b(i8|i16|i32|i64|i128|u8|u16|u32|u64|u128|f32|f64|usize|isize|bool|str|String|Vec|Option|Result|Box|Arc|Rc|DateTime|Utc)\b/g;
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

function LabelBadge({ label, removable }: { label: { name: string; color: string }; removable?: boolean }) {
    const scopedIndex = label.name.indexOf("::");
    const isScoped = scopedIndex !== -1;
    const scopeKey = isScoped ? label.name.slice(0, scopedIndex) : null;
    const scopeValue = isScoped ? label.name.slice(scopedIndex + 2) : null;

    if (isScoped) {
        return (
            <span className="group/label inline-flex items-center text-xs rounded overflow-hidden">
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
            const text = headingMatch[2];
            const cls =
                level === 1
                    ? "text-lg font-semibold mt-4 mb-2"
                    : level === 2
                      ? "text-base font-semibold mt-4 mb-1.5"
                      : "text-sm font-semibold mt-3 mb-1";
            nodes.push(
                <div key={k()} className={cls}>
                    {text}
                </div>
            );
            i++;
            continue;
        }

        if (line.match(/^- \[[ x]\]/)) {
            const items: React.ReactNode[] = [];
            while (i < lines.length && lines[i].match(/^- \[[ x]\]/)) {
                const checked = lines[i][3] === "x";
                const itemText = lines[i].slice(6);
                items.push(
                    <li key={k()} className="flex items-start gap-2">
                        <span className={`mt-0.5 text-xs ${checked ? "text-green-500" : "text-muted-foreground"}`}>
                            {checked ? "☑" : "☐"}
                        </span>
                        <span className={checked ? "line-through text-muted-foreground/60" : ""}>{itemText}</span>
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

function PriorityIndicator({ priority }: { priority: Priority }) {
    const config = priorityConfig[priority];
    return (
        <div className="flex items-end gap-0.5 h-4 w-4" title={config.label}>
            {[1, 2, 3, 4].map((bar) => (
                <div
                    key={bar}
                    className={`w-0.5 rounded-full ${bar <= config.bars ? config.color : "bg-muted-foreground/20"}`}
                    style={{ height: `${bar * 25}%` }}
                />
            ))}
        </div>
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

function CommentBlock({ item }: { item: Extract<ActivityItem, { type: "comment" }> }) {
    return (
        <div
            className={`group pl-4 border-l-2 transition-colors ${item.isBot ? "border-secondary hover:border-muted-foreground/30" : "border-border hover:border-muted-foreground/40"}`}
        >
            <div className="flex items-center gap-2 mb-2">
                <AuthorAvatar author={item.author} isBot={item.isBot} />
                <span className="text-sm font-medium">{item.author}</span>
                {item.isBot && (
                    <span className="text-[10px] px-1 py-0.5 rounded bg-secondary border border-border text-muted-foreground font-medium leading-none">
                        bot
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
                            className={`flex items-center gap-1 px-1.5 py-0.5 text-xs rounded transition-colors ${
                                reaction.reacted ? "bg-accent text-foreground" : "bg-secondary text-muted-foreground hover:text-foreground"
                            }`}
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

function CommentComposer({ label = "Comment", extra }: { label?: string; extra?: React.ReactNode }) {
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
                        {extra}
                        <Button size="sm" disabled={!text.trim()}>
                            {label}
                        </Button>
                    </div>
                </div>
            </div>
        </div>
    );
}

export default function IssuePage() {
    const statusInfo = statusConfig[issue.status];
    const StatusIcon = statusInfo.icon;

    return (
        <div className="min-h-screen bg-background flex flex-col">
            <TopBar
                breadcrumb={[
                    { label: repoData.org, href: `/${repoData.org}` },
                    { label: repoData.name, href: `/${repoData.org}/${repoData.name}` },
                    { label: "Issues", href: `/${repoData.org}/${repoData.name}/issues` },
                    { label: `#${issue.id}` },
                ]}
                navLinks={[
                    { label: "Code", href: `/${repoData.org}/${repoData.name}`, icon: <Code className="h-[18px] w-[18px]" /> },
                    {
                        label: "Issues",
                        href: `/${repoData.org}/${repoData.name}/issues`,
                        icon: <AlertCircle className="h-[18px] w-[18px]" />,
                        active: true,
                    },
                    {
                        label: "Merge Requests",
                        href: `/${repoData.org}/${repoData.name}/merge-requests`,
                        icon: <GitMerge className="h-[18px] w-[18px]" />,
                    },
                ]}
                hasNotifications
            />

            <div className="flex-1 flex overflow-hidden">
                <main className="flex-1 overflow-y-auto">
                    <div className="max-w-4xl mx-auto px-6 py-8">
                        <Link
                            href={`/${repoData.org}/${repoData.name}/issues`}
                            className="inline-flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors mb-6"
                        >
                            <ArrowLeft className="h-4 w-4" />
                            Back to issues
                        </Link>

                        <div className="mb-8">
                            <div className="flex items-start gap-4 mb-3">
                                <h1 className="text-2xl font-semibold flex-1 leading-snug">
                                    <span className="text-muted-foreground font-normal">#{issue.id}</span> {issue.title}
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
                                            <Trash2 className="h-4 w-4 mr-2" />
                                            Delete
                                        </DropdownMenuItem>
                                    </DropdownMenuContent>
                                </DropdownMenu>
                            </div>
                            <div className="flex items-center gap-2.5 text-sm text-muted-foreground flex-wrap">
                                <StatusIcon className={`h-4 w-4 ${statusInfo.color}`} />
                                <span className={`font-medium ${statusInfo.color}`}>{statusInfo.label}</span>
                                <span className="text-muted-foreground/40">·</span>
                                <span>
                                    Opened {issue.createdAt} by <span className="text-foreground font-medium">{issue.author}</span>
                                </span>
                                <span className="text-muted-foreground/40">·</span>
                                <span className="flex items-center gap-1">
                                    <MessageSquare className="h-3.5 w-3.5" />
                                    {activity.filter((a) => a.type === "comment").length} comments
                                </span>
                            </div>
                        </div>

                        <div className="group/desc mb-8 pl-5 border-l-4 border-muted-foreground/20 hover:border-muted-foreground/40 transition-colors">
                            <div className="flex items-center gap-2 mb-3">
                                <div className="flex h-6 w-6 items-center justify-center rounded-full bg-secondary text-xs font-medium shrink-0">
                                    {issue.author[0].toUpperCase()}
                                </div>
                                <span className="text-sm font-medium">{issue.author}</span>
                                <span className="text-xs text-muted-foreground">opened {issue.createdAt}</span>
                                <div className="ml-auto flex items-center gap-1 opacity-0 group-hover/desc:opacity-100 transition-opacity">
                                    <button className="p-1 hover:bg-accent rounded transition-colors">
                                        <Smile className="h-3.5 w-3.5 text-muted-foreground" />
                                    </button>
                                    <button className="p-1 hover:bg-accent rounded transition-colors">
                                        <Edit className="h-3.5 w-3.5 text-muted-foreground" />
                                    </button>
                                </div>
                            </div>
                            <div className="text-sm text-foreground/80">{renderMarkdown(issue.description)}</div>
                            {issue.reactions.length > 0 && (
                                <div className="flex items-center gap-1.5 mt-3">
                                    {issue.reactions.map((reaction, i) => (
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

                        <div className="mb-8">
                            <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-5">Activity</h3>
                            <div className="space-y-5">
                                {activity.map((item) =>
                                    item.type === "comment" ? (
                                        <CommentBlock key={item.id} item={item} />
                                    ) : (
                                        <ActivityEvent key={item.id} item={item} />
                                    )
                                )}
                            </div>
                        </div>

                        <div className="border-t border-border pt-6">
                            <CommentComposer />
                        </div>
                    </div>
                </main>

                <aside className="w-72 border-l border-border shrink-0 overflow-y-auto p-5 space-y-6">
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
                            <DropdownMenuContent align="start" className="w-48">
                                <DropdownMenuItem>
                                    <Circle className="h-4 w-4 mr-2" />
                                    Todo
                                </DropdownMenuItem>
                                <DropdownMenuItem>
                                    <CircleDot className="h-4 w-4 mr-2 text-amber-500" />
                                    In Progress
                                </DropdownMenuItem>
                                <DropdownMenuItem>
                                    <CheckCircle2 className="h-4 w-4 mr-2 text-green-500" />
                                    Done
                                </DropdownMenuItem>
                                <DropdownMenuItem>
                                    <XCircle className="h-4 w-4 mr-2 text-muted-foreground" />
                                    Cancelled
                                </DropdownMenuItem>
                            </DropdownMenuContent>
                        </DropdownMenu>
                    </div>

                    <div>
                        <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">Priority</h3>
                        <DropdownMenu>
                            <DropdownMenuTrigger asChild>
                                <button className="w-full flex items-center justify-between px-3 py-2 border border-border rounded-md hover:bg-accent/50 transition-colors text-sm">
                                    <div className="flex items-center gap-2">
                                        <PriorityIndicator priority={issue.priority} />
                                        {priorityConfig[issue.priority].label}
                                    </div>
                                    <ChevronDown className="h-4 w-4 text-muted-foreground" />
                                </button>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent align="start" className="w-48">
                                <DropdownMenuItem>Urgent</DropdownMenuItem>
                                <DropdownMenuItem>High</DropdownMenuItem>
                                <DropdownMenuItem>Medium</DropdownMenuItem>
                                <DropdownMenuItem>Low</DropdownMenuItem>
                                <DropdownMenuItem>No priority</DropdownMenuItem>
                            </DropdownMenuContent>
                        </DropdownMenu>
                    </div>

                    <div>
                        <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">Assignees</h3>
                        <div className="space-y-1.5">
                            {issue.assignees.map((assignee) => (
                                <div
                                    key={assignee}
                                    className="group flex items-center gap-2 px-3 py-2 rounded-md border border-border text-sm hover:bg-accent/30 transition-colors"
                                >
                                    <div className="flex h-5 w-5 items-center justify-center rounded-full bg-secondary text-[10px] font-medium shrink-0">
                                        {assignee[0].toUpperCase()}
                                    </div>
                                    <span className="flex-1">{assignee}</span>
                                    <button
                                        className="opacity-0 group-hover:opacity-100 p-0.5 hover:bg-accent rounded transition-all"
                                        title="Remove assignee"
                                    >
                                        <X className="h-3 w-3 text-muted-foreground" />
                                    </button>
                                </div>
                            ))}
                            <button className="w-full flex items-center justify-center gap-2 px-3 py-2 border border-dashed border-border rounded-md text-sm text-muted-foreground hover:text-foreground hover:border-solid transition-colors">
                                <User className="h-3.5 w-3.5" />
                                Add assignee
                            </button>
                        </div>
                    </div>

                    <div>
                        <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">Labels</h3>
                        <div className="flex flex-wrap gap-2">
                            {issue.labels.map((label) => (
                                <LabelBadge key={label.name} label={label} removable />
                            ))}
                            <button className="flex items-center gap-1 px-2 py-0.5 text-xs border border-dashed border-border rounded text-muted-foreground hover:text-foreground hover:border-solid transition-colors">
                                <Tag className="h-3 w-3" />
                                Add
                            </button>
                        </div>
                    </div>

                    <div>
                        <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">Milestone</h3>
                        <div className="flex items-center gap-2 px-3 py-2 border border-border rounded-md text-sm">
                            <Clock className="h-3.5 w-3.5 text-muted-foreground" />
                            {issue.milestone}
                        </div>
                    </div>

                    {issue.linkedMR && (
                        <div>
                            <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">
                                Linked Merge Request
                            </h3>
                            <Link
                                href={`/${repoData.org}/${repoData.name}/merge-requests/${issue.linkedMR.id}`}
                                className="flex items-center gap-2 px-3 py-2 border border-border rounded-md hover:bg-accent/50 transition-colors text-sm"
                            >
                                <GitBranch className="h-3.5 w-3.5 text-green-500 shrink-0" />
                                <span className="truncate">
                                    !{issue.linkedMR.id} {issue.linkedMR.title}
                                </span>
                            </Link>
                        </div>
                    )}

                    <div>
                        <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">Timestamps</h3>
                        <div className="space-y-1.5 text-xs text-muted-foreground">
                            <div className="flex items-center gap-2">
                                <Calendar className="h-3.5 w-3.5" />
                                <span>Created {issue.createdAt}</span>
                            </div>
                            <div className="flex items-center gap-2">
                                <Clock className="h-3.5 w-3.5" />
                                <span>Updated {issue.updatedAt}</span>
                            </div>
                        </div>
                    </div>
                </aside>
            </div>
        </div>
    );
}

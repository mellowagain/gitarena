"use client";

import Link from "next/link";
import {
    GitBranch,
    GitMerge,
    AlertCircle,
    MessageSquare,
    CheckCircle2,
    CircleDot,
    Circle,
    Package,
    Shield,
    Zap,
    Globe,
    Lock,
    Users,
    ChevronRight,
    FileCode,
    ArrowRight,
    Check,
    X,
    Terminal,
    Server,
    Key,
} from "lucide-react";

const demoIssues = [
    {
        id: 42,
        status: "in_progress",
        title: "Add support for SSH key authentication",
        labels: [
            { name: "enhancement", color: "#a2eeef" },
            { name: "component::auth", color: "#d73a4a" },
        ],
        comments: 8,
        ago: "2h ago",
    },
    {
        id: 41,
        status: "todo",
        title: "Repository mirroring from GitHub/GitLab",
        labels: [{ name: "feature", color: "#0075ca" }],
        comments: 3,
        ago: "1d ago",
    },
    {
        id: 38,
        status: "done",
        title: "Fix timezone offset in commit timestamps",
        labels: [{ name: "bug", color: "#d73a4a" }],
        comments: 2,
        ago: "3d ago",
    },
    {
        id: 37,
        status: "todo",
        title: "Add webhook support for push events",
        labels: [
            { name: "enhancement", color: "#a2eeef" },
            { name: "component::webhooks", color: "#7057ff" },
        ],
        comments: 5,
        ago: "4d ago",
    },
];

const demoMRs = [
    {
        id: 15,
        status: "open",
        ci: "passed",
        title: "feat: implement SSH key parsing and validation",
        source: "feature/ssh-keys",
        additions: 312,
        deletions: 18,
    },
    {
        id: 14,
        status: "merged",
        ci: "passed",
        title: "fix: handle UTF-8 filenames in tree view",
        source: "fix/utf8-tree",
        additions: 47,
        deletions: 22,
    },
    {
        id: 13,
        status: "open",
        ci: "running",
        title: "refactor: split auth module into sub-crates",
        source: "refactor/auth",
        additions: 189,
        deletions: 203,
    },
    { id: 12, status: "closed", ci: "failed", title: "chore: upgrade axum to 0.7", source: "chore/axum-07", additions: 84, deletions: 91 },
];

const demoCommits = [
    { hash: "9bf39d9", message: "feat(auth): add SSH key fingerprint verification", author: "Mari", ago: "2h ago", ci: "passed" },
    { hash: "3ca21f1", message: "fix(tree): handle symlinks in directory listing", author: "Alex", ago: "5h ago", ci: "passed" },
    { hash: "b8e04d2", message: "refactor(db): migrate pool to sqlx 0.7", author: "Jordan", ago: "1d ago", ci: "failed" },
    { hash: "f1a9c33", message: "docs: update API reference for v0.2", author: "Mari", ago: "2d ago", ci: "passed" },
];

const stats = [
    { label: "Written in", value: "Rust" },
    { label: "License", value: "AGPL-3.0" },
    { label: "Open issues", value: "42" },
    { label: "Contributors", value: "9" },
    { label: "Releases", value: "3" },
    { label: "Stars", value: "1.2k" },
];

const features = [
    { icon: GitBranch, label: "Git hosting", desc: "Full Git protocol over HTTPS and SSH with LFS support." },
    { icon: AlertCircle, label: "Issue tracking", desc: "Labels, milestones, assignments, scoped labels." },
    { icon: GitMerge, label: "Merge requests", desc: "Inline diffs, CI integration, review workflows." },
    { icon: Shield, label: "Access control", desc: "Org-level roles, repo visibility, deploy keys." },
    { icon: Zap, label: "CI integration", desc: "Webhook-driven pipelines — bring your own runner." },
    { icon: Package, label: "Package registry", desc: "Cargo, npm, Docker — one registry, one auth layer." },
    { icon: Globe, label: "Federation", desc: "Follow repos and users across GitArena instances." },
    { icon: FileCode, label: "Code review", desc: "Line-level comments, suggestions, approval gates." },
];

const comparison = [
    { feature: "Self-hostable", gitarena: true, github: false, gitlab: true },
    { feature: "Written in Rust", gitarena: true, github: false, gitlab: false },
    { feature: "Single binary deploy", gitarena: true, github: false, gitlab: false },
    { feature: "Scoped labels", gitarena: true, github: false, gitlab: true },
    { feature: "Built-in package registry", gitarena: true, github: true, gitlab: true },
    { feature: "Federation", gitarena: true, github: false, gitlab: false },
    { feature: "Open source (AGPL-3.0)", gitarena: true, github: false, gitlab: false },
    { feature: "No telemetry", gitarena: true, github: false, gitlab: false },
];

const testimonials = [
    { quote: "Finally a Git platform that doesn't require a Kubernetes cluster to run.", author: "Lars K.", role: "Platform engineer" },
    { quote: "Migrated off GitLab in a weekend. Single binary, SQLite, done.", author: "Yuki T.", role: "Solo founder" },
    { quote: "The Rust rewrite means it runs great on my old server. Zero bloat.", author: "Morgan S.", role: "Open source maintainer" },
];

function IssueStatusIcon({ status }: { status: string }) {
    if (status === "in_progress") {
        return <CircleDot className="h-4 w-4 text-yellow-500 shrink-0" />;
    }
    if (status === "done") {
        return <CheckCircle2 className="h-4 w-4 text-green-500 shrink-0" />;
    }
    return <Circle className="h-4 w-4 text-muted-foreground shrink-0" />;
}

function MRStatusIcon({ status }: { status: string }) {
    if (status === "merged") {
        return <GitMerge className="h-4 w-4 text-purple-500 shrink-0" />;
    }
    if (status === "closed") {
        return <GitMerge className="h-4 w-4 text-red-500 shrink-0" />;
    }
    return <GitMerge className="h-4 w-4 text-green-500 shrink-0" />;
}

function CIDot({ ci }: { ci: string }) {
    if (ci === "passed") {
        return <span className="w-2 h-2 rounded-full bg-green-500 shrink-0" />;
    }
    if (ci === "failed") {
        return <span className="w-2 h-2 rounded-full bg-red-500 shrink-0" />;
    }
    if (ci === "running") {
        return <span className="w-2 h-2 rounded-full bg-yellow-500 shrink-0 animate-pulse" />;
    }
    return <span className="w-2 h-2 rounded-full bg-muted-foreground shrink-0" />;
}

function LabelBadge({ label }: { label: { name: string; color: string } }) {
    const idx = label.name.indexOf("::");
    if (idx !== -1) {
        const key = label.name.slice(0, idx);
        const val = label.name.slice(idx + 2);
        return (
            <span className="inline-flex items-center text-xs rounded overflow-hidden shrink-0">
                <span className="px-1.5 py-0.5 font-medium" style={{ backgroundColor: `${label.color}35`, color: label.color }}>
                    {key}
                </span>
                <span className="px-1.5 py-0.5" style={{ backgroundColor: `${label.color}20`, color: label.color }}>
                    {val}
                </span>
            </span>
        );
    }
    return (
        <span className="shrink-0 px-1.5 py-0.5 text-xs rounded" style={{ backgroundColor: `${label.color}20`, color: label.color }}>
            {label.name}
        </span>
    );
}

function Avatar({ name }: { name: string }) {
    return (
        <span className="w-5 h-5 rounded-full bg-accent border border-border flex items-center justify-center text-[10px] font-medium text-muted-foreground shrink-0">
            {name[0].toUpperCase()}
        </span>
    );
}

function Tick({ ok }: { ok: boolean }) {
    return ok ? <Check className="h-3.5 w-3.5 text-green-500 mx-auto" /> : <X className="h-3.5 w-3.5 text-muted-foreground/40 mx-auto" />;
}

export default function AboutPage() {
    return (
        <div className="min-h-screen bg-background text-foreground font-sans">
            <header className="sticky top-0 z-50 border-b border-border bg-background/95 backdrop-blur-sm">
                <div className="max-w-6xl mx-auto px-6 h-12 flex items-center justify-between gap-4">
                    <div className="flex items-center gap-6">
                        <Link href="/" className="flex items-center gap-2 font-semibold text-sm">
                            <GitBranch className="h-4 w-4" />
                            GitArena
                        </Link>
                        <nav className="hidden md:flex items-center gap-1">
                            {(["Features", "Self-host", "Docs", "Changelog"] as const).map((item) => (
                                <Link
                                    key={item}
                                    href="#"
                                    className="px-3 py-1.5 text-sm text-muted-foreground hover:text-foreground transition-colors"
                                >
                                    {item}
                                </Link>
                            ))}
                        </nav>
                    </div>
                    <div className="flex items-center gap-2">
                        <Link href="/login" className="px-3 py-1.5 text-sm text-muted-foreground hover:text-foreground transition-colors">
                            Sign in
                        </Link>
                        <Link
                            href="/register"
                            className="px-3 py-1.5 text-sm bg-foreground text-background rounded-md hover:opacity-90 transition-opacity font-medium"
                        >
                            Get started
                        </Link>
                    </div>
                </div>
            </header>

            <main>
                <section className="border-b border-border">
                    <div className="max-w-6xl mx-auto px-6 py-20 flex flex-col items-center text-center gap-6">
                        <div className="inline-flex items-center gap-1.5 border border-border rounded-full px-3 py-1 text-xs text-muted-foreground">
                            <span className="w-1.5 h-1.5 rounded-full bg-green-500" />
                            v0.2.1 just shipped — SSH keys &amp; webhook push events
                            <ChevronRight className="h-3 w-3" />
                        </div>
                        <h1 className="text-4xl md:text-5xl font-semibold tracking-tight text-balance max-w-2xl leading-tight">
                            The git forge that fits in your homelab
                        </h1>
                        <p className="text-muted-foreground text-base leading-relaxed max-w-xl text-balance">
                            GitArena is a lightweight, Rust-powered alternative to Gitea and Forgejo. Ships in minutes with Docker Compose
                        </p>
                        <div className="flex items-center gap-3 flex-wrap justify-center">
                            <Link
                                href="/mellowagain/test"
                                className="flex items-center gap-2 px-5 py-2.5 text-sm bg-foreground text-background rounded-md hover:opacity-90 transition-opacity font-medium"
                            >
                                Try the demo
                                <ArrowRight className="h-4 w-4" />
                            </Link>
                            <Link
                                href="#"
                                className="flex items-center gap-2 px-5 py-2.5 text-sm border border-border rounded-md hover:bg-accent/50 transition-colors"
                            >
                                Documentation
                            </Link>
                        </div>
                        <div className="flex items-center gap-2 border border-border rounded-md px-4 py-2.5 bg-accent/30 font-mono text-sm text-muted-foreground">
                            <Terminal className="h-3.5 w-3.5 shrink-0" />
                            <span>curl -fsSL https://git.mari.zip/install | docker compose -f - up -d</span>
                        </div>
                    </div>
                </section>

                <section className="border-b border-border">
                    <div className="max-w-6xl mx-auto px-6">
                        <div className="flex items-stretch divide-x divide-border">
                            {stats.map((s) => (
                                <div key={s.label} className="flex-1 py-5 px-4 text-center">
                                    <p className="text-lg font-semibold font-mono">{s.value}</p>
                                    <p className="text-xs text-muted-foreground mt-0.5">{s.label}</p>
                                </div>
                            ))}
                        </div>
                    </div>
                </section>

                <section className="border-b border-border">
                    <div className="max-w-6xl mx-auto px-6">
                        <div className="flex gap-0 divide-x divide-border">
                            <div className="flex-1 min-w-0 py-12 pr-10 space-y-14">
                                <div>
                                    <p className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-1">
                                        Issue tracking
                                    </p>
                                    <h2 className="text-xl font-semibold mb-2">Bug reports and feature requests, organised</h2>
                                    <p className="text-sm text-muted-foreground mb-5 leading-relaxed max-w-lg">
                                        Labels, milestones, assignments, and scoped labels to keep large projects tidy without drowning in
                                        noise.
                                    </p>
                                    <div className="border border-border rounded-md overflow-hidden">
                                        <div className="flex items-center gap-4 px-4 py-2 border-b border-border bg-accent/30 text-xs text-muted-foreground">
                                            <span className="flex items-center gap-1.5">
                                                <Circle className="h-3 w-3" /> {demoIssues.filter((i) => i.status !== "done").length} open
                                            </span>
                                            <span className="flex items-center gap-1.5">
                                                <CheckCircle2 className="h-3 w-3" /> {demoIssues.filter((i) => i.status === "done").length}{" "}
                                                closed
                                            </span>
                                        </div>
                                        {demoIssues.map((issue, i) => (
                                            <div
                                                key={issue.id}
                                                className={`flex items-center gap-3 px-4 py-2.5 text-sm ${i < demoIssues.length - 1 ? "border-b border-border" : ""} hover:bg-accent/40 transition-colors`}
                                            >
                                                <IssueStatusIcon status={issue.status} />
                                                <span className="text-xs text-muted-foreground font-mono shrink-0">#{issue.id}</span>
                                                <span className="flex-1 min-w-0 truncate">{issue.title}</span>
                                                <div className="flex items-center gap-1.5 shrink-0">
                                                    {issue.labels.map((l) => (
                                                        <LabelBadge key={l.name} label={l} />
                                                    ))}
                                                </div>
                                                <span className="shrink-0 flex items-center gap-1 text-xs text-muted-foreground">
                                                    <MessageSquare className="h-3 w-3" />
                                                    {issue.comments}
                                                </span>
                                                <span className="text-xs text-muted-foreground shrink-0">{issue.ago}</span>
                                            </div>
                                        ))}
                                    </div>
                                </div>

                                <div>
                                    <p className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-1">
                                        Merge requests
                                    </p>
                                    <h2 className="text-xl font-semibold mb-2">Code review that stays out of your way</h2>
                                    <p className="text-sm text-muted-foreground mb-5 leading-relaxed max-w-lg">
                                        Inline diffs, threaded review comments, and per-commit CI status. Approve and merge without leaving
                                        the browser.
                                    </p>
                                    <div className="border border-border rounded-md overflow-hidden">
                                        <div className="flex items-center gap-4 px-4 py-2 border-b border-border bg-accent/30 text-xs text-muted-foreground">
                                            <span className="flex items-center gap-1.5">
                                                <GitMerge className="h-3 w-3 text-green-500" />{" "}
                                                {demoMRs.filter((m) => m.status === "open").length} open
                                            </span>
                                            <span className="flex items-center gap-1.5">
                                                <GitMerge className="h-3 w-3 text-purple-500" />{" "}
                                                {demoMRs.filter((m) => m.status === "merged").length} merged
                                            </span>
                                        </div>
                                        {demoMRs.map((mr, i) => (
                                            <div
                                                key={mr.id}
                                                className={`flex items-center gap-3 px-4 py-2.5 text-sm ${i < demoMRs.length - 1 ? "border-b border-border" : ""} hover:bg-accent/40 transition-colors`}
                                            >
                                                <MRStatusIcon status={mr.status} />
                                                <CIDot ci={mr.ci} />
                                                <span className="text-xs text-muted-foreground font-mono shrink-0">!{mr.id}</span>
                                                <span className="flex-1 min-w-0 truncate">{mr.title}</span>
                                                <span className="shrink-0 flex items-center gap-1 text-xs text-muted-foreground font-mono">
                                                    <GitBranch className="h-3 w-3" />
                                                    <span className="max-w-[80px] truncate">{mr.source}</span>
                                                </span>
                                                <span className="shrink-0 flex items-center gap-2 text-xs font-mono">
                                                    <span className="text-green-500">+{mr.additions}</span>
                                                    <span className="text-red-500">-{mr.deletions}</span>
                                                </span>
                                            </div>
                                        ))}
                                    </div>
                                </div>

                                <div>
                                    <p className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-1">
                                        Commit history
                                    </p>
                                    <h2 className="text-xl font-semibold mb-2">Every commit, accounted for</h2>
                                    <p className="text-sm text-muted-foreground mb-5 leading-relaxed max-w-lg">
                                        Full commit log with CI status, author attribution, and one-click diffs. Works with every Git
                                        client.
                                    </p>
                                    <div className="border border-border rounded-md overflow-hidden">
                                        {demoCommits.map((c, i) => (
                                            <div
                                                key={c.hash}
                                                className={`flex items-center gap-3 px-4 py-2.5 text-sm ${i < demoCommits.length - 1 ? "border-b border-border" : ""} hover:bg-accent/40 transition-colors`}
                                            >
                                                <CIDot ci={c.ci} />
                                                <span className="font-mono text-xs text-muted-foreground shrink-0">{c.hash}</span>
                                                <span className="flex-1 min-w-0 truncate">{c.message}</span>
                                                <Avatar name={c.author} />
                                                <span className="text-xs text-muted-foreground shrink-0">{c.ago}</span>
                                            </div>
                                        ))}
                                    </div>
                                </div>
                            </div>

                            <div className="w-72 shrink-0 py-12 pl-8 space-y-8">
                                <div>
                                    <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">
                                        Everything included
                                    </h3>
                                    <div className="border border-border rounded-md overflow-hidden">
                                        {features.map((f, i) => (
                                            <div
                                                key={f.label}
                                                className={`flex items-start gap-3 px-3 py-2.5 ${i < features.length - 1 ? "border-b border-border" : ""} hover:bg-accent/40 transition-colors`}
                                            >
                                                <f.icon className="h-3.5 w-3.5 text-muted-foreground mt-0.5 shrink-0" />
                                                <div>
                                                    <p className="text-sm font-medium leading-none mb-1">{f.label}</p>
                                                    <p className="text-xs text-muted-foreground leading-relaxed">{f.desc}</p>
                                                </div>
                                            </div>
                                        ))}
                                    </div>
                                </div>

                                <div>
                                    <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">Self-host</h3>
                                    <div className="border border-border rounded-md overflow-hidden">
                                        {[
                                            { icon: Server, label: "Your data, your server" },
                                            { icon: Users, label: "Unlimited users and repos" },
                                            { icon: Zap, label: "Single binary, ships with SQLite" },
                                            { icon: Key, label: "SSH + HTTPS, no agent required" },
                                            { icon: Globe, label: "Federate with other instances" },
                                            { icon: Lock, label: "No telemetry, no call-home" },
                                        ].map((item, i, arr) => (
                                            <div
                                                key={item.label}
                                                className={`flex items-center gap-3 px-3 py-2.5 text-sm ${i < arr.length - 1 ? "border-b border-border" : ""}`}
                                            >
                                                <item.icon className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                                                {item.label}
                                            </div>
                                        ))}
                                    </div>
                                </div>

                                <div>
                                    <h3 className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">
                                        Latest release
                                    </h3>
                                    <div className="border border-border rounded-md px-3 py-3 space-y-1.5">
                                        <div className="flex items-center justify-between">
                                            <span className="text-sm font-medium font-mono">v0.2.1</span>
                                            <span className="text-xs text-muted-foreground">3 days ago</span>
                                        </div>
                                        <p className="text-xs text-muted-foreground leading-relaxed">
                                            SSH key auth, webhook push events, improved diff renderer.
                                        </p>
                                        <Link
                                            href="#"
                                            className="text-xs text-muted-foreground hover:text-foreground transition-colors flex items-center gap-1 pt-0.5"
                                        >
                                            View changelog <ChevronRight className="h-3 w-3" />
                                        </Link>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </section>

                <section className="border-b border-border">
                    <div className="max-w-6xl mx-auto px-6 py-16">
                        <p className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-1 text-center">
                            How it stacks up
                        </p>
                        <h2 className="text-2xl font-semibold text-center mb-2">GitArena vs the alternatives</h2>
                        <p className="text-sm text-muted-foreground text-center mb-10">
                            No subscriptions. No data leaving your network. No surprises.
                        </p>

                        <div className="border border-border rounded-md overflow-hidden max-w-2xl mx-auto">
                            <div className="grid grid-cols-4 border-b border-border bg-accent/30 text-xs font-medium">
                                <div className="px-4 py-2.5 text-muted-foreground uppercase tracking-widest">Feature</div>
                                <div className="px-4 py-2.5 text-center border-l border-border">GitArena</div>
                                <div className="px-4 py-2.5 text-center border-l border-border text-muted-foreground">GitHub</div>
                                <div className="px-4 py-2.5 text-center border-l border-border text-muted-foreground">GitLab</div>
                            </div>
                            {comparison.map((row, i) => (
                                <div
                                    key={row.feature}
                                    className={`grid grid-cols-4 text-sm ${i < comparison.length - 1 ? "border-b border-border" : ""}`}
                                >
                                    <div className="px-4 py-2.5 text-muted-foreground text-xs">{row.feature}</div>
                                    <div className="px-4 py-2.5 border-l border-border">
                                        <Tick ok={row.gitarena} />
                                    </div>
                                    <div className="px-4 py-2.5 border-l border-border">
                                        <Tick ok={row.github} />
                                    </div>
                                    <div className="px-4 py-2.5 border-l border-border">
                                        <Tick ok={row.gitlab} />
                                    </div>
                                </div>
                            ))}
                        </div>
                    </div>
                </section>

                <section className="border-b border-border">
                    <div className="max-w-6xl mx-auto px-6 py-16">
                        <p className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-8 text-center">
                            What people are saying
                        </p>
                        <div className="grid md:grid-cols-3 gap-0 divide-y md:divide-y-0 md:divide-x divide-border border border-border rounded-md overflow-hidden">
                            {testimonials.map((t) => (
                                <div key={t.author} className="px-6 py-6 space-y-4">
                                    <p className="text-sm leading-relaxed">&ldquo;{t.quote}&rdquo;</p>
                                    <div className="flex items-center gap-2.5">
                                        <span className="w-7 h-7 rounded-full bg-accent border border-border flex items-center justify-center text-xs font-medium text-muted-foreground shrink-0">
                                            {t.author[0]}
                                        </span>
                                        <div>
                                            <p className="text-sm font-medium leading-none">{t.author}</p>
                                            <p className="text-xs text-muted-foreground mt-0.5">{t.role}</p>
                                        </div>
                                    </div>
                                </div>
                            ))}
                        </div>
                    </div>
                </section>

                <section>
                    <div className="max-w-6xl mx-auto px-6 py-20 flex flex-col items-center text-center gap-6">
                        <p className="text-xs font-medium uppercase tracking-widest text-muted-foreground">Get started today</p>
                        <h2 className="text-3xl font-semibold tracking-tight text-balance max-w-lg">Own your Git infrastructure. Fully.</h2>
                        <p className="text-sm text-muted-foreground max-w-md leading-relaxed">
                            One binary. One command. Your repos, your rules. No seats, no storage limits, no surprise bills.
                        </p>
                        <div className="flex items-center gap-3 flex-wrap justify-center">
                            <Link
                                href="/mellowagain/test"
                                className="flex items-center gap-2 px-5 py-2.5 text-sm bg-foreground text-background rounded-md hover:opacity-90 transition-opacity font-medium"
                            >
                                Try the live demo
                                <ArrowRight className="h-4 w-4" />
                            </Link>
                            <Link
                                href="#"
                                className="flex items-center gap-2 px-5 py-2.5 text-sm border border-border rounded-md hover:bg-accent/50 transition-colors"
                            >
                                Self-host in 5 minutes
                            </Link>
                        </div>
                        <div className="flex items-center gap-6 text-xs text-muted-foreground mt-2">
                            <span className="flex items-center gap-1.5">
                                <Check className="h-3 w-3 text-green-500" /> Free forever
                            </span>
                            <span className="flex items-center gap-1.5">
                                <Check className="h-3 w-3 text-green-500" /> No credit card
                            </span>
                            <span className="flex items-center gap-1.5">
                                <Check className="h-3 w-3 text-green-500" /> AGPL-3.0
                            </span>
                        </div>
                    </div>
                </section>
            </main>

            <footer className="border-t border-border">
                <div className="max-w-6xl mx-auto px-6 h-12 flex items-center justify-between text-xs text-muted-foreground">
                    <span className="flex items-center gap-1.5">
                        <GitBranch className="h-3.5 w-3.5" /> GitArena &mdash; AGPL-3.0
                    </span>
                    <div className="flex items-center gap-4">
                        {["Docs", "Source", "Issues", "Releases"].map((l) => (
                            <Link key={l} href="#" className="hover:text-foreground transition-colors">
                                {l}
                            </Link>
                        ))}
                    </div>
                </div>
            </footer>
        </div>
    );
}

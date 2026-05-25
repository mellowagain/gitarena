import Link from "next/link";
import {
    GitMerge,
    MessageSquare,
    CheckCircle2,
    CircleDot,
    Circle,
    Shield,
    Zap,
    Globe,
    Lock,
    Users,
    ArrowRight,
    Server,
    GitBranch,
    BookOpen,
    Braces,
    Sparkles,
    PaintRoller,
} from "lucide-react";
import { TopBar } from "@/components/top-bar";
import { CTA } from "@/components/landing/cta";
import { Footer } from "@/components/landing/footer";

const demoIssues = [
    {
        id: 19,
        status: "open",
        title: "Allow Git access over SSH",
        labels: [
            { name: "feature::git", color: "#d21386" },
            { name: "help wanted", color: "#0FE3F1" },
        ],
        comments: 0,
        ago: "2021",
    },
    {
        id: 50,
        status: "open",
        title: "Implement password reset self service",
        labels: [
            { name: "feature::users", color: "#d21386" },
            { name: "type::enhancement", color: "#3371d6" },
        ],
        comments: 0,
        ago: "2022",
    },
    {
        id: 74,
        status: "in_progress",
        title: "Federation with other software forges",
        labels: [
            { name: "feature::federation", color: "#d21386" },
            { name: "status::investigating", color: "#f9ef66" },
        ],
        comments: 0,
        ago: "2025",
    },
    {
        id: 26,
        status: "completed",
        title: "Extend admin panel to allow changing of settings",
        labels: [
            { name: "feature::admin", color: "#d21386" },
            { name: "type::enhancement", color: "#3371d6" },
        ],
        comments: 0,
        ago: "2021",
    },
];

const demoMRs = [
    {
        id: 83,
        status: "open",
        ci: "running",
        title: "Frontend redesign",
        source: "redesign",
        additions: 27841,
        deletions: 186,
    },
    {
        id: 73,
        status: "merged",
        ci: "passed",
        title: "Add /{username}.keys route to list user SSH keys",
        source: "ssh-keys",
        additions: 102,
        deletions: 3,
    },
    {
        id: 52,
        status: "merged",
        ci: "passed",
        title: "IPC between Main <-> Workhorse",
        source: "ipc",
        additions: 695,
        deletions: 91,
    },
    {
        id: 29,
        status: "merged",
        ci: "passed",
        title: "Revamp error handling",
        source: "error-handling",
        additions: 674,
        deletions: 441,
    },
];

const demoCommits = [
    { hash: "0dec4ee", message: "add openapi schema using utopia", author: "Mari", ago: "8d ago", ci: "passed" },
    { hash: "eec8561", message: "fix `REF_DELTA`s not being found within same pack", author: "Mari", ago: "9d ago", ci: "passed" },
    { hash: "a3a02e5", message: "adjust argon config to OWASP recommended", author: "Mari", ago: "6mo ago", ci: "passed" },
    { hash: "b15b33a", message: "modernize docker files and add healthz endpoint", author: "Mari", ago: "6mo ago", ci: "passed" },
];

const staticStats = [
    { label: "Written in", value: "Rust" },
    { label: "License", value: "MIT" },
];

interface GithubStats {
    openPRs: number;
    openIssues: number;
    contributors: number;
    forks: number;
    stars: number;
}

async function getGithubStats(): Promise<GithubStats | null> {
    try {
        const headers = process.env.GITHUB_TOKEN ? { Authorization: `Bearer ${process.env.GITHUB_TOKEN}` } : undefined;
        const [repoRes, prsRes, contributorsRes] = await Promise.all([
            fetch("https://api.github.com/repos/mellowagain/gitarena", { headers, next: { revalidate: 3600 } }),
            fetch("https://api.github.com/repos/mellowagain/gitarena/pulls?state=open&per_page=1", { headers, next: { revalidate: 3600 } }),
            fetch("https://api.github.com/repos/mellowagain/gitarena/contributors?per_page=100", { headers, next: { revalidate: 3600 } }),
        ]);
        if (!repoRes.ok) {
            return null;
        }
        const repo = await repoRes.json();
        const prs = prsRes.ok ? await prsRes.json() : [];
        const contributors = contributorsRes.ok ? await contributorsRes.json() : [];
        return {
            openPRs: Array.isArray(prs) ? prs.length : 0,
            openIssues: repo.open_issues_count ?? 0,
            contributors: Array.isArray(contributors) ? contributors.length : 0,
            forks: repo.forks_count ?? 0,
            stars: repo.stargazers_count ?? 0,
        };
    } catch {
        return null;
    }
}

const features = [
    { icon: GitBranch, label: "Git hosting", desc: "Full Git protocol over HTTPS and SSH with LFS support." },
    { icon: Shield, label: "Access control", desc: "Org-level roles, repo visibility, deploy keys." },
    { icon: Zap, label: "CI integration", desc: "Webhook-driven pipelines — bring your own runner.", wip: true },
    {
        icon: Lock,
        label: "Privacy first",
        desc: "No telemetry, no analytics, no call-home. Self-host and your data never leaves your server.",
    },
    {
        icon: Globe,
        label: "Federation",
        desc: "Follow repos and users across the ForgeFed universe, including other GitArena instances.",
        wip: true,
    },
    {
        icon: Braces,
        label: "Open API",
        desc: "Full REST API with an OpenAPI spec. Any registered user can generate an API key.",
    },
    {
        icon: Sparkles,
        label: "AI native",
        desc: "Our MCP server lets AI agents browse repos, read issues, and open merge requests.",
        wip: true,
    },
];

function IssueStatusIcon({ status }: { status: string }) {
    if (status === "in_progress") {
        return <CircleDot className="h-4 w-4 text-yellow-500 shrink-0" />;
    }
    if (status === "completed") {
        return <CheckCircle2 className="h-4 w-4 text-muted-foreground shrink-0" />;
    }
    return <Circle className="h-4 w-4 text-green-500 shrink-0" />;
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

import { InstanceConfig } from "@/lib/instance-config";

async function getApiInfo(): Promise<InstanceConfig | null> {
    try {
        const backendUrl = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080";
        const res = await fetch(`${backendUrl}/api`, { next: { revalidate: 3600 } });
        if (!res.ok) {
            return null;
        }
        return res.json();
    } catch {
        return null;
    }
}

export default async function AboutPage() {
    const [apiInfo, githubStats] = await Promise.all([getApiInfo(), getGithubStats()]);

    const activityLabel = githubStats
        ? githubStats.openPRs > 0
            ? { label: githubStats.openPRs === 1 ? "Open MR" : "Open MRs", value: String(githubStats.openPRs) }
            : { label: githubStats.openIssues === 1 ? "Open issue" : "Open issues", value: String(githubStats.openIssues) }
        : { label: "Open issues", value: "19" };

    const stats = [
        ...staticStats,
        activityLabel,
        { label: "Contributors", value: githubStats ? String(githubStats.contributors) : "3" },
        { label: "Forks", value: githubStats ? String(githubStats.forks) : "11" },
        { label: "Stars", value: githubStats ? String(githubStats.stars) : "99" },
    ];
    return (
        <div className="min-h-screen bg-background text-foreground font-sans">
            <TopBar
                navLinks={[
                    { label: "Documentation", href: "/docs", icon: <BookOpen className="h-[18px] w-[18px]" />, external: true },
                    {
                        label: "Source code",
                        href: "https://github.com/mellowagain/gitarena",
                        icon: <GitBranch className="h-[18px] w-[18px]" />,
                    },
                ]}
            />

            <main>
                <section className="border-b border-border">
                    <div className="max-w-6xl mx-auto px-6 py-20 flex flex-col items-center text-center gap-6">
                        <h1 className="text-4xl md:text-5xl font-semibold tracking-tight text-balance max-w-2xl leading-tight">
                            The git forge that fits in your homelab
                        </h1>
                        <p className="text-muted-foreground text-base leading-relaxed max-w-xl text-balance">
                            Take control of <b>your</b> data with GitArena, a lightweight, self-hosted Git forge written in Rust. Deploy in
                            one command with Docker Compose.
                        </p>
                        <div className="flex items-center gap-3 flex-wrap justify-center">
                            <Link
                                href="/explore"
                                className="flex items-center gap-2 px-5 py-2.5 text-sm bg-foreground text-background rounded-md hover:opacity-90 transition-opacity font-medium"
                            >
                                Explore instance
                                <ArrowRight className="h-4 w-4" />
                            </Link>
                            <a
                                href="/docs/quickstart"
                                className="flex items-center gap-2 px-5 py-2.5 text-sm border border-border rounded-md hover:bg-accent/50 transition-colors"
                            >
                                Deploy your own
                            </a>
                        </div>

                        <p className="text-xs text-muted-foreground">
                            The hosted instance at{" "}
                            <a href="https://git.mari.zip" className="underline underline-offset-2 hover:text-foreground transition-colors">
                                git.mari.zip
                            </a>{" "}
                            is invite-only for now.
                        </p>
                    </div>
                </section>

                <section className="border-b border-border">
                    <div className="max-w-6xl mx-auto px-6">
                        <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6">
                            {stats.map((s) => (
                                <div
                                    key={s.label}
                                    className="py-5 px-4 text-center border-r border-b border-border last:border-r-0 [&:nth-child(2n)]:border-r-0 sm:[&:nth-child(2n)]:border-r sm:[&:nth-child(3n)]:border-r-0 lg:[&:nth-child(3n)]:border-r lg:[&:nth-child(6n)]:border-r-0 lg:[&:nth-last-child(-n+6)]:border-b-0 [&:nth-last-child(-n+2)]:border-b-0 sm:[&:nth-last-child(-n+3)]:border-b-0"
                                >
                                    <p className="text-lg font-semibold font-mono">{s.value}</p>
                                    <p className="text-xs text-muted-foreground mt-0.5">{s.label}</p>
                                </div>
                            ))}
                        </div>
                    </div>
                </section>

                <section className="border-b border-border">
                    <div className="max-w-6xl mx-auto px-6">
                        <div className="flex flex-col lg:flex-row gap-0 lg:divide-x divide-border">
                            <div className="flex-1 min-w-0 py-12 lg:pr-10 space-y-14">
                                <div>
                                    <p className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-1 flex items-center gap-2">
                                        Issue tracking
                                    </p>
                                    <h2 className="text-xl font-semibold mb-2">Bug reports and feature requests, organised</h2>
                                    <p className="text-sm text-muted-foreground mb-5 leading-relaxed max-w-lg">
                                        Issues live inside the repository as git objects, stored in the same format as{" "}
                                        <a
                                            href="https://github.com/git-bug/git-bug"
                                            className="underline underline-offset-2 hover:text-foreground transition-colors"
                                        >
                                            git-bug
                                        </a>
                                        . No separate database needed. Enjoy full offline access to your issues.
                                    </p>
                                    <div className="border border-border rounded-md overflow-hidden">
                                        <div className="flex items-center gap-4 px-4 py-2 border-b border-border bg-accent/30 text-xs text-muted-foreground">
                                            <span className="flex items-center gap-1.5">
                                                <Circle className="h-3 w-3" /> {demoIssues.filter((i) => i.status !== "completed").length}{" "}
                                                open
                                            </span>
                                            <span className="flex items-center gap-1.5">
                                                <CheckCircle2 className="h-3 w-3" />{" "}
                                                {demoIssues.filter((i) => i.status === "completed").length} closed
                                            </span>
                                        </div>
                                        {demoIssues.map((issue, i) => (
                                            <div
                                                key={issue.id}
                                                className={`flex items-center gap-3 px-4 py-2.5 text-sm min-w-0 ${i < demoIssues.length - 1 ? "border-b border-border" : ""} hover:bg-accent/40 transition-colors`}
                                            >
                                                <IssueStatusIcon status={issue.status} />
                                                <span className="text-xs text-muted-foreground font-mono w-8 shrink-0">#{issue.id}</span>
                                                <span className="flex-1 min-w-0 truncate">{issue.title}</span>
                                                <div className="hidden sm:flex items-center gap-1.5 shrink-0">
                                                    {issue.labels.map((l, li) => (
                                                        <span key={l.name} className={li > 0 ? "hidden lg:contents" : undefined}>
                                                            <LabelBadge label={l} />
                                                        </span>
                                                    ))}
                                                </div>
                                                <span className="hidden sm:flex w-8 shrink-0 items-center gap-1 text-xs text-muted-foreground justify-end">
                                                    <MessageSquare className="h-3 w-3" />
                                                    {issue.comments}
                                                </span>
                                                <span className="text-xs text-muted-foreground shrink-0 w-10 text-right">{issue.ago}</span>
                                            </div>
                                        ))}
                                    </div>
                                </div>

                                <div>
                                    <p className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-1 flex items-center gap-2">
                                        Merge requests
                                        <span className="px-1 py-0.5 text-[10px] font-medium uppercase tracking-wide rounded bg-yellow-500/15 text-yellow-500 border border-yellow-500/20">
                                            WIP
                                        </span>
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
                                                <span className="text-xs text-muted-foreground font-mono w-8 shrink-0">!{mr.id}</span>
                                                <span className="flex-1 min-w-0 truncate">{mr.title}</span>
                                                <span className="hidden sm:flex w-24 shrink-0 items-center gap-1 text-xs text-muted-foreground font-mono justify-end">
                                                    <GitBranch className="h-3 w-3 shrink-0" />
                                                    <span className="truncate">{mr.source}</span>
                                                </span>
                                                <span className="hidden sm:flex w-28 shrink-0 items-center gap-1.5 text-xs font-mono justify-end">
                                                    <span className="text-green-500 w-14 text-right">+{mr.additions}</span>
                                                    <span className="text-red-500 w-10 text-right">-{mr.deletions}</span>
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
                                                <span className="font-mono text-xs text-muted-foreground w-14 shrink-0">{c.hash}</span>
                                                <span className="flex-1 min-w-0 truncate">{c.message}</span>
                                                <Avatar name={c.author} />
                                                <span className="text-xs text-muted-foreground shrink-0 w-16 text-right">{c.ago}</span>
                                            </div>
                                        ))}
                                    </div>
                                </div>
                            </div>

                            <div className="w-full lg:w-72 shrink-0 py-12 lg:pl-8 lg:border-t-0 border-t border-border space-y-8">
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
                                                    <p className="text-sm font-medium leading-none mb-1 flex items-center gap-2">
                                                        {f.label}
                                                        {f.wip && (
                                                            <span className="px-1 py-0.5 text-[10px] font-medium uppercase tracking-wide rounded bg-yellow-500/15 text-yellow-500 border border-yellow-500/20">
                                                                WIP
                                                            </span>
                                                        )}
                                                    </p>
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
                                            { icon: Zap, label: "Single Docker Compose" },
                                            { icon: Globe, label: "Federate with the ForgeFed" },
                                            { icon: Lock, label: "No telemetry, no call-home" },
                                            { icon: PaintRoller, label: "Whitelabel and customize" },
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
                            </div>
                        </div>
                    </div>
                </section>

                <CTA />
            </main>

            <Footer apiInfo={apiInfo} />
        </div>
    );
}

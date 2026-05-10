"use client";

import { Star, GitFork, Eye, Copy, Check, ChevronDown, ExternalLink, Scale, Users, Package, Calendar } from "lucide-react";
import Link from "next/link";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import useSWR from "swr";
import { ErrorDisplay } from "@/components/error-display";
import { RepoPageSkeleton } from "@/app/[user]/[repo]/page";
import { useInstanceConfig } from "@/components/instance-config-provider";
import prettyBytes from "pretty-bytes";
import { LanguageBar } from "@/components/language-bar";
import { useLocalStorage } from "@/hooks/use-local-storage";

type Release = {
    tag: string;
    name: string;
    date: string;
};

type Contributor = {
    name: string;
    commits: number;
    avatarUrl: string | null;
};

export type RepoSidebarProps = {
    user: string;
    repo: string;
    description: string;
    projectId: string;
    license?: string;
    websiteUrl?: string;
    createdAt?: string;
    topics: string[];
    languages: Record<string, number>;
    latestRelease?: Release;
    contributors?: Contributor[];
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

interface RepoStats {
    size: number;
    stars: RepoDetailedStats;
    forks: RepoDetailedStats;
    watchers: RepoDetailedStats;
}

interface RepoDetailedStats {
    count: number;
    self: boolean;
}

function StatButton({
    icon: Icon,
    count,
    active,
    onClick,
}: {
    icon: React.ElementType;
    count: number;
    active: boolean;
    onClick?: () => void;
}) {
    return (
        <button
            onClick={onClick}
            className={`flex items-center gap-1.5 px-2 py-1 text-sm border rounded transition-colors ${
                active
                    ? "text-yellow-400 border-yellow-400/40 bg-yellow-400/10 hover:bg-yellow-400/20"
                    : "text-muted-foreground border-border hover:text-foreground hover:bg-accent/50"
            }`}
        >
            <Icon className={`h-3.5 w-3.5${active ? " fill-current" : ""}`} />
            <span>{count}</span>
        </button>
    );
}

export function RepoSidebar({
    user,
    repo,
    description,
    projectId,
    license,
    websiteUrl,
    createdAt,
    topics,
    languages,
    latestRelease,
    contributors,
}: RepoSidebarProps) {
    const [protocol, setProtocol] = useState<"https" | "ssh">("https");
    const [allLanguages] = useLocalStorage<boolean>("gitarena:all-languages", false);
    const instanceConfig = useInstanceConfig();
    const apiUrl = process.env.NEXT_PUBLIC_API_URL ?? "";
    const host = apiUrl.replace(/^https?:\/\//, "");
    const sshEnabled = instanceConfig?.sshPort != null;
    const sshCloneUrl =
        instanceConfig?.sshPort === 22
            ? `git@${host}:${user}/${repo}.git`
            : `ssh://git@${host}:${instanceConfig?.sshPort}/${user}/${repo}.git`;
    const cloneUrl = protocol === "https" ? `${apiUrl}/${user}/${repo}.git` : sshCloneUrl;

    const { data, error, isLoading } = useSWR<RepoStats>(`/api/repos/${user}/${repo}/stats`);

    if (isLoading) {
        return <RepoPageSkeleton user={user} repo={repo} />;
    }

    if (error || !data) {
        return <ErrorDisplay failed={"stats"} error={error} />;
    }

    return (
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
                                {sshEnabled && <DropdownMenuItem onClick={() => setProtocol("ssh")}>SSH</DropdownMenuItem>}
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
                            <StatButton
                                icon={Star}
                                count={data.stars.count}
                                active={data.stars.self}
                                onClick={() => console.log("clicked")}
                            />
                            <StatButton
                                icon={GitFork}
                                count={data.forks.count}
                                active={data.forks.self}
                                onClick={() => console.log("clicked")}
                            />
                            <StatButton
                                icon={Eye}
                                count={data.watchers.count}
                                active={data.watchers.self}
                                onClick={() => console.log("clicked")}
                            />
                        </div>
                    </div>
                    <p className="text-foreground leading-relaxed mb-4">{description}</p>
                    {websiteUrl && (
                        <a
                            href={websiteUrl}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="inline-flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors mb-4"
                        >
                            <ExternalLink className="h-3.5 w-3.5" />
                            {websiteUrl.replace(/^https?:\/\//, "")}
                        </a>
                    )}
                    {topics && topics.length > 0 && (
                        <div className="flex flex-wrap gap-2 mb-4">
                            {topics.map((topic) => (
                                <Link
                                    key={topic}
                                    href={`/topics/${encodeURI(topic)}`}
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
                            <Tooltip>
                                <TooltipTrigger asChild>
                                    <span className="font-mono text-foreground cursor-default">{projectId.slice(-7)}</span>
                                </TooltipTrigger>
                                <TooltipContent>
                                    <span className="font-mono">{projectId}</span>
                                </TooltipContent>
                            </Tooltip>
                        </div>
                        <div className="flex items-center justify-between text-muted-foreground">
                            <span>Size</span>
                            <span className="text-foreground">{prettyBytes(data.size)}</span>
                        </div>

                        {license && (
                            <div className="flex items-center justify-between text-muted-foreground">
                                <span>License</span>
                                <span className="text-foreground flex items-center gap-1.5">
                                    <Scale className="h-3.5 w-3.5" />
                                    {license}
                                </span>
                            </div>
                        )}

                        {createdAt && (
                            <div className="flex items-center justify-between text-muted-foreground">
                                <span>Created</span>
                                <span className="text-foreground flex items-center gap-1.5">
                                    <Calendar className="h-3.5 w-3.5" />
                                    {createdAt}
                                </span>
                            </div>
                        )}
                    </div>
                </div>

                {Object.keys(languages).length > 0 && (
                    <div className="pt-4 border-t border-border">
                        <h3 className="text-sm font-medium uppercase tracking-wider text-muted-foreground mb-3">Languages</h3>
                        <LanguageBar languages={languages} allLanguages={allLanguages} />
                    </div>
                )}

                {latestRelease && (
                    <div className="pt-4 border-t border-border">
                        <div className="flex items-center justify-between mb-3">
                            <h3 className="text-sm font-medium uppercase tracking-wider text-muted-foreground">Releases</h3>
                            <Link href="#" className="text-xs text-muted-foreground hover:text-foreground transition-colors">
                                View all
                            </Link>
                        </div>
                        <Link href="#" className="flex items-center gap-3 p-3 -mx-3 rounded-md hover:bg-accent/50 transition-colors group">
                            <Package className="h-5 w-5 text-muted-foreground shrink-0" />
                            <div className="min-w-0 flex-1">
                                <div className="flex items-center gap-2">
                                    <span className="font-medium text-foreground">{latestRelease.tag}</span>
                                    <span className="px-2 py-0.5 text-[10px] uppercase tracking-wider bg-secondary text-muted-foreground rounded">
                                        Latest
                                    </span>
                                </div>
                                <div className="text-sm text-muted-foreground truncate">
                                    {latestRelease.name} · {latestRelease.date}
                                </div>
                            </div>
                        </Link>
                    </div>
                )}

                {contributors && (
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
                                    {contributors.slice(0, 3).map((c) => (
                                        <div
                                            key={c.name}
                                            className="flex h-6 w-6 items-center justify-center rounded-full bg-secondary text-xs font-medium border-2 border-background"
                                        >
                                            {c.name[0].toUpperCase()}
                                        </div>
                                    ))}
                                </div>
                                <span>{contributors.length}</span>
                            </span>
                        </Link>
                    </div>
                )}
            </div>
        </aside>
    );
}

export function RepoSidebarSkeleton() {
    return (
        <aside className="w-[340px] border-l border-border shrink-0 overflow-y-auto animate-pulse">
            <div className="p-5 space-y-5">
                {/* Clone section */}
                <div className="space-y-2">
                    <div className="flex items-center justify-between">
                        <div className="h-3 w-12 rounded bg-accent" />
                        <div className="h-5 w-16 rounded bg-accent" />
                    </div>
                    <div className="h-9 w-full rounded bg-accent" />
                </div>

                {/* About section */}
                <div className="pt-4 border-t border-border space-y-3">
                    <div className="flex items-center justify-between">
                        <div className="h-3 w-12 rounded bg-accent" />
                        <div className="flex gap-1.5">
                            {[1, 2, 3].map((i) => (
                                <div key={i} className="h-6 w-12 rounded bg-accent" />
                            ))}
                        </div>
                    </div>
                    <div className="h-3 w-full rounded bg-accent" />
                    <div className="h-3 w-4/5 rounded bg-accent" />
                    <div className="h-3 w-24 rounded bg-accent" />
                    <div className="flex gap-2">
                        {[1, 2, 3].map((i) => (
                            <div key={i} className="h-5 w-14 rounded-full bg-accent" />
                        ))}
                    </div>
                    {[1, 2, 3, 4].map((i) => (
                        <div key={i} className="flex items-center justify-between">
                            <div className="h-3 w-16 rounded bg-accent" />
                            <div className="h-3 w-20 rounded bg-accent" />
                        </div>
                    ))}
                </div>

                {/* Languages section */}
                <div className="pt-4 border-t border-border space-y-2.5">
                    <div className="h-3 w-20 rounded bg-accent" />
                    <div className="h-2.5 w-full rounded-full bg-accent" />
                    <div className="flex gap-4">
                        {[1, 2, 3].map((i) => (
                            <div key={i} className="h-3 w-16 rounded bg-accent" />
                        ))}
                    </div>
                </div>

                {/* Releases section */}
                <div className="pt-4 border-t border-border space-y-2">
                    <div className="h-3 w-16 rounded bg-accent" />
                    <div className="flex items-center gap-3 p-3 -mx-3">
                        <div className="h-5 w-5 rounded bg-accent shrink-0" />
                        <div className="flex-1 space-y-1.5">
                            <div className="h-3 w-20 rounded bg-accent" />
                            <div className="h-2.5 w-32 rounded bg-accent" />
                        </div>
                    </div>
                </div>

                {/* Contributors section */}
                <div className="pt-4 border-t border-border flex items-center justify-between">
                    <div className="h-3 w-24 rounded bg-accent" />
                    <div className="flex -space-x-2">
                        {[1, 2, 3].map((i) => (
                            <div key={i} className="h-6 w-6 rounded-full bg-accent border-2 border-background" />
                        ))}
                    </div>
                </div>
            </div>
        </aside>
    );
}

"use client";

import { useState } from "react";
import Link from "next/link";
import { useParams } from "next/navigation";
import { TopBar } from "@/components/top-bar";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { ErrorDisplay } from "@/components/error-display";
import { jsonFetcher, deleteFetcher } from "@/lib/fetchers";
import useSWR from "swr";
import useSWRMutation from "swr/mutation";
import { formatDistanceToNow } from "date-fns";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Spinner } from "@/components/ui/spinner";
import { Tag, GitCommit, Code, AlertCircle, GitMerge, Settings, Search, Calendar, Trash2 } from "lucide-react";

interface TagInfo {
    name: string;
    commit: string;
    commitMessage: string;
    author: {
        name: string;
        email: string;
        timestamp: number;
        uid: string | null;
    };
    date: string;
    kind: "lightweight" | "annotated";
    message?: string;
}

interface TagsResponse {
    tags: TagInfo[];
}

interface PermissionsResponse {
    permissions: {
        view: boolean;
        push: boolean;
        manageIssues: boolean;
        admin: boolean;
    };
}

function TagRowSkeleton() {
    return (
        <div className="flex items-start gap-4 px-4 py-4">
            <Skeleton className="h-8 w-8 rounded-md shrink-0" />
            <div className="flex-1 space-y-2">
                <Skeleton className="h-4 w-24" />
                <Skeleton className="h-3 w-64" />
            </div>
        </div>
    );
}

export default function TagsPage() {
    const { user, repo } = useParams<{ user: string; repo: string }>();
    const [search, setSearch] = useState("");
    const [tagToDelete, setTagToDelete] = useState<string | null>(null);
    const [isDeleting, setIsDeleting] = useState(false);

    const tagsKey = user && repo ? `/api/repos/${user}/${repo}/tags` : null;
    const { data, isLoading, error, mutate } = useSWR<TagsResponse>(tagsKey, jsonFetcher);

    const { data: permsData } = useSWR<PermissionsResponse>(user && repo ? `/api/repos/${user}/${repo}/permissions` : null, jsonFetcher);

    const { trigger: triggerDelete } = useSWRMutation(tagsKey, (url: string, { arg }: { arg: string }) => deleteFetcher(`${url}/${arg}`));

    const canPush = permsData?.permissions.push ?? false;

    const navLinks = [
        { label: "Code", href: `/${user}/${repo}`, icon: <Code className="h-[18px] w-[18px]" /> },
        { label: "Issues", href: `/${user}/${repo}/issues`, icon: <AlertCircle className="h-[18px] w-[18px]" /> },
        { label: "Merge Requests", href: `/${user}/${repo}/merge-requests`, icon: <GitMerge className="h-[18px] w-[18px]" /> },
        { label: "Settings", href: `/${user}/${repo}/settings`, icon: <Settings className="h-[18px] w-[18px]" /> },
    ];
    const tags = data?.tags ?? [];
    const filtered = tags.filter(
        (t) => t.name.toLowerCase().includes(search.toLowerCase()) || t.commitMessage.toLowerCase().includes(search.toLowerCase())
    );

    async function handleDelete(tagName: string) {
        setIsDeleting(true);
        try {
            await triggerDelete(tagName);
            await mutate();
        } finally {
            setIsDeleting(false);
            setTagToDelete(null);
        }
    }

    return (
        <div className="flex flex-col h-screen bg-background">
            <TopBar
                breadcrumb={[{ label: user, href: `/${user}` }, { label: repo, href: `/${user}/${repo}` }, { label: "Tags" }]}
                search={{ placeholder: "Search GitArena…" }}
                navLinks={navLinks}
            />

            <div className="flex-1 overflow-y-auto">
                <div className="max-w-4xl mx-auto px-6 py-8 space-y-6">
                    {/* Header */}
                    <div className="flex items-center justify-between">
                        <div>
                            <h1 className="text-2xl font-semibold flex items-center gap-2">
                                <Tag className="h-5 w-5 text-muted-foreground" />
                                Tags
                            </h1>
                            <span className="block text-sm text-muted-foreground mt-1">
                                {isLoading ? (
                                    <Skeleton className="h-4 w-32 inline-block" />
                                ) : (
                                    <>
                                        {tags.length} tags in{" "}
                                        <Link href={`/${user}/${repo}`} className="hover:text-foreground transition-colors">
                                            {user}/{repo}
                                        </Link>
                                    </>
                                )}
                            </span>
                        </div>

                        <div className="flex items-center gap-2">
                            <Link
                                href={`/${user}/${repo}/commits/main`}
                                className="flex items-center gap-2 h-9 px-3 text-sm text-muted-foreground border border-border rounded-md hover:text-foreground hover:bg-accent/50 transition-colors"
                            >
                                <GitCommit className="h-4 w-4" />
                                Commits
                            </Link>
                        </div>
                    </div>

                    {/* Search */}
                    <div className="relative">
                        <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground pointer-events-none" />
                        <input
                            type="text"
                            placeholder="Search tags…"
                            value={search}
                            onChange={(e) => setSearch(e.target.value)}
                            className="w-full h-9 pl-9 pr-3 bg-card border border-border rounded-md text-sm focus:outline-none focus:ring-1 focus:ring-ring"
                        />
                    </div>

                    {/* Tag list */}
                    {error ? (
                        <ErrorDisplay failed="tags" error={error} />
                    ) : isLoading ? (
                        <div className="border border-border rounded-lg overflow-hidden">
                            {Array.from({ length: 4 }, (_, i) => (
                                <div key={i} className={i > 0 ? "border-t border-border" : ""}>
                                    <TagRowSkeleton />
                                </div>
                            ))}
                        </div>
                    ) : filtered.length === 0 ? (
                        <div className="flex flex-col items-center justify-center py-20 text-center border border-border rounded-lg">
                            <Tag className="h-10 w-10 text-muted-foreground mb-3" />
                            <p className="font-medium">No tags found</p>
                            <p className="text-sm text-muted-foreground mt-1">
                                {search ? "Try a different search term." : "This repository has no tags yet."}
                            </p>
                        </div>
                    ) : (
                        <div className="border border-border rounded-lg overflow-hidden">
                            {filtered.map((tag, i) => (
                                <div
                                    key={tag.name}
                                    className={`flex items-start gap-4 px-4 py-4 hover:bg-accent/20 transition-colors ${
                                        i > 0 ? "border-t border-border" : ""
                                    }`}
                                >
                                    {/* Left: tag icon */}
                                    <div className="shrink-0 mt-0.5">
                                        <div className="h-8 w-8 rounded-md bg-secondary border border-border flex items-center justify-center">
                                            <Tag className="h-4 w-4 text-muted-foreground" />
                                        </div>
                                    </div>

                                    {/* Center: tag info */}
                                    <div className="flex-1 min-w-0 space-y-1.5">
                                        <div className="flex items-center gap-2 flex-wrap">
                                            <Link
                                                href={`/${user}/${repo}/tree/${tag.name}`}
                                                className="font-semibold font-mono hover:text-primary transition-colors"
                                            >
                                                {tag.name}
                                            </Link>
                                            {tag.message && (
                                                <span className="px-1.5 py-0.5 text-[10px] uppercase tracking-wider font-medium border border-border rounded bg-secondary text-muted-foreground">
                                                    annotated
                                                </span>
                                            )}
                                        </div>

                                        {/* Annotated tag message */}
                                        {tag.message && <p className="text-sm text-muted-foreground">{tag.message}</p>}

                                        {/* Commit info */}
                                        <div className="flex items-center gap-2 text-xs text-muted-foreground flex-wrap">
                                            <Link
                                                href={`/${user}/${repo}/commit/${tag.commit}`}
                                                className="flex items-center gap-1 font-mono hover:text-foreground transition-colors"
                                            >
                                                <GitCommit className="h-3.5 w-3.5" />
                                                {tag.commit.slice(0, 7)}
                                            </Link>
                                            <span className="text-border">·</span>
                                            <span className="truncate">{tag.commitMessage}</span>
                                            <span className="text-border">·</span>
                                            <span className="flex items-center gap-1">
                                                <Calendar className="h-3 w-3" />
                                                {formatDistanceToNow(new Date(tag.date), { addSuffix: true })}
                                            </span>
                                            <span className="text-border">·</span>
                                            {tag.author.uid !== null ? (
                                                <Link href={`/${tag.author.name}`} className="hover:text-foreground transition-colors">
                                                    {tag.author.name}
                                                </Link>
                                            ) : (
                                                <span>{tag.author.name}</span>
                                            )}
                                        </div>
                                    </div>

                                    {/* Right: actions */}
                                    <div className="flex items-center gap-1.5 shrink-0">
                                        {canPush && (
                                            <Button
                                                variant="ghost"
                                                size="sm"
                                                className="h-8 w-8 p-0 text-muted-foreground hover:text-destructive"
                                                onClick={() => setTagToDelete(tag.name)}
                                            >
                                                <Trash2 className="h-3.5 w-3.5" />
                                            </Button>
                                        )}
                                    </div>
                                </div>
                            ))}
                        </div>
                    )}
                </div>
            </div>

            <Dialog
                open={tagToDelete !== null}
                onOpenChange={(open) => {
                    if (!isDeleting) {
                        setTagToDelete(open ? tagToDelete : null);
                    }
                }}
            >
                <DialogContent showCloseButton={!isDeleting}>
                    <DialogHeader>
                        <DialogTitle>Delete tag &ldquo;{tagToDelete}&rdquo;?</DialogTitle>
                        <DialogDescription>
                            This will permanently delete the tag from the repository. This action cannot be undone.
                        </DialogDescription>
                    </DialogHeader>
                    <DialogFooter>
                        <Button variant="outline" disabled={isDeleting} onClick={() => setTagToDelete(null)}>
                            Cancel
                        </Button>
                        <Button variant="destructive" disabled={isDeleting} onClick={() => tagToDelete && handleDelete(tagToDelete)}>
                            {isDeleting && <Spinner className="mr-2" />}
                            Delete
                        </Button>
                    </DialogFooter>
                </DialogContent>
            </Dialog>
        </div>
    );
}

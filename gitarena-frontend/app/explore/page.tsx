"use client";

import { useState } from "react";
import useSWRInfinite from "swr/infinite";
import Link from "next/link";
import { TopBar } from "@/components/top-bar";
import { Button } from "@/components/ui/button";
import { Star, Lock, Globe, Clock, ChevronDown, Compass, GitMerge, Sparkles, Search } from "lucide-react";
import { jsonFetcher } from "@/lib/fetchers";
import { Badge } from "@/components/ui/badge";
import * as allLangs from "linguist-languages";

function languageColor(name: string): string {
    const color = (allLangs as Record<string, { color?: string }>)[name]?.color;
    if (color) {
        return color;
    }
    let hash = 0;
    for (let i = 0; i < name.length; i++) {
        hash = name.charCodeAt(i) + ((hash << 5) - hash);
    }
    return `hsl(${Math.abs(hash) % 360}, 60%, 55%)`;
}

const PAGE_SIZE = 20;

interface ExploreRepo {
    id: string;
    name: string;
    description: string;
    ownerId: string;
    ownerName: string;
    visibility: string;
    archivedAt: string | null;
    disabled: boolean;
    languages: Record<string, number>;
    stars: number;
    issues: number;
    mergeRequests: number;
}

interface ExploreResponse {
    repositories: ExploreRepo[];
}

type Category = "popular" | "new";

const categories: { id: Category; label: string; icon: React.ElementType; sort: string }[] = [
    { id: "popular", label: "Popular", icon: Star, sort: "stars_desc" },
    { id: "new", label: "New", icon: Sparkles, sort: "id_desc" },
];

function getPrimaryLanguage(languages: Record<string, number>): string | null {
    const entries = Object.entries(languages);
    if (entries.length === 0) {
        return null;
    }
    return entries.reduce((a, b) => (b[1] > a[1] ? b : a))[0];
}

function formatNumber(num: number): string {
    if (num >= 1000) {
        return (num / 1000).toFixed(1).replace(/\.0$/, "") + "k";
    }
    return num.toString();
}

function RepoRow({ repo, rank }: { repo: ExploreRepo; rank: number }) {
    const primaryLang = getPrimaryLanguage(repo.languages);

    return (
        <div className="flex items-center gap-4 px-4 py-3 border-b border-border hover:bg-accent/30 transition-colors">
            <div className="shrink-0 w-8 text-lg font-medium text-muted-foreground/50 text-center">{rank}</div>

            <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                    <Link href={`/${repo.ownerName}/${repo.name}`} className="font-medium hover:underline truncate">
                        <span className="text-muted-foreground">{repo.ownerName}</span>
                        <span className="text-muted-foreground mx-0.5">/</span>
                        <span>{repo.name}</span>
                    </Link>
                    {repo.visibility === "private" && (
                        <Badge variant="secondary" className="shrink-0">
                            <Lock className="h-3 w-3" />
                            Private
                        </Badge>
                    )}
                    {repo.visibility === "internal" && (
                        <Badge variant="outline" className="shrink-0">
                            <Globe className="h-3 w-3" />
                            Internal
                        </Badge>
                    )}
                    {repo.archivedAt && (
                        <span className="px-1.5 py-0.5 text-[10px] font-medium rounded bg-secondary text-muted-foreground border border-border leading-none shrink-0">
                            archived
                        </span>
                    )}
                </div>
                {repo.description && <p className="text-sm text-muted-foreground truncate mt-0.5">{repo.description}</p>}
            </div>

            {primaryLang && (
                <div className="hidden sm:flex items-center gap-2 shrink-0 w-28">
                    <span className="w-3 h-3 rounded-full shrink-0" style={{ backgroundColor: languageColor(primaryLang) }} />
                    <span className="text-sm text-muted-foreground truncate">{primaryLang}</span>
                </div>
            )}

            <div className="flex items-center gap-4 shrink-0 text-sm text-muted-foreground">
                <div className="flex items-center gap-1">
                    <Star className="h-4 w-4" />
                    <span>{formatNumber(repo.stars)}</span>
                </div>
            </div>
        </div>
    );
}

function RepoRowSkeleton({ rank }: { rank: number }) {
    return (
        <div className="flex items-center gap-4 px-4 py-3 border-b border-border">
            <div className="shrink-0 w-8 text-lg font-medium text-muted-foreground/50 text-center">{rank}</div>
            <div className="flex-1 min-w-0 space-y-1.5">
                <div className="h-4 w-48 bg-muted animate-pulse rounded" />
                <div className="h-3 w-72 bg-muted animate-pulse rounded" />
            </div>
            <div className="hidden sm:block h-3 w-20 bg-muted animate-pulse rounded" />
            <div className="h-3 w-10 bg-muted animate-pulse rounded" />
        </div>
    );
}

export default function ExplorePage() {
    const [category, setCategory] = useState<Category>("popular");
    const activeCategory = categories.find((c) => c.id === category)!;
    const sort = activeCategory.sort;

    const { data, error, isLoading, isValidating, size, setSize } = useSWRInfinite<ExploreResponse>(
        (index) => `/api/explore?sort=${sort}&offset=${index * PAGE_SIZE}`,
        jsonFetcher
    );

    const allRepos = data?.flatMap((page) => page.repositories) ?? [];
    const lastPage = data?.[data.length - 1];
    const hasMore = !isLoading && lastPage != null && lastPage.repositories.length === PAGE_SIZE;

    return (
        <div className="min-h-screen bg-background flex flex-col">
            <TopBar
                breadcrumb={[{ label: "Explore" }]}
                search={{ placeholder: "Search repositories..." }}
                navLinks={[
                    { label: "Explore", href: "/explore", icon: <Compass className="h-[18px] w-[18px]" />, active: true },
                    { label: "Merge Requests", href: "#", icon: <GitMerge className="h-[18px] w-[18px]" /> },
                ]}
                hasNotifications
            />

            <main className="flex-1 flex">
                <aside className="w-64 border-r border-border p-5 hidden lg:block">
                    <div className="space-y-6">
                        <div>
                            <h3 className="text-sm font-medium text-muted-foreground mb-3">Browse</h3>
                            <div className="space-y-1">
                                {categories.map((cat) => {
                                    const Icon = cat.icon;
                                    return (
                                        <button
                                            key={cat.id}
                                            onClick={() => {
                                                setCategory(cat.id);
                                                setSize(1);
                                            }}
                                            className={`w-full flex items-center gap-3 px-3 py-2 text-sm rounded-md transition-colors ${
                                                category === cat.id
                                                    ? "bg-accent/50 text-foreground"
                                                    : "text-muted-foreground hover:text-foreground hover:bg-accent/30"
                                            }`}
                                        >
                                            <Icon className="h-4 w-4" />
                                            {cat.label}
                                        </button>
                                    );
                                })}
                            </div>
                        </div>
                    </div>
                </aside>

                <div className="flex-1 overflow-auto">
                    <div className="p-6">
                        <div className="flex items-center justify-between mb-6">
                            <div className="flex items-center gap-3">
                                <activeCategory.icon className="h-6 w-6 text-orange-500" />
                                <h1 className="text-xl font-semibold">{activeCategory.label} repositories</h1>
                            </div>
                        </div>

                        <div className="border border-border rounded-lg overflow-hidden bg-card/30">
                            {error ? (
                                <div className="flex flex-col items-center justify-center py-20 text-muted-foreground">
                                    <Search className="h-12 w-12 mb-4 opacity-30" />
                                    <p className="text-lg font-medium">Failed to load repositories</p>
                                    <p className="mt-1 text-sm">{error.message}</p>
                                </div>
                            ) : isLoading ? (
                                Array.from({ length: 10 }, (_, i) => <RepoRowSkeleton key={i} rank={i + 1} />)
                            ) : allRepos.length > 0 ? (
                                allRepos.map((repo, index) => <RepoRow key={repo.id} repo={repo} rank={index + 1} />)
                            ) : (
                                <div className="flex flex-col items-center justify-center py-20 text-muted-foreground">
                                    <Search className="h-12 w-12 mb-4 opacity-30" />
                                    <p className="text-lg font-medium">No repositories found</p>
                                </div>
                            )}
                        </div>

                        {hasMore && (
                            <div className="flex justify-center mt-6">
                                <Button variant="secondary" className="gap-2" onClick={() => setSize(size + 1)} disabled={isValidating}>
                                    {isValidating ? (
                                        <>
                                            <Clock className="h-4 w-4 animate-spin" />
                                            Loading...
                                        </>
                                    ) : (
                                        <>
                                            Load more
                                            <ChevronDown className="h-4 w-4" />
                                        </>
                                    )}
                                </Button>
                            </div>
                        )}
                    </div>
                </div>
            </main>
        </div>
    );
}

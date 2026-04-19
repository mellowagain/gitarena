"use client";

import { useState } from "react";
import Link from "next/link";
import { TopBar } from "@/components/top-bar";
import { Button } from "@/components/ui/button";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import {
    Star,
    GitFork,
    TrendingUp,
    Clock,
    Flame,
    Globe,
    ChevronDown,
    Compass,
    GitMerge,
    Sparkles,
    Code,
    Users,
    Search,
} from "lucide-react";

type Repository = {
    id: number;
    org: string;
    name: string;
    description: string;
    stars: number;
    forks: number;
    language: string;
    languageColor: string;
    updatedAt: string;
    topics: string[];
    starsToday?: number;
};

const trendingRepos: Repository[] = [
    {
        id: 1,
        org: "rust-lang",
        name: "rust",
        description: "Empowering everyone to build reliable and efficient software.",
        stars: 89420,
        forks: 11850,
        language: "Rust",
        languageColor: "#dea584",
        updatedAt: "2h",
        topics: ["rust", "compiler", "programming-language"],
        starsToday: 142,
    },
    {
        id: 2,
        org: "denoland",
        name: "deno",
        description: "A modern runtime for JavaScript and TypeScript.",
        stars: 92100,
        forks: 5120,
        language: "Rust",
        languageColor: "#dea584",
        updatedAt: "4h",
        topics: ["javascript", "typescript", "runtime"],
        starsToday: 89,
    },
    {
        id: 3,
        org: "vercel",
        name: "next.js",
        description: "The React Framework for the Web",
        stars: 118500,
        forks: 25400,
        language: "TypeScript",
        languageColor: "#3178c6",
        updatedAt: "1h",
        topics: ["react", "nextjs", "framework"],
        starsToday: 234,
    },
    {
        id: 4,
        org: "tauri-apps",
        name: "tauri",
        description: "Build smaller, faster, and more secure desktop applications with a web frontend.",
        stars: 74200,
        forks: 2180,
        language: "Rust",
        languageColor: "#dea584",
        updatedAt: "6h",
        topics: ["desktop", "rust", "webview"],
        starsToday: 156,
    },
    {
        id: 5,
        org: "astral-sh",
        name: "uv",
        description: "An extremely fast Python package installer and resolver, written in Rust.",
        stars: 28900,
        forks: 820,
        language: "Rust",
        languageColor: "#dea584",
        updatedAt: "3h",
        topics: ["python", "package-manager", "rust"],
        starsToday: 312,
    },
    {
        id: 6,
        org: "oven-sh",
        name: "bun",
        description: "Incredibly fast JavaScript runtime, bundler, test runner, and package manager.",
        stars: 68400,
        forks: 2340,
        language: "Zig",
        languageColor: "#ec915c",
        updatedAt: "5h",
        topics: ["javascript", "runtime", "bundler"],
        starsToday: 98,
    },
    {
        id: 7,
        org: "biomejs",
        name: "biome",
        description: "A toolchain for web projects, aimed to provide functionalities to maintain them.",
        stars: 11200,
        forks: 380,
        language: "Rust",
        languageColor: "#dea584",
        updatedAt: "8h",
        topics: ["linter", "formatter", "rust"],
        starsToday: 67,
    },
    {
        id: 8,
        org: "mellowagain",
        name: "gitarena",
        description: "A lightweight and performant git hosting platform for self-hosting.",
        stars: 2840,
        forks: 124,
        language: "Rust",
        languageColor: "#dea584",
        updatedAt: "1h",
        topics: ["git", "self-hosted", "rust"],
        starsToday: 45,
    },
];

const timeRanges = [
    { id: "today", label: "Today" },
    { id: "week", label: "This week" },
    { id: "month", label: "This month" },
];

const languages = [
    { id: "all", label: "All languages" },
    { id: "rust", label: "Rust" },
    { id: "typescript", label: "TypeScript" },
    { id: "python", label: "Python" },
    { id: "go", label: "Go" },
    { id: "javascript", label: "JavaScript" },
];

const categories = [
    { id: "trending", label: "Trending", icon: Flame },
    { id: "new", label: "New", icon: Sparkles },
    { id: "popular", label: "Popular", icon: Star },
];

function formatNumber(num: number): string {
    if (num >= 1000) {
        return (num / 1000).toFixed(1).replace(/\.0$/, "") + "k";
    }
    return num.toString();
}

function RepoRow({ repo, rank }: { repo: Repository; rank: number }) {
    return (
        <div className="flex items-center gap-4 px-4 py-3 border-b border-border hover:bg-accent/30 transition-colors group">
            <div className="shrink-0 w-8 text-lg font-medium text-muted-foreground/50 text-center">{rank}</div>

            <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                    <Link href={`/${repo.org}/${repo.name}`} className="font-medium hover:underline truncate">
                        <span className="text-muted-foreground">{repo.org}</span>
                        <span className="text-muted-foreground mx-0.5">/</span>
                        <span>{repo.name}</span>
                    </Link>
                </div>
                <p className="text-sm text-muted-foreground truncate mt-0.5">{repo.description}</p>
            </div>

            <div className="hidden sm:flex items-center gap-2 shrink-0 w-28">
                <span className="w-3 h-3 rounded-full shrink-0" style={{ backgroundColor: repo.languageColor }} />
                <span className="text-sm text-muted-foreground truncate">{repo.language}</span>
            </div>

            <div className="flex items-center gap-4 shrink-0 text-sm text-muted-foreground">
                <div className="flex items-center gap-1">
                    <Star className="h-4 w-4" />
                    <span>{formatNumber(repo.stars)}</span>
                </div>
                <div className="hidden sm:flex items-center gap-1">
                    <GitFork className="h-4 w-4" />
                    <span>{formatNumber(repo.forks)}</span>
                </div>
                {repo.starsToday && (
                    <div className="hidden md:flex items-center gap-1 text-foreground">
                        <TrendingUp className="h-4 w-4" />
                        <span>{repo.starsToday}</span>
                    </div>
                )}
            </div>

            <Button variant="secondary" size="sm" className="shrink-0 h-8 gap-1.5 opacity-0 group-hover:opacity-100 transition-opacity">
                <Star className="h-3.5 w-3.5" />
                Star
            </Button>
        </div>
    );
}

export default function ExplorePage() {
    const [searchQuery] = useState("");
    const [timeRange, setTimeRange] = useState("today");
    const [language, setLanguage] = useState("all");
    const [category, setCategory] = useState("trending");

    const filteredRepos = trendingRepos.filter((repo) => {
        if (language !== "all" && repo.language.toLowerCase() !== language) {
            return false;
        }
        if (
            searchQuery &&
            !repo.name.toLowerCase().includes(searchQuery.toLowerCase()) &&
            !repo.description.toLowerCase().includes(searchQuery.toLowerCase())
        ) {
            return false;
        }
        return true;
    });

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
                                            onClick={() => setCategory(cat.id)}
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

                        <div>
                            <h3 className="text-sm font-medium text-muted-foreground mb-3">Resources</h3>
                            <div className="space-y-1">
                                <Link
                                    href="#"
                                    className="flex items-center gap-3 px-3 py-2 text-sm text-muted-foreground hover:text-foreground hover:bg-accent/30 rounded-md transition-colors"
                                >
                                    <Users className="h-4 w-4" />
                                    Organizations
                                </Link>
                                <Link
                                    href="#"
                                    className="flex items-center gap-3 px-3 py-2 text-sm text-muted-foreground hover:text-foreground hover:bg-accent/30 rounded-md transition-colors"
                                >
                                    <Code className="h-4 w-4" />
                                    Topics
                                </Link>
                                <Link
                                    href="#"
                                    className="flex items-center gap-3 px-3 py-2 text-sm text-muted-foreground hover:text-foreground hover:bg-accent/30 rounded-md transition-colors"
                                >
                                    <Globe className="h-4 w-4" />
                                    Collections
                                </Link>
                            </div>
                        </div>

                        <div>
                            <h3 className="text-sm font-medium text-muted-foreground mb-3">Languages</h3>
                            <div className="space-y-1">
                                {languages.slice(1).map((lang) => (
                                    <button
                                        key={lang.id}
                                        onClick={() => setLanguage(language === lang.id ? "all" : lang.id)}
                                        className={`w-full flex items-center gap-3 px-3 py-2 text-sm rounded-md transition-colors ${
                                            language === lang.id
                                                ? "bg-accent/50 text-foreground"
                                                : "text-muted-foreground hover:text-foreground hover:bg-accent/30"
                                        }`}
                                    >
                                        {lang.label}
                                    </button>
                                ))}
                            </div>
                        </div>
                    </div>
                </aside>

                <div className="flex-1 overflow-auto">
                    <div className="p-6">
                        <div className="flex items-center justify-between mb-6">
                            <div className="flex items-center gap-3">
                                <Flame className="h-6 w-6 text-orange-500" />
                                <h1 className="text-xl font-semibold">Trending repositories</h1>
                            </div>

                            <div className="flex items-center gap-2">
                                <DropdownMenu>
                                    <DropdownMenuTrigger asChild>
                                        <Button variant="secondary" size="sm" className="h-9 gap-2">
                                            <Clock className="h-4 w-4" />
                                            {timeRanges.find((t) => t.id === timeRange)?.label}
                                            <ChevronDown className="h-4 w-4 opacity-50" />
                                        </Button>
                                    </DropdownMenuTrigger>
                                    <DropdownMenuContent align="end">
                                        {timeRanges.map((range) => (
                                            <DropdownMenuItem key={range.id} onClick={() => setTimeRange(range.id)}>
                                                {range.label}
                                            </DropdownMenuItem>
                                        ))}
                                    </DropdownMenuContent>
                                </DropdownMenu>

                                <DropdownMenu>
                                    <DropdownMenuTrigger asChild>
                                        <Button variant="secondary" size="sm" className="h-9 gap-2 lg:hidden">
                                            {languages.find((l) => l.id === language)?.label}
                                            <ChevronDown className="h-4 w-4 opacity-50" />
                                        </Button>
                                    </DropdownMenuTrigger>
                                    <DropdownMenuContent align="end">
                                        {languages.map((lang) => (
                                            <DropdownMenuItem key={lang.id} onClick={() => setLanguage(lang.id)}>
                                                {lang.label}
                                            </DropdownMenuItem>
                                        ))}
                                    </DropdownMenuContent>
                                </DropdownMenu>
                            </div>
                        </div>

                        <div className="border border-border rounded-lg overflow-hidden bg-card/30">
                            {filteredRepos.length > 0 ? (
                                filteredRepos.map((repo, index) => <RepoRow key={repo.id} repo={repo} rank={index + 1} />)
                            ) : (
                                <div className="flex flex-col items-center justify-center py-20 text-muted-foreground">
                                    <Search className="h-12 w-12 mb-4 opacity-30" />
                                    <p className="text-lg font-medium">No repositories found</p>
                                    <p className="mt-1">Try adjusting your filters</p>
                                </div>
                            )}
                        </div>

                        {filteredRepos.length > 0 && (
                            <div className="flex justify-center mt-6">
                                <Button variant="secondary" className="gap-2">
                                    Load more
                                    <ChevronDown className="h-4 w-4" />
                                </Button>
                            </div>
                        )}
                    </div>
                </div>
            </main>
        </div>
    );
}

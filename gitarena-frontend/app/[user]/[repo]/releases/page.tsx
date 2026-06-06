"use client";

import { useState } from "react";
import Link from "next/link";
import { useParams } from "next/navigation";
import { TopBar } from "@/components/top-bar";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { ErrorDisplay } from "@/components/error-display";
import { jsonFetcher } from "@/lib/fetchers";
import { uuidToDate } from "@/lib/utils";
import { MarkdownRenderer } from "@/components/markdown-renderer";
import DeviceDetector from "device-detector-js";
import useSWR from "swr";
import { formatDistanceToNow } from "date-fns";
import {
    Tag,
    GitCommit,
    Download,
    Code,
    AlertCircle,
    GitMerge,
    Settings,
    Package,
    Plus,
    Calendar,
    FileArchive,
    ChevronDown,
    ChevronUp,
    Monitor,
    Apple,
    Smartphone,
    HelpCircle,
    Pencil,
    Copy,
    Check,
} from "lucide-react";

type OS = "linux" | "windows" | "macos" | "freebsd" | "openbsd" | "netbsd" | "android" | "ios" | "unknown";
type Arch =
    | "x86_64"
    | "i686"
    | "aarch64"
    | "armv7"
    | "armv6"
    | "riscv64"
    | "loongarch64"
    | "powerpc64"
    | "s390x"
    | "wasm32"
    | "universal"
    | "unknown";
type Libc = "gnu" | "musl" | "msvc" | "mingw" | "bionic" | "unknown";
type AssetKind = "binary" | "installer" | "library" | "source" | "sbom" | "other";

interface Asset {
    id: string;
    name: string;
    size: number;
    hash: string;
    available: boolean;
    downloads: number;
    os: OS | null;
    arch: Arch | null;
    libc: Libc | null;
    kind: AssetKind | null;
}

interface Release {
    id: string;
    tag: string;
    title: string;
    description: string | null;
    preRelease: boolean;
    latest: boolean;
    author: string;
    commit: string | null;
    commitMessage: string | null;
    assets: Asset[];
}

interface PermissionsResponse {
    permissions: {
        view: boolean;
        push: boolean;
        manageIssues: boolean;
        admin: boolean;
    };
}

const OS_LABEL: Record<OS, string> = {
    linux: "Linux",
    windows: "Windows",
    macos: "macOS",
    freebsd: "FreeBSD",
    openbsd: "OpenBSD",
    netbsd: "NetBSD",
    android: "Android",
    ios: "iOS",
    unknown: "Unknown",
};

const OS_ORDER: OS[] = ["linux", "windows", "macos", "freebsd", "openbsd", "netbsd", "android", "ios", "unknown"];

const KIND_ORDER: AssetKind[] = ["binary", "installer", "library", "source", "sbom", "other"];

const KIND_ICON: Record<AssetKind, typeof FileArchive> = {
    binary: FileArchive,
    installer: Package,
    library: FileArchive,
    source: Code,
    sbom: FileArchive,
    other: FileArchive,
};

const KIND_LABEL: Record<AssetKind, string> = {
    binary: "Binary",
    installer: "Installer",
    library: "Library",
    source: "Source",
    sbom: "SBOM",
    other: "Other",
};

const deviceDetector = new DeviceDetector();

function detectCurrentOs(): OS | null {
    if (typeof navigator === "undefined") {
        return null;
    }
    try {
        const result = deviceDetector.parse(navigator.userAgent);
        const name = (result.os?.name ?? "").toLowerCase();
        if (name.includes("windows")) {
            return "windows";
        }
        if (name.includes("mac") || name.includes("ios")) {
            return name.includes("ios") ? "ios" : "macos";
        }
        if (name.includes("android")) {
            return "android";
        }
        if (name.includes("freebsd")) {
            return "freebsd";
        }
        if (name.includes("openbsd")) {
            return "openbsd";
        }
        if (name.includes("netbsd")) {
            return "netbsd";
        }
        if (name.includes("linux") || name.includes("gnu")) {
            return "linux";
        }
    } catch {
        // ignore
    }
    return null;
}

function formatBytes(bytes: number): string {
    if (bytes < 1024) {
        return `${bytes} B`;
    }
    if (bytes < 1024 * 1024) {
        return `${(bytes / 1024).toFixed(1)} KB`;
    }
    if (bytes < 1024 * 1024 * 1024) {
        return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    }
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

function OsIcon({ os, className }: { os: OS; className?: string }) {
    const cls = className ?? "h-3.5 w-3.5";
    if (os === "linux" || os === "freebsd" || os === "openbsd" || os === "netbsd") {
        return <Monitor className={cls} />;
    }
    if (os === "windows") {
        return <Monitor className={cls} />;
    }
    if (os === "macos" || os === "ios") {
        return <Apple className={cls} />;
    }
    if (os === "android") {
        return <Smartphone className={cls} />;
    }
    return <HelpCircle className={cls} />;
}

function Avatar({ name, size = "sm" }: { name: string; size?: "sm" | "xs" }) {
    const dim = size === "sm" ? "h-6 w-6 text-[10px]" : "h-5 w-5 text-[10px]";
    return (
        <div
            className={`${dim} rounded-full bg-secondary border border-border flex items-center justify-center font-semibold text-muted-foreground shrink-0`}
        >
            {name[0].toUpperCase()}
        </div>
    );
}

function AssetRow({ asset, user, repo, releaseId }: { asset: Asset; user: string; repo: string; releaseId: string }) {
    const [copied, setCopied] = useState(false);
    const KindIcon = asset.kind ? KIND_ICON[asset.kind] : FileArchive;

    function copyHash(e: React.MouseEvent) {
        e.preventDefault();
        navigator.clipboard.writeText(asset.hash);
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
    }

    return (
        <div className="flex items-center gap-3 px-3 py-2 rounded-md hover:bg-accent/40 transition-colors group">
            <KindIcon className="h-4 w-4 text-muted-foreground shrink-0 self-start mt-0.5" />
            <div className="flex-1 min-w-0">
                <a
                    href={`/api/repos/${user}/${repo}/releases/${releaseId}/assets/${asset.id}/download`}
                    className="text-sm font-mono text-foreground/90 truncate block hover:underline underline-offset-2"
                >
                    {asset.name}
                </a>
                {asset.hash && (
                    <button
                        onClick={copyHash}
                        title={copied ? "Copied!" : "Click to copy SHA-256"}
                        className="flex items-center gap-1 text-xs font-mono text-muted-foreground/60 hover:text-muted-foreground transition-colors mt-0.5 max-w-full"
                    >
                        <span className="truncate">sha256:{asset.hash}</span>
                        {copied ? (
                            <Check className="h-3 w-3 text-green-500 shrink-0" />
                        ) : (
                            <Copy className="h-3 w-3 shrink-0 opacity-0 group-hover:opacity-100" />
                        )}
                    </button>
                )}
            </div>
            <div className="flex items-center gap-1 shrink-0">
                {asset.arch && asset.arch !== "unknown" && (
                    <span className="px-1.5 py-0.5 text-xs font-mono border border-border rounded bg-secondary text-muted-foreground">
                        {asset.arch}
                    </span>
                )}
                {asset.libc && asset.libc !== "unknown" && (
                    <span className="px-1.5 py-0.5 text-xs font-mono border border-border rounded bg-secondary text-muted-foreground">
                        {asset.libc}
                    </span>
                )}
            </div>
            <span className="text-xs text-muted-foreground shrink-0 w-14 text-right">{formatBytes(asset.size)}</span>
            <span className="text-xs text-muted-foreground shrink-0 w-16 text-right">{asset.downloads.toLocaleString()} dl</span>
            <a
                href={`/api/repos/${user}/${repo}/releases/${releaseId}/assets/${asset.id}/download`}
                className="shrink-0 text-muted-foreground hover:text-foreground transition-colors opacity-0 group-hover:opacity-100"
            >
                <Download className="h-4 w-4" />
            </a>
        </div>
    );
}

function ReleaseCard({ release, user, repo, canPush }: { release: Release; user: string; repo: string; canPush: boolean }) {
    const [assetsOpen, setAssetsOpen] = useState(release.latest);
    const [bodyOpen, setBodyOpen] = useState(release.latest);

    const { data: authorData } = useSWR<{ id: string; username: string }>(`/api/users/by-id/${release.author}`, jsonFetcher);

    const totalDownloads = release.assets.reduce((s, a) => s + a.downloads, 0);
    const osAssets = release.assets.filter((a) => a.os !== null);
    const genericAssets = release.assets.filter((a) => a.os === null);

    const userOs = detectCurrentOs();
    const sortedOsOrder = userOs ? [userOs, ...OS_ORDER.filter((o) => o !== userOs)] : OS_ORDER;

    return (
        <div
            className={`border rounded-lg overflow-hidden transition-colors ${
                release.latest ? "border-primary/40 bg-primary/[0.02]" : "border-border bg-card"
            }`}
        >
            {/* Header */}
            <div className="px-5 py-4 flex items-start gap-4">
                <div
                    className={`mt-0.5 shrink-0 h-9 w-9 rounded-lg flex items-center justify-center ${
                        release.preRelease ? "bg-amber-500/10 text-amber-500" : "bg-primary/10 text-primary"
                    }`}
                >
                    <Package className="h-4.5 w-4.5" />
                </div>

                <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 flex-wrap mb-1">
                        {canPush ? (
                            <Link
                                href={`/${user}/${repo}/releases/${release.id}`}
                                className="font-semibold text-lg hover:underline underline-offset-2"
                            >
                                {release.title}
                            </Link>
                        ) : (
                            <span className="font-semibold text-lg">{release.title}</span>
                        )}
                        {release.latest && (
                            <span className="px-1.5 py-0.5 text-xs font-semibold uppercase tracking-wider rounded-full bg-primary/10 text-primary border border-primary/20">
                                Latest
                            </span>
                        )}
                        {release.preRelease && (
                            <span className="px-1.5 py-0.5 text-xs font-semibold uppercase tracking-wider rounded-full bg-amber-500/10 text-amber-500 border border-amber-500/20">
                                Pre-release
                            </span>
                        )}
                    </div>
                    <div className="flex items-center gap-3 text-sm text-muted-foreground flex-wrap">
                        <Link
                            href={`/${user}/${repo}/tags`}
                            className="flex items-center gap-1 hover:text-foreground transition-colors font-mono"
                        >
                            <Tag className="h-4 w-4" />
                            {release.tag}
                        </Link>
                        {release.commit && (
                            <Link
                                href={`/${user}/${repo}/commit/${release.commit}`}
                                className="flex items-center gap-1 hover:text-foreground transition-colors font-mono"
                            >
                                <GitCommit className="h-4 w-4" />
                                {release.commit.slice(0, 7)}
                            </Link>
                        )}
                        <span className="flex items-center gap-1">
                            <Calendar className="h-4 w-4" />
                            {formatDistanceToNow(uuidToDate(release.id), { addSuffix: true })}
                        </span>
                        {authorData ? (
                            <span className="flex items-center gap-1.5">
                                <Avatar name={authorData.username} size="xs" />
                                {authorData.username}
                            </span>
                        ) : (
                            <Skeleton className="h-4 w-16" />
                        )}
                        {totalDownloads > 0 && (
                            <span className="flex items-center gap-1">
                                <Download className="h-4 w-4" />
                                {totalDownloads.toLocaleString()} downloads
                            </span>
                        )}
                    </div>
                </div>

                {canPush && (
                    <Link
                        href={`/${user}/${repo}/releases/${release.id}`}
                        className="shrink-0 mt-1 p-1.5 text-muted-foreground hover:text-foreground hover:bg-accent/50 rounded-md transition-colors"
                        title="Edit release"
                    >
                        <Pencil className="h-4 w-4" />
                    </Link>
                )}
            </div>

            {/* Release notes */}
            {release.description && (
                <div className="border-t border-border/60">
                    <button
                        onClick={() => setBodyOpen((v) => !v)}
                        className="w-full flex items-center justify-between px-5 py-3 text-sm font-medium text-muted-foreground hover:text-foreground hover:bg-accent/30 transition-colors"
                    >
                        <span>Release notes</span>
                        {bodyOpen ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
                    </button>
                    {bodyOpen && (
                        <div className="px-5 pb-4">
                            <MarkdownRenderer content={release.description} user={user} repo={repo} className="text-sm leading-relaxed" />
                        </div>
                    )}
                </div>
            )}

            {/* Assets */}
            {release.assets.length > 0 && (
                <div className="border-t border-border/60">
                    <button
                        onClick={() => setAssetsOpen((v) => !v)}
                        className="w-full flex items-center justify-between px-5 py-3 text-sm font-medium text-muted-foreground hover:text-foreground hover:bg-accent/30 transition-colors"
                    >
                        <span className="flex items-center gap-1.5">
                            <FileArchive className="h-4 w-4" />
                            Assets
                            <span className="ml-1 px-1.5 py-0.5 rounded-full bg-secondary text-muted-foreground border border-border text-xs">
                                {release.assets.length}
                            </span>
                        </span>
                        {assetsOpen ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
                    </button>

                    {assetsOpen && (
                        <div className="px-5 pb-4 space-y-4">
                            {/* OS sections — flat list, user's OS first */}
                            {sortedOsOrder
                                .filter((os) => osAssets.some((a) => a.os === os))
                                .map((os) => {
                                    const byOs = osAssets.filter((a) => a.os === os);
                                    return (
                                        <div key={os}>
                                            <div className="flex items-center gap-2 mb-2">
                                                <OsIcon os={os} className="h-4 w-4 text-muted-foreground" />
                                                <span className="text-sm font-semibold text-foreground">{OS_LABEL[os]}</span>
                                                {os === userOs && <span className="text-xs text-muted-foreground">(your platform)</span>}
                                                <div className="flex-1 h-px bg-border/60" />
                                            </div>
                                            <div className="space-y-0.5 pl-1">
                                                {byOs.map((asset) => (
                                                    <AssetRow key={asset.id} asset={asset} user={user} repo={repo} releaseId={release.id} />
                                                ))}
                                            </div>
                                        </div>
                                    );
                                })}

                            {/* Kind sections for assets with no OS */}
                            {KIND_ORDER.filter((k) => genericAssets.some((a) => (a.kind ?? "other") === k)).map((kind) => {
                                const byKind = genericAssets.filter((a) => (a.kind ?? "other") === kind);
                                const KindIcon = KIND_ICON[kind];
                                return (
                                    <div key={kind}>
                                        <div className="flex items-center gap-2 mb-2">
                                            <KindIcon className="h-4 w-4 text-muted-foreground" />
                                            <span className="text-sm font-semibold text-foreground">{KIND_LABEL[kind]}</span>
                                            <div className="flex-1 h-px bg-border/60" />
                                        </div>
                                        <div className="space-y-0.5 pl-1">
                                            {byKind.map((asset) => (
                                                <AssetRow key={asset.id} asset={asset} user={user} repo={repo} releaseId={release.id} />
                                            ))}
                                        </div>
                                    </div>
                                );
                            })}
                        </div>
                    )}
                </div>
            )}
        </div>
    );
}

function ReleaseCardSkeleton() {
    return (
        <div className="border border-border bg-card rounded-lg overflow-hidden">
            <div className="px-5 py-4 flex items-start gap-4">
                <Skeleton className="mt-0.5 shrink-0 h-9 w-9 rounded-lg" />
                <div className="flex-1 space-y-2">
                    <Skeleton className="h-5 w-48" />
                    <div className="flex items-center gap-3">
                        <Skeleton className="h-3.5 w-16" />
                        <Skeleton className="h-3.5 w-20" />
                        <Skeleton className="h-3.5 w-24" />
                    </div>
                </div>
            </div>
        </div>
    );
}

export default function ReleasesPage() {
    const params = useParams();
    const user = params.user as string;
    const repo = params.repo as string;

    const [filter, setFilter] = useState<"all" | "stable" | "pre-release">("all");

    const {
        data: releases,
        isLoading,
        error,
    } = useSWR<Release[]>(user && repo ? `/api/repos/${user}/${repo}/releases` : null, jsonFetcher);

    const displayed = releases && filter !== "all" ? releases.filter((r) => (filter === "stable") !== r.preRelease) : releases;

    const { data: permsData } = useSWR<PermissionsResponse>(user && repo ? `/api/repos/${user}/${repo}/permissions` : null, jsonFetcher);
    const canPush = permsData?.permissions.push ?? false;

    const stableCount = releases?.filter((r) => !r.preRelease).length ?? 0;
    const preCount = releases?.filter((r) => r.preRelease).length ?? 0;
    const totalCount = releases?.length ?? 0;

    const navLinks = [
        { label: "Code", href: `/${user}/${repo}`, icon: <Code className="h-[18px] w-[18px]" /> },
        { label: "Issues", href: `/${user}/${repo}/issues`, icon: <AlertCircle className="h-[18px] w-[18px]" /> },
        { label: "Merge Requests", href: `/${user}/${repo}/merge-requests`, icon: <GitMerge className="h-[18px] w-[18px]" /> },
        { label: "Releases", href: `/${user}/${repo}/releases`, icon: <Package className="h-[18px] w-[18px]" />, active: true },
        { label: "Settings", href: `/${user}/${repo}/settings`, icon: <Settings className="h-[18px] w-[18px]" /> },
    ];

    return (
        <div className="min-h-screen bg-background flex flex-col">
            <TopBar
                search={{ placeholder: "Search or jump to…" }}
                navLinks={navLinks}
                breadcrumb={[{ label: user, href: `/${user}` }, { label: repo, href: `/${user}/${repo}` }, { label: "Releases" }]}
            />

            <main className="flex-1 max-w-4xl w-full mx-auto px-6 py-8 space-y-6">
                {/* Page header */}
                <div className="flex items-center justify-between">
                    <div>
                        <h1 className="text-2xl font-semibold">Releases</h1>
                        {!isLoading && releases && (
                            <p className="text-sm text-muted-foreground mt-0.5">
                                {stableCount} stable &middot; {preCount} pre-release
                            </p>
                        )}
                    </div>
                    {canPush && (
                        <Link href={`/${user}/${repo}/releases/new`}>
                            <Button size="sm" className="gap-2">
                                <Plus className="h-4 w-4" />
                                New Release
                            </Button>
                        </Link>
                    )}
                </div>

                {/* Filter tabs */}
                <div className="flex items-center gap-0.5 border-b border-border">
                    {(["all", "stable", "pre-release"] as const).map((f) => {
                        const count = f === "all" ? totalCount : f === "stable" ? stableCount : preCount;
                        return (
                            <button
                                key={f}
                                onClick={() => setFilter(f)}
                                className={`px-4 py-2.5 text-sm capitalize border-b-2 transition-colors -mb-px ${
                                    filter === f
                                        ? "border-foreground text-foreground font-medium"
                                        : "border-transparent text-muted-foreground hover:text-foreground"
                                }`}
                            >
                                {f === "pre-release" ? "Pre-release" : f.charAt(0).toUpperCase() + f.slice(1)}
                                <span
                                    className={`ml-1.5 px-1.5 py-0.5 text-[10px] rounded-full border ${
                                        filter === f
                                            ? "border-border bg-secondary text-foreground"
                                            : "border-transparent bg-secondary text-muted-foreground"
                                    }`}
                                >
                                    {count}
                                </span>
                            </button>
                        );
                    })}

                    <div className="flex-1" />
                </div>

                {/* Release cards */}
                <div className="space-y-4">
                    {isLoading ? (
                        <>
                            <ReleaseCardSkeleton />
                            <ReleaseCardSkeleton />
                            <ReleaseCardSkeleton />
                        </>
                    ) : error ? (
                        <ErrorDisplay failed="releases" error={error} />
                    ) : !displayed || displayed.length === 0 ? (
                        <div className="flex flex-col items-center justify-center py-20 text-center">
                            <Package className="h-10 w-10 text-muted-foreground mb-4" />
                            <p className="font-medium">No releases found</p>
                            <p className="text-sm text-muted-foreground mt-1">
                                {filter !== "all" ? "Try a different filter." : "Create the first release to get started."}
                            </p>
                        </div>
                    ) : (
                        displayed.map((release) => (
                            <ReleaseCard key={release.id} release={release} user={user} repo={repo} canPush={canPush} />
                        ))
                    )}
                </div>
            </main>
        </div>
    );
}

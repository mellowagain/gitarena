"use client";

import { useRef, useState } from "react";
import Link from "next/link";
import { useParams, useRouter } from "next/navigation";
import useSWR from "swr";
import useSWRMutation from "swr/mutation";
import { TopBar } from "@/components/top-bar";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { ErrorDisplay } from "@/components/error-display";
import { MarkdownRenderer } from "@/components/markdown-renderer";
import { jsonFetcher, patchJsonFetcher } from "@/lib/fetchers";
import { toast } from "sonner";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import {
    AlertCircle,
    ArrowLeft,
    Code,
    Eye,
    FileArchive,
    GitMerge,
    Loader2,
    Package,
    Settings,
    Tag,
    Trash2,
    Upload,
    X,
    CheckCircle2,
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
    assets: Asset[];
}

interface UpdateReleaseRequest {
    title?: string;
    description?: string;
    preRelease?: boolean;
}

interface CreateAssetRequest {
    name: string;
    size: number;
    os: OS | null;
    arch: Arch | null;
    libc: Libc | null;
    kind: AssetKind | null;
}

interface CreateAssetResponse {
    assetId: string;
    uploadUrl: string;
}

const OS_OPTIONS: { value: OS | ""; label: string }[] = [
    { value: "", label: "— None —" },
    { value: "linux", label: "Linux" },
    { value: "windows", label: "Windows" },
    { value: "macos", label: "macOS" },
    { value: "freebsd", label: "FreeBSD" },
    { value: "openbsd", label: "OpenBSD" },
    { value: "netbsd", label: "NetBSD" },
    { value: "android", label: "Android" },
    { value: "ios", label: "iOS" },
    { value: "unknown", label: "Unknown" },
];

const ARCH_OPTIONS: { value: Arch | ""; label: string }[] = [
    { value: "", label: "— None —" },
    { value: "x86_64", label: "x86_64" },
    { value: "i686", label: "i686" },
    { value: "aarch64", label: "aarch64" },
    { value: "armv7", label: "armv7" },
    { value: "armv6", label: "armv6" },
    { value: "riscv64", label: "riscv64" },
    { value: "loongarch64", label: "loongarch64" },
    { value: "powerpc64", label: "powerpc64" },
    { value: "s390x", label: "s390x" },
    { value: "wasm32", label: "wasm32" },
    { value: "universal", label: "universal" },
    { value: "unknown", label: "Unknown" },
];

const LIBC_OPTIONS: { value: Libc | ""; label: string }[] = [
    { value: "", label: "— None —" },
    { value: "gnu", label: "glibc (gnu)" },
    { value: "musl", label: "musl" },
    { value: "msvc", label: "MSVC" },
    { value: "mingw", label: "MinGW" },
    { value: "bionic", label: "Bionic (Android)" },
    { value: "unknown", label: "Unknown" },
];

const KIND_OPTIONS: { value: AssetKind | ""; label: string }[] = [
    { value: "", label: "— None —" },
    { value: "binary", label: "Binary" },
    { value: "installer", label: "Installer" },
    { value: "library", label: "Library" },
    { value: "source", label: "Source archive" },
    { value: "sbom", label: "SBOM" },
    { value: "other", label: "Other" },
];

function formatBytes(bytes: number): string {
    if (bytes < 1024) {
        return `${bytes} B`;
    }
    if (bytes < 1024 * 1024) {
        return `${(bytes / 1024).toFixed(1)} KB`;
    }
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

type UploadState =
    | { status: "idle" }
    | { status: "selected"; file: File; name: string; os: OS | ""; arch: Arch | ""; libc: Libc | ""; kind: AssetKind | "" }
    | { status: "uploading"; progress: number }
    | { status: "confirming" }
    | { status: "done" };

function detectFromFilename(filename: string): { os: OS | ""; arch: Arch | ""; libc: Libc | ""; kind: AssetKind | "" } {
    const lower = filename.toLowerCase().replace(/\.(tar\.gz|tar\.xz|tar\.bz2|tar\.zst|zip|exe|msi|dmg|pkg|deb|rpm|appimage)$/, "");

    let os: OS | "" = "";
    let arch: Arch | "" = "";
    let libc: Libc | "" = "";
    let kind: AssetKind | "" = "";

    if (lower.includes("linux")) {
        os = "linux";
    } else if (lower.includes("windows")) {
        os = "windows";
    } else if (lower.includes("darwin") || lower.includes("apple") || lower.includes("macos")) {
        os = "macos";
    } else if (lower.includes("freebsd")) {
        os = "freebsd";
    } else if (lower.includes("openbsd")) {
        os = "openbsd";
    } else if (lower.includes("netbsd")) {
        os = "netbsd";
    } else if (lower.includes("android")) {
        os = "android";
    } else if (/[-_.]ios[-_.]/.test(lower) || lower.endsWith("-ios") || lower.includes("-ios-")) {
        os = "ios";
    }

    if (lower.includes("x86_64") || lower.includes("x86-64") || lower.includes("amd64")) {
        arch = "x86_64";
    } else if (lower.includes("aarch64") || lower.includes("arm64")) {
        arch = "aarch64";
    } else if (lower.includes("armv7")) {
        arch = "armv7";
    } else if (lower.includes("armv6")) {
        arch = "armv6";
    } else if (lower.includes("i686") || lower.includes("i386")) {
        arch = "i686";
    } else if (lower.includes("riscv64")) {
        arch = "riscv64";
    } else if (lower.includes("loongarch64")) {
        arch = "loongarch64";
    } else if (lower.includes("powerpc64") || lower.includes("ppc64")) {
        arch = "powerpc64";
    } else if (lower.includes("s390x")) {
        arch = "s390x";
    } else if (lower.includes("wasm32") || lower.includes("wasm")) {
        arch = "wasm32";
    } else if (lower.includes("universal")) {
        arch = "universal";
    }

    if (lower.includes("musl")) {
        libc = "musl";
    } else if (lower.includes("msvc")) {
        libc = "msvc";
    } else if (lower.includes("mingw")) {
        libc = "mingw";
    } else if (lower.includes("bionic")) {
        libc = "bionic";
    } else if (/-(gnueabihf|gnueabi|gnu)$/.test(lower) || lower.includes("-gnu-") || lower.endsWith("-glibc")) {
        libc = "gnu";
    }

    if (filename.toLowerCase().includes(".sbom")) {
        kind = "sbom";
    } else if (/\.(exe|msi)$/i.test(filename) || lower.includes("installer") || lower.includes("setup")) {
        kind = "installer";
    } else if (/\.(so|dll|dylib|lib|a)$/i.test(filename) || lower.includes("lib-") || lower.includes("-lib")) {
        kind = "library";
    } else if (os || arch) {
        kind = "binary";
    } else if (/\.(tar\.gz|tar\.xz|tar\.bz2|tar\.zst|zip)$/i.test(filename)) {
        kind = "source";
    }

    return { os, arch, libc, kind };
}

const SENTINEL = "_none";

function SelectField({
    label,
    value,
    onChange,
    options,
}: {
    label: string;
    value: string;
    onChange: (v: string) => void;
    options: { value: string; label: string }[];
}) {
    return (
        <div>
            <label className="text-xs font-medium text-muted-foreground block mb-1">{label}</label>
            <Select value={value || SENTINEL} onValueChange={(v) => onChange(v === SENTINEL ? "" : v)}>
                <SelectTrigger size="sm" className="w-full">
                    <SelectValue />
                </SelectTrigger>
                <SelectContent>
                    {options.map((opt) => (
                        <SelectItem key={opt.value || SENTINEL} value={opt.value || SENTINEL}>
                            {opt.label}
                        </SelectItem>
                    ))}
                </SelectContent>
            </Select>
        </div>
    );
}

export default function EditReleasePage() {
    const params = useParams();
    const router = useRouter();
    const user = params.user as string;
    const repo = params.repo as string;
    const releaseId = params.id as string;

    const {
        data: release,
        isLoading,
        error,
        mutate,
    } = useSWR<Release>(user && repo && releaseId ? `/api/repos/${user}/${repo}/releases/${releaseId}` : null, jsonFetcher);

    const [title, setTitle] = useState("");
    const [description, setDescription] = useState("");
    const [preRelease, setPreRelease] = useState(false);
    const [preview, setPreview] = useState(false);
    const [seeded, setSeeded] = useState(false);

    if (release && !seeded) {
        setTitle(release.title);
        setDescription(release.description ?? "");
        setPreRelease(release.preRelease);
        setSeeded(true);
    }

    const { trigger: saveRelease, isMutating: isSaving } = useSWRMutation<Release, Error, string, UpdateReleaseRequest>(
        `/api/repos/${user}/${repo}/releases/${releaseId}`,
        patchJsonFetcher
    );

    async function handleSave() {
        if (!title.trim()) {
            return;
        }
        try {
            await saveRelease({
                title: title.trim(),
                description: description.trim() || undefined,
                preRelease,
            });
            toast.success("Release updated");
            await mutate();
        } catch (err) {
            toast.error(err instanceof Error ? err.message : "Failed to update release");
        }
    }

    const [deletingAssetId, setDeletingAssetId] = useState<string | null>(null);

    async function handleDeleteAsset(assetId: string) {
        setDeletingAssetId(assetId);
        try {
            const res = await fetch(`/api/repos/${user}/${repo}/releases/${releaseId}/assets/${assetId}`, {
                method: "DELETE",
            });
            if (!res.ok) {
                const body = await res.json().catch(() => ({ error: res.statusText }));
                throw new Error((body as { error?: string }).error ?? res.statusText);
            }
            toast.success("Asset deleted");
            await mutate();
        } catch (err) {
            toast.error(err instanceof Error ? err.message : "Failed to delete asset");
        } finally {
            setDeletingAssetId(null);
        }
    }

    const fileInputRef = useRef<HTMLInputElement>(null);
    const [uploadState, setUploadState] = useState<UploadState>({ status: "idle" });

    function handleFileSelected(e: React.ChangeEvent<HTMLInputElement>) {
        const file = e.target.files?.[0];
        if (!file) {
            return;
        }
        const detected = detectFromFilename(file.name);
        setUploadState({
            status: "selected",
            file,
            name: file.name,
            os: detected.os,
            arch: detected.arch,
            libc: detected.libc,
            kind: detected.kind,
        });
        // Reset the input so the same file can be re-selected
        e.target.value = "";
    }

    async function handleUpload() {
        if (uploadState.status !== "selected") {
            return;
        }

        const { file, name, os, arch, libc, kind } = uploadState;

        const body: CreateAssetRequest = {
            name: name.trim() || file.name,
            size: file.size,
            os: os || null,
            arch: arch || null,
            libc: libc || null,
            kind: kind || null,
        };

        try {
            // Step 1: Create asset record + get presigned PUT URL
            const createRes = await fetch(`/api/repos/${user}/${repo}/releases/${releaseId}/assets`, {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify(body),
            });

            if (!createRes.ok) {
                const errBody = await createRes.json().catch(() => ({ error: createRes.statusText }));
                throw new Error((errBody as { error?: string }).error ?? createRes.statusText);
            }

            const { assetId, uploadUrl } = (await createRes.json()) as CreateAssetResponse;

            // Step 2: Upload file directly to S3 presigned URL
            setUploadState({ status: "uploading", progress: 0 });

            await new Promise<void>((resolve, reject) => {
                const xhr = new XMLHttpRequest();
                xhr.open("PUT", uploadUrl);
                xhr.setRequestHeader("Content-Type", "application/octet-stream");
                xhr.upload.onprogress = (e) => {
                    if (e.lengthComputable) {
                        setUploadState({ status: "uploading", progress: Math.round((e.loaded / e.total) * 100) });
                    }
                };
                xhr.onload = () =>
                    xhr.status >= 200 && xhr.status < 300 ? resolve() : reject(new Error(`S3 upload failed: ${xhr.status}`));
                xhr.onerror = () => reject(new Error("Upload failed"));
                xhr.send(file);
            });

            // Step 3: Confirm upload — server computes SHA-256 and marks available
            setUploadState({ status: "confirming" });
            const confirmRes = await fetch(`/api/repos/${user}/${repo}/releases/${releaseId}/assets/${assetId}/confirm`, { method: "PUT" });

            if (!confirmRes.ok) {
                const errBody = await confirmRes.json().catch(() => ({ error: confirmRes.statusText }));
                throw new Error((errBody as { error?: string }).error ?? confirmRes.statusText);
            }

            toast.success("Asset uploaded");
            setUploadState({ status: "idle" });
            if (fileInputRef.current) {
                fileInputRef.current.value = "";
            }
            await mutate();
        } catch (err) {
            toast.error(err instanceof Error ? err.message : "Upload failed");
            setUploadState({ status: "idle" });
        }
    }

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
                breadcrumb={[
                    { label: user, href: `/${user}` },
                    { label: repo, href: `/${user}/${repo}` },
                    { label: "Releases", href: `/${user}/${repo}/releases` },
                    { label: isLoading ? "…" : (release?.title ?? "Edit Release") },
                ]}
                navLinks={navLinks}
                search={{ placeholder: "Search or jump to…" }}
            />

            <main className="flex-1 max-w-4xl w-full mx-auto px-6 py-8 space-y-8">
                <Link
                    href={`/${user}/${repo}/releases`}
                    className="inline-flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
                >
                    <ArrowLeft className="h-4 w-4" />
                    Back to releases
                </Link>

                {isLoading ? (
                    <div className="space-y-4">
                        <Skeleton className="h-8 w-64" />
                        <Skeleton className="h-10 w-full" />
                        <Skeleton className="h-40 w-full" />
                    </div>
                ) : error || !release ? (
                    <ErrorDisplay failed="release" error={error} />
                ) : (
                    <>
                        <section className="space-y-6">
                            <div className="flex items-center gap-3">
                                <div className="h-9 w-9 rounded-lg bg-primary/10 flex items-center justify-center shrink-0">
                                    <Package className="h-4.5 w-4.5 text-primary" />
                                </div>
                                <div>
                                    <h1 className="text-xl font-semibold">Edit release</h1>
                                    <div className="flex items-center gap-1.5 text-sm text-muted-foreground mt-0.5">
                                        <Tag className="h-3.5 w-3.5" />
                                        <span className="font-mono">{release.tag}</span>
                                    </div>
                                </div>
                            </div>

                            <div>
                                <label className="text-sm font-medium block mb-1.5">
                                    Title <span className="text-red-500">*</span>
                                </label>
                                <input
                                    type="text"
                                    value={title}
                                    onChange={(e) => setTitle(e.target.value)}
                                    className="w-full h-10 px-3 bg-card border border-border rounded-md text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
                                />
                            </div>

                            <div>
                                <label className="text-sm font-medium block mb-1.5">Release notes</label>
                                <div className="border border-border rounded-md overflow-hidden focus-within:ring-1 focus-within:ring-ring/50">
                                    <div className="flex items-center border-b border-border">
                                        <button
                                            type="button"
                                            onClick={() => setPreview(false)}
                                            className={`px-3 py-1.5 text-xs font-medium transition-colors ${
                                                !preview
                                                    ? "text-foreground border-b-2 border-foreground -mb-px"
                                                    : "text-muted-foreground hover:text-foreground"
                                            }`}
                                        >
                                            Write
                                        </button>
                                        <button
                                            type="button"
                                            onClick={() => setPreview(true)}
                                            className={`px-3 py-1.5 text-xs font-medium transition-colors flex items-center gap-1.5 ${
                                                preview
                                                    ? "text-foreground border-b-2 border-foreground -mb-px"
                                                    : "text-muted-foreground hover:text-foreground"
                                            }`}
                                        >
                                            <Eye className="h-3 w-3" />
                                            Preview
                                        </button>
                                    </div>
                                    {preview ? (
                                        <div className="px-3 py-2 min-h-[160px]">
                                            {description.trim() ? (
                                                <MarkdownRenderer
                                                    content={description}
                                                    user={user}
                                                    repo={repo}
                                                    className="space-y-4 text-sm leading-relaxed"
                                                />
                                            ) : (
                                                <span className="text-muted-foreground italic">Nothing to preview.</span>
                                            )}
                                        </div>
                                    ) : (
                                        <textarea
                                            value={description}
                                            onChange={(e) => setDescription(e.target.value)}
                                            placeholder="Describe this release… Markdown is supported."
                                            rows={6}
                                            className="w-full px-3 py-2 bg-transparent text-sm resize-none focus:outline-none"
                                        />
                                    )}
                                    <div className="flex items-center px-3 py-2 border-t border-border bg-card/50">
                                        <span className="text-xs text-muted-foreground font-mono">Markdown</span>
                                    </div>
                                </div>
                            </div>

                            <div className="flex items-start gap-3 p-4 border border-border rounded-md bg-card">
                                <input
                                    id="pre-release"
                                    type="checkbox"
                                    checked={preRelease}
                                    onChange={(e) => setPreRelease(e.target.checked)}
                                    className="mt-0.5 h-4 w-4 rounded border-border accent-primary cursor-pointer"
                                />
                                <div>
                                    <label htmlFor="pre-release" className="text-sm font-medium cursor-pointer select-none">
                                        Set as a pre-release
                                    </label>
                                    <p className="text-xs text-muted-foreground mt-0.5">
                                        Pre-releases are not considered stable and will not be shown as the latest release.
                                    </p>
                                </div>
                            </div>

                            <div className="flex items-center justify-between">
                                <Link href={`/${user}/${repo}/releases`}>
                                    <Button variant="outline">Cancel</Button>
                                </Link>
                                <Button onClick={handleSave} disabled={!title.trim() || isSaving}>
                                    {isSaving && <Loader2 className="h-3.5 w-3.5 mr-1.5 animate-spin" />}
                                    Save changes
                                </Button>
                            </div>
                        </section>

                        <section className="border-t border-border pt-8 space-y-5">
                            <h2 className="text-base font-semibold">Assets</h2>

                            {/* Existing assets */}
                            {release.assets.length === 0 ? (
                                <p className="text-sm text-muted-foreground">No assets attached to this release yet.</p>
                            ) : (
                                <div className="border border-border rounded-md divide-y divide-border overflow-hidden">
                                    {release.assets.map((asset) => (
                                        <div key={asset.id} className="flex items-center gap-3 px-4 py-3 bg-card">
                                            <FileArchive className="h-4 w-4 text-muted-foreground shrink-0" />
                                            <span className="flex-1 min-w-0 font-mono text-sm truncate">{asset.name}</span>
                                            <span className="text-xs text-muted-foreground shrink-0">{formatBytes(asset.size)}</span>
                                            {asset.os && (
                                                <span className="text-xs px-1.5 py-0.5 border border-border rounded bg-secondary text-muted-foreground shrink-0">
                                                    {asset.os}
                                                </span>
                                            )}
                                            {asset.arch && (
                                                <span className="text-xs px-1.5 py-0.5 border border-border rounded bg-secondary text-muted-foreground shrink-0">
                                                    {asset.arch}
                                                </span>
                                            )}
                                            <button
                                                onClick={() => handleDeleteAsset(asset.id)}
                                                disabled={deletingAssetId === asset.id}
                                                className="shrink-0 text-muted-foreground hover:text-destructive transition-colors disabled:opacity-50"
                                            >
                                                {deletingAssetId === asset.id ? (
                                                    <Loader2 className="h-4 w-4 animate-spin" />
                                                ) : (
                                                    <Trash2 className="h-4 w-4" />
                                                )}
                                            </button>
                                        </div>
                                    ))}
                                </div>
                            )}

                            {/* Upload new asset */}
                            <div className="border border-dashed border-border rounded-md overflow-hidden">
                                <div className="p-4">
                                    <h3 className="text-sm font-medium mb-3 flex items-center gap-2">
                                        <Upload className="h-4 w-4" />
                                        Upload new asset
                                    </h3>

                                    {uploadState.status === "idle" && (
                                        <>
                                            <input ref={fileInputRef} type="file" className="hidden" onChange={handleFileSelected} />
                                            <button
                                                onClick={() => fileInputRef.current?.click()}
                                                className="w-full flex flex-col items-center justify-center gap-2 py-6 border border-dashed border-border rounded-md text-sm text-muted-foreground hover:text-foreground hover:border-foreground/40 transition-colors"
                                            >
                                                <Upload className="h-5 w-5" />
                                                Click to select a file
                                            </button>
                                        </>
                                    )}

                                    {uploadState.status === "selected" && (
                                        <div className="space-y-4">
                                            <div className="flex items-center gap-3 p-3 bg-secondary/50 rounded-md border border-border">
                                                <FileArchive className="h-4 w-4 text-muted-foreground shrink-0" />
                                                <div className="flex-1 min-w-0">
                                                    <div className="text-sm font-mono truncate">{uploadState.file.name}</div>
                                                    <div className="text-xs text-muted-foreground">
                                                        {formatBytes(uploadState.file.size)}
                                                    </div>
                                                </div>
                                                <button
                                                    onClick={() => setUploadState({ status: "idle" })}
                                                    className="text-muted-foreground hover:text-foreground transition-colors"
                                                >
                                                    <X className="h-4 w-4" />
                                                </button>
                                            </div>

                                            <div>
                                                <label className="text-xs font-medium text-muted-foreground block mb-1">Name</label>
                                                <input
                                                    type="text"
                                                    value={uploadState.name}
                                                    onChange={(e) =>
                                                        setUploadState((s) =>
                                                            s.status === "selected" ? { ...s, name: e.target.value } : s
                                                        )
                                                    }
                                                    className="w-full h-8 px-2 text-sm font-mono bg-card border border-border rounded-md focus:outline-none focus:ring-1 focus:ring-ring"
                                                />
                                            </div>

                                            <div className="grid grid-cols-2 gap-3">
                                                <SelectField
                                                    label="OS"
                                                    value={uploadState.os}
                                                    onChange={(v) =>
                                                        setUploadState((s) => (s.status === "selected" ? { ...s, os: v as OS | "" } : s))
                                                    }
                                                    options={OS_OPTIONS}
                                                />
                                                <SelectField
                                                    label="Architecture"
                                                    value={uploadState.arch}
                                                    onChange={(v) =>
                                                        setUploadState((s) =>
                                                            s.status === "selected" ? { ...s, arch: v as Arch | "" } : s
                                                        )
                                                    }
                                                    options={ARCH_OPTIONS}
                                                />
                                                <SelectField
                                                    label="libc"
                                                    value={uploadState.libc}
                                                    onChange={(v) =>
                                                        setUploadState((s) =>
                                                            s.status === "selected" ? { ...s, libc: v as Libc | "" } : s
                                                        )
                                                    }
                                                    options={LIBC_OPTIONS}
                                                />
                                                <SelectField
                                                    label="Kind"
                                                    value={uploadState.kind}
                                                    onChange={(v) =>
                                                        setUploadState((s) =>
                                                            s.status === "selected" ? { ...s, kind: v as AssetKind | "" } : s
                                                        )
                                                    }
                                                    options={KIND_OPTIONS}
                                                />
                                            </div>

                                            <div className="flex items-center justify-end gap-2">
                                                <Button variant="outline" size="sm" onClick={() => setUploadState({ status: "idle" })}>
                                                    Cancel
                                                </Button>
                                                <Button size="sm" onClick={handleUpload}>
                                                    <Upload className="h-3.5 w-3.5 mr-1.5" />
                                                    Upload
                                                </Button>
                                            </div>
                                        </div>
                                    )}

                                    {uploadState.status === "uploading" && (
                                        <div className="space-y-2 py-4">
                                            <div className="flex items-center justify-between text-sm mb-1">
                                                <span className="text-muted-foreground">Uploading…</span>
                                                <span className="text-foreground font-medium">{uploadState.progress}%</span>
                                            </div>
                                            <div className="w-full h-1.5 bg-secondary rounded-full overflow-hidden">
                                                <div
                                                    className="h-full bg-primary rounded-full transition-all duration-150"
                                                    style={{ width: `${uploadState.progress}%` }}
                                                />
                                            </div>
                                        </div>
                                    )}

                                    {uploadState.status === "confirming" && (
                                        <div className="flex items-center gap-2 py-4 text-sm text-muted-foreground">
                                            <Loader2 className="h-4 w-4 animate-spin" />
                                            Verifying and computing checksum…
                                        </div>
                                    )}

                                    {uploadState.status === "done" && (
                                        <div className="flex items-center gap-2 py-4 text-sm text-green-500">
                                            <CheckCircle2 className="h-4 w-4" />
                                            Upload complete
                                        </div>
                                    )}
                                </div>
                            </div>

                            <div className="flex justify-end">
                                <Link href={`/${user}/${repo}/releases`}>
                                    <Button variant="ghost" size="sm">
                                        Done editing
                                    </Button>
                                </Link>
                            </div>
                        </section>
                    </>
                )}
            </main>
        </div>
    );
}

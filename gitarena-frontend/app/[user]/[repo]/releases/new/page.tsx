"use client";

import { useRef, useState } from "react";
import Link from "next/link";
import { useParams, useRouter } from "next/navigation";
import { TopBar } from "@/components/top-bar";
import { Button } from "@/components/ui/button";
import { MarkdownRenderer } from "@/components/markdown-renderer";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { toast } from "sonner";
import { AlertCircle, ArrowLeft, Code, Eye, FileArchive, GitMerge, Loader2, Package, Plus, Settings, Tag, Upload, X } from "lucide-react";

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

interface QueuedAsset {
    localId: string;
    file: File;
    name: string;
    os: OS | "";
    arch: Arch | "";
    libc: Libc | "";
    kind: AssetKind | "";
}

interface CreateReleaseRequest {
    tag: string;
    title: string;
    description?: string;
    preRelease: boolean;
}

interface CreateReleaseResponse {
    id: string;
    tag: string;
    title: string;
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

const SENTINEL = "_none";

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

type PublishStatus =
    | { status: "idle" }
    | { status: "creating" }
    | { status: "uploading"; index: number; total: number; progress: number; name: string }
    | { status: "confirming"; index: number; total: number; name: string };

export default function NewReleasePage() {
    const params = useParams();
    const router = useRouter();
    const user = params.user as string;
    const repo = params.repo as string;

    // Release form
    const [tag, setTag] = useState("");
    const [title, setTitle] = useState("");
    const [description, setDescription] = useState("");
    const [preRelease, setPreRelease] = useState(false);
    const [preview, setPreview] = useState(false);

    // Asset queue
    const [assets, setAssets] = useState<QueuedAsset[]>([]);
    const fileInputRef = useRef<HTMLInputElement>(null);

    // Publish state
    const [publishStatus, setPublishStatus] = useState<PublishStatus>({ status: "idle" });

    const isPublishing = publishStatus.status !== "idle";

    function handleFileSelected(e: React.ChangeEvent<HTMLInputElement>) {
        const files = Array.from(e.target.files ?? []);
        if (files.length === 0) {
            return;
        }
        const newAssets: QueuedAsset[] = files.map((file) => {
            const detected = detectFromFilename(file.name);
            return {
                localId: `${Date.now()}-${Math.random()}`,
                file,
                name: file.name,
                ...detected,
            };
        });
        setAssets((prev) => [...prev, ...newAssets]);
        e.target.value = "";
    }

    function updateAsset(localId: string, patch: Partial<QueuedAsset>) {
        setAssets((prev) => prev.map((a) => (a.localId === localId ? { ...a, ...patch } : a)));
    }

    function removeAsset(localId: string) {
        setAssets((prev) => prev.filter((a) => a.localId !== localId));
    }

    async function handlePublish() {
        if (!tag.trim() || !title.trim()) {
            return;
        }

        // Step 1: Create release
        setPublishStatus({ status: "creating" });
        let releaseId: string;
        try {
            const createBody: CreateReleaseRequest = {
                tag: tag.trim(),
                title: title.trim(),
                ...(description.trim() ? { description: description.trim() } : {}),
                preRelease,
            };
            const res = await fetch(`/api/repos/${user}/${repo}/releases`, {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify(createBody),
            });
            if (!res.ok) {
                const body = await res.json().catch(() => ({ error: res.statusText }));
                throw new Error((body as { error?: string }).error ?? res.statusText);
            }
            const data = (await res.json()) as CreateReleaseResponse;
            releaseId = data.id;
        } catch (err) {
            toast.error(err instanceof Error ? err.message : "Failed to create release");
            setPublishStatus({ status: "idle" });
            return;
        }

        // Step 2: Upload each queued asset
        for (let i = 0; i < assets.length; i++) {
            const asset = assets[i];

            try {
                // Create asset record → get presigned PUT URL
                const assetBody: CreateAssetRequest = {
                    name: asset.name.trim() || asset.file.name,
                    size: asset.file.size,
                    os: asset.os || null,
                    arch: asset.arch || null,
                    libc: asset.libc || null,
                    kind: asset.kind || null,
                };
                const createRes = await fetch(`/api/repos/${user}/${repo}/releases/${releaseId}/assets`, {
                    method: "POST",
                    headers: { "Content-Type": "application/json" },
                    body: JSON.stringify(assetBody),
                });
                if (!createRes.ok) {
                    const errBody = await createRes.json().catch(() => ({ error: createRes.statusText }));
                    throw new Error((errBody as { error?: string }).error ?? createRes.statusText);
                }
                const { assetId, uploadUrl } = (await createRes.json()) as CreateAssetResponse;

                // PUT to S3 presigned URL
                setPublishStatus({ status: "uploading", index: i, total: assets.length, progress: 0, name: asset.name });
                await new Promise<void>((resolve, reject) => {
                    const xhr = new XMLHttpRequest();
                    xhr.open("PUT", uploadUrl);
                    xhr.setRequestHeader("Content-Type", "application/octet-stream");
                    xhr.upload.onprogress = (ev) => {
                        if (ev.lengthComputable) {
                            setPublishStatus({
                                status: "uploading",
                                index: i,
                                total: assets.length,
                                progress: Math.round((ev.loaded / ev.total) * 100),
                                name: asset.name,
                            });
                        }
                    };
                    xhr.onload = () =>
                        xhr.status >= 200 && xhr.status < 300 ? resolve() : reject(new Error(`Upload failed: ${xhr.status}`));
                    xhr.onerror = () => reject(new Error("Upload failed"));
                    xhr.send(asset.file);
                });

                // Confirm — server computes SHA-256 and marks available
                setPublishStatus({ status: "confirming", index: i, total: assets.length, name: asset.name });
                const confirmRes = await fetch(`/api/repos/${user}/${repo}/releases/${releaseId}/assets/${assetId}/confirm`, {
                    method: "PUT",
                });
                if (!confirmRes.ok) {
                    const errBody = await confirmRes.json().catch(() => ({ error: confirmRes.statusText }));
                    throw new Error((errBody as { error?: string }).error ?? confirmRes.statusText);
                }
            } catch (err) {
                toast.error(`Failed to upload "${asset.name}": ${err instanceof Error ? err.message : "Unknown error"}`);
                // Release was already created — redirect to edit page so user can retry
                router.push(`/${user}/${repo}/releases/${releaseId}`);
                return;
            }
        }

        router.push(`/${user}/${repo}/releases`);
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
                    { label: "New Release" },
                ]}
                navLinks={navLinks}
                search={{ placeholder: "Search or jump to…" }}
            />

            <main className="flex-1 max-w-4xl w-full mx-auto px-6 py-8">
                <Link
                    href={`/${user}/${repo}/releases`}
                    className="inline-flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors mb-6"
                >
                    <ArrowLeft className="h-4 w-4" />
                    Back to releases
                </Link>

                <h1 className="text-2xl font-semibold mb-8">New Release</h1>

                <div className="space-y-6">
                    {/* Tag */}
                    <div>
                        <label className="text-sm font-medium block mb-1.5">
                            Tag <span className="text-red-500">*</span>
                        </label>
                        <div className="relative">
                            <Tag className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground pointer-events-none" />
                            <input
                                type="text"
                                value={tag}
                                onChange={(e) => setTag(e.target.value)}
                                placeholder="v1.0.0"
                                disabled={isPublishing}
                                className="w-full h-10 pl-9 pr-3 bg-card border border-border rounded-md text-sm font-mono placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring disabled:opacity-50"
                            />
                        </div>
                        <p className="mt-1.5 text-xs text-muted-foreground">
                            If this tag does not exist in the repository, an annotated git tag will be created referencing the latest commit
                            on the default branch.
                        </p>
                    </div>

                    {/* Title */}
                    <div>
                        <label className="text-sm font-medium block mb-1.5">
                            Title <span className="text-red-500">*</span>
                        </label>
                        <input
                            type="text"
                            value={title}
                            onChange={(e) => setTitle(e.target.value)}
                            placeholder="Release title…"
                            disabled={isPublishing}
                            className="w-full h-10 px-3 bg-card border border-border rounded-md text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring disabled:opacity-50"
                        />
                    </div>

                    {/* Description */}
                    <div>
                        <label className="text-sm font-medium block mb-1.5">Release notes</label>
                        <div className="border border-border rounded-md overflow-hidden focus-within:ring-1 focus-within:ring-ring/50">
                            <div className="flex items-center border-b border-border">
                                <button
                                    type="button"
                                    onClick={() => setPreview(false)}
                                    disabled={isPublishing}
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
                                    disabled={isPublishing}
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
                                <div className="px-3 py-2 min-h-[200px]">
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
                                    disabled={isPublishing}
                                    rows={8}
                                    className="w-full px-3 py-2 bg-transparent text-sm resize-none focus:outline-none disabled:opacity-50"
                                />
                            )}
                            <div className="flex items-center px-3 py-2 border-t border-border bg-card/50">
                                <span className="text-xs text-muted-foreground font-mono">Markdown</span>
                            </div>
                        </div>
                    </div>

                    {/* Pre-release toggle */}
                    <div className="flex items-start gap-3 p-4 border border-border rounded-md bg-card">
                        <input
                            id="pre-release"
                            type="checkbox"
                            checked={preRelease}
                            onChange={(e) => setPreRelease(e.target.checked)}
                            disabled={isPublishing}
                            className="mt-0.5 h-4 w-4 rounded border-border accent-primary cursor-pointer disabled:opacity-50"
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

                    {/* Assets */}
                    <div className="border border-border rounded-md overflow-hidden">
                        <div className="px-4 py-3 border-b border-border bg-card/50 flex items-center justify-between">
                            <h2 className="text-sm font-medium flex items-center gap-2">
                                <Upload className="h-4 w-4" />
                                Assets
                                {assets.length > 0 && (
                                    <span className="px-1.5 py-0.5 rounded-full bg-secondary text-muted-foreground border border-border text-[10px]">
                                        {assets.length}
                                    </span>
                                )}
                            </h2>
                            <input ref={fileInputRef} type="file" multiple className="hidden" onChange={handleFileSelected} />
                            <Button variant="outline" size="sm" onClick={() => fileInputRef.current?.click()} disabled={isPublishing}>
                                <Plus className="h-3.5 w-3.5 mr-1" />
                                Add files
                            </Button>
                        </div>

                        {assets.length === 0 ? (
                            <button
                                onClick={() => fileInputRef.current?.click()}
                                disabled={isPublishing}
                                className="w-full flex flex-col items-center justify-center gap-2 py-8 text-sm text-muted-foreground hover:text-foreground hover:bg-accent/20 transition-colors disabled:opacity-50 disabled:pointer-events-none"
                            >
                                <FileArchive className="h-5 w-5" />
                                Click to attach binaries or source archives
                            </button>
                        ) : (
                            <div className="divide-y divide-border">
                                {assets.map((asset) => (
                                    <div key={asset.localId} className="p-4 space-y-3">
                                        <div className="flex items-center gap-3">
                                            <FileArchive className="h-4 w-4 text-muted-foreground shrink-0" />
                                            <div className="flex-1 min-w-0">
                                                <input
                                                    type="text"
                                                    value={asset.name}
                                                    onChange={(e) => updateAsset(asset.localId, { name: e.target.value })}
                                                    disabled={isPublishing}
                                                    className="w-full h-7 px-2 bg-card border border-border rounded text-sm font-mono focus:outline-none focus:ring-1 focus:ring-ring disabled:opacity-50"
                                                />
                                            </div>
                                            <span className="text-xs text-muted-foreground shrink-0">{formatBytes(asset.file.size)}</span>
                                            <button
                                                onClick={() => removeAsset(asset.localId)}
                                                disabled={isPublishing}
                                                className="shrink-0 text-muted-foreground hover:text-destructive transition-colors disabled:opacity-50"
                                            >
                                                <X className="h-4 w-4" />
                                            </button>
                                        </div>
                                        <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
                                            <SelectField
                                                label="OS"
                                                value={asset.os}
                                                onChange={(v) => updateAsset(asset.localId, { os: v as OS | "" })}
                                                options={OS_OPTIONS}
                                            />
                                            <SelectField
                                                label="Architecture"
                                                value={asset.arch}
                                                onChange={(v) => updateAsset(asset.localId, { arch: v as Arch | "" })}
                                                options={ARCH_OPTIONS}
                                            />
                                            <SelectField
                                                label="libc"
                                                value={asset.libc}
                                                onChange={(v) => updateAsset(asset.localId, { libc: v as Libc | "" })}
                                                options={LIBC_OPTIONS}
                                            />
                                            <SelectField
                                                label="Kind"
                                                value={asset.kind}
                                                onChange={(v) => updateAsset(asset.localId, { kind: v as AssetKind | "" })}
                                                options={KIND_OPTIONS}
                                            />
                                        </div>
                                    </div>
                                ))}
                            </div>
                        )}
                    </div>

                    {/* Publish progress */}
                    {isPublishing && (
                        <div className="p-4 border border-border rounded-md bg-card space-y-2">
                            {publishStatus.status === "creating" && (
                                <div className="flex items-center gap-2 text-sm text-muted-foreground">
                                    <Loader2 className="h-4 w-4 animate-spin" />
                                    Creating release…
                                </div>
                            )}
                            {publishStatus.status === "uploading" && (
                                <>
                                    <div className="flex items-center justify-between text-sm">
                                        <span className="text-muted-foreground truncate">
                                            Uploading {publishStatus.index + 1}/{publishStatus.total}: {publishStatus.name}
                                        </span>
                                        <span className="font-medium shrink-0 ml-2">{publishStatus.progress}%</span>
                                    </div>
                                    <div className="w-full h-1.5 bg-secondary rounded-full overflow-hidden">
                                        <div
                                            className="h-full bg-primary rounded-full transition-all duration-150"
                                            style={{ width: `${publishStatus.progress}%` }}
                                        />
                                    </div>
                                </>
                            )}
                            {publishStatus.status === "confirming" && (
                                <div className="flex items-center gap-2 text-sm text-muted-foreground">
                                    <Loader2 className="h-4 w-4 animate-spin" />
                                    Verifying {publishStatus.index + 1}/{publishStatus.total}: {publishStatus.name}…
                                </div>
                            )}
                        </div>
                    )}

                    {/* Actions */}
                    <div className="flex items-center justify-between pt-2">
                        <Link href={`/${user}/${repo}/releases`}>
                            <Button variant="outline" disabled={isPublishing}>
                                Cancel
                            </Button>
                        </Link>
                        <Button onClick={handlePublish} disabled={!tag.trim() || !title.trim() || isPublishing}>
                            {isPublishing && <Loader2 className="h-3.5 w-3.5 mr-1.5 animate-spin" />}
                            {isPublishing ? "Publishing…" : "Publish release"}
                        </Button>
                    </div>
                </div>
            </main>
        </div>
    );
}

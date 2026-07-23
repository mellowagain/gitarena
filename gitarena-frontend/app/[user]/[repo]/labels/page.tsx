"use client";

import { useState, useRef, useEffect, CSSProperties } from "react";
import { useParams } from "next/navigation";
import useSWR from "swr";
import useSWRMutation from "swr/mutation";
import { TopBar } from "@/components/top-bar";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { jsonFetcher, postJsonFetcher, putJsonFetcher, deleteFetcher } from "@/lib/fetchers";
import { AlertCircle, GitMerge, Code, Tag, Plus, Pencil, Trash2, Check, X, RefreshCw } from "lucide-react";
import { HexColorPicker } from "react-colorful";
import { createPortal } from "react-dom";

type Label = {
    id: string;
    name: string;
    color: string;
    description: string | null;
};

type LabelsResponse = {
    labels: Label[];
};

type PermissionsResponse = {
    permissions: {
        manageIssues: boolean;
    };
};

type CreateLabelRequest = {
    name: string;
    color: string;
    description: string;
};

type UpdateLabelRequest = {
    name?: string;
    color?: string;
    description?: string;
};

const PRESET_COLORS = [
    "#e11d48",
    "#dc2626",
    "#ea580c",
    "#d97706",
    "#ca8a04",
    "#65a30d",
    "#16a34a",
    "#059669",
    "#0d9488",
    "#0891b2",
    "#2563eb",
    "#4f46e5",
    "#7c3aed",
    "#9333ea",
    "#db2777",
    "#6b7280",
    "#374151",
    "#0f172a",
];

function randomHex(): string {
    return (
        "#" +
        Math.floor(Math.random() * 0xffffff)
            .toString(16)
            .padStart(6, "0")
    );
}

function hexToRgb(hex: string) {
    const r = parseInt(hex.slice(1, 3), 16);
    const g = parseInt(hex.slice(3, 5), 16);
    const b = parseInt(hex.slice(5, 7), 16);
    return { r, g, b };
}

function contrastColor(hex: string): string {
    const { r, g, b } = hexToRgb(hex);
    const luminance = (0.299 * r + 0.587 * g + 0.114 * b) / 255;
    return luminance > 0.5 ? "#0f172a" : "#ffffff";
}

function isValidHex(hex: string): boolean {
    return /^#[0-9a-fA-F]{6}$/.test(hex);
}

function parseLabelName(name: string): { scope: string | null; value: string } {
    const idx = name.indexOf("::");
    if (idx === -1) {
        return { scope: null, value: name };
    }
    return { scope: name.slice(0, idx), value: name.slice(idx + 2) };
}

function LabelChip({ label }: { label: Label }) {
    const { scope, value } = parseLabelName(label.name);
    const text = contrastColor(label.color);
    if (scope) {
        return (
            <span className="inline-flex max-w-full items-stretch overflow-hidden rounded-full text-xs font-medium">
                <span style={{ backgroundColor: label.color, color: text }} className="min-w-0 truncate px-2.5 py-1 opacity-70">
                    {scope}
                </span>
                <span style={{ backgroundColor: label.color, color: text }} className="min-w-0 truncate px-2.5 py-1">
                    {value}
                </span>
            </span>
        );
    }
    return (
        <span
            style={{ backgroundColor: label.color, color: text }}
            className="inline-flex max-w-full items-center truncate rounded-full px-2.5 py-1 text-xs font-medium"
        >
            {label.name}
        </span>
    );
}

function ColorPicker({ value, onChange }: { value: string; onChange: (v: string) => void }) {
    const [open, setOpen] = useState(false);
    const [inputVal, setInputVal] = useState(value);
    const [popoverStyle, setPopoverStyle] = useState<CSSProperties>({});
    const triggerRef = useRef<HTMLButtonElement>(null);
    const popoverRef = useRef<HTMLDivElement>(null);

    // Approximate height of the popover to decide open direction
    const POPOVER_HEIGHT = 370;

    const openPicker = () => {
        if (triggerRef.current) {
            const rect = triggerRef.current.getBoundingClientRect();
            const spaceBelow = window.innerHeight - rect.bottom;
            if (spaceBelow >= POPOVER_HEIGHT) {
                setPopoverStyle({ position: "fixed", top: rect.bottom + 8, left: rect.left });
            } else {
                setPopoverStyle({ position: "fixed", bottom: window.innerHeight - rect.top + 8, left: rect.left });
            }
        }
        setOpen((v) => !v);
    };

    useEffect(() => {
        if (!open) {
            return;
        }
        function handler(e: MouseEvent) {
            const target = e.target as Node;
            const inTrigger = triggerRef.current?.contains(target);
            const inPopover = popoverRef.current?.contains(target);
            if (!inTrigger && !inPopover) {
                setOpen(false);
            }
        }
        document.addEventListener("mousedown", handler);
        return () => document.removeEventListener("mousedown", handler);
    }, [open]);

    const handleInput = (v: string) => {
        setInputVal(v);
        if (isValidHex(v)) {
            onChange(v);
        }
    };

    const pickPreset = (c: string) => {
        onChange(c);
        setInputVal(c);
    };

    const pickRandom = () => {
        const random = randomHex();
        onChange(random);
        setInputVal(random);
    };

    const displayColor = isValidHex(inputVal) ? inputVal : "#6b7280";

    const popover = open
        ? createPortal(
              <div
                  ref={popoverRef}
                  style={popoverStyle}
                  className="z-[9999] w-72 bg-card border border-border rounded-lg shadow-xl p-4 space-y-3 [&_.react-colorful]:w-full [&_.react-colorful]:rounded-md [&_.react-colorful\_\_saturation]:rounded-t-md [&_.react-colorful\_\_last-control]:rounded-b-md [&_.react-colorful\_\_pointer]:w-5 [&_.react-colorful\_\_pointer]:h-5 [&_.react-colorful\_\_pointer]:border-2 [&_.react-colorful\_\_pointer]:border-white [&_.react-colorful\_\_pointer]:shadow-md [&_.react-colorful\_\_hue]:h-4 [&_.react-colorful\_\_hue]:rounded-none"
              >
                  {/* Gradient + hue picker */}
                  <HexColorPicker
                      color={displayColor}
                      onChange={(c) => {
                          onChange(c);
                          setInputVal(c);
                      }}
                  />

                  {/* Divider */}
                  <div className="border-t border-border" />

                  {/* Presets */}
                  <div>
                      <p className="text-xs font-medium uppercase tracking-wider text-muted-foreground mb-2">Presets</p>
                      <div className="grid grid-cols-9 gap-1.5">
                          {PRESET_COLORS.map((c) => (
                              <button
                                  key={c}
                                  type="button"
                                  onClick={() => pickPreset(c)}
                                  style={{ backgroundColor: c }}
                                  className={`h-6 w-6 rounded border-2 transition-all hover:scale-110 focus:outline-none ${
                                      value === c ? "border-foreground scale-110" : "border-transparent"
                                  }`}
                                  aria-label={c}
                              />
                          ))}
                          <button
                              type="button"
                              onClick={pickRandom}
                              className="h-6 w-6 rounded border-2 border-border bg-secondary flex items-center justify-center text-muted-foreground hover:text-foreground hover:scale-110 transition-all focus:outline-none"
                              title="Random"
                          >
                              <RefreshCw className="h-3 w-3" />
                          </button>
                      </div>
                  </div>

                  {/* Divider */}
                  <div className="border-t border-border" />

                  {/* Hex input inside popover */}
                  <div className="flex items-center gap-2">
                      <div className="h-8 w-8 rounded border border-border shrink-0" style={{ backgroundColor: displayColor }} />
                      <input
                          type="text"
                          value={inputVal}
                          onChange={(e) => handleInput(e.target.value)}
                          placeholder="#000000"
                          maxLength={7}
                          className="h-8 flex-1 px-2 font-mono text-sm bg-background border border-border rounded focus:outline-none focus:ring-1 focus:ring-ring"
                      />
                  </div>
                  {inputVal.length > 1 && !isValidHex(inputVal) && <p className="text-xs text-destructive -mt-1">Invalid hex color</p>}
              </div>,
              document.body
          )
        : null;

    return (
        <div className="relative flex items-center gap-2">
            {/* Swatch trigger */}
            <button
                ref={triggerRef}
                type="button"
                onClick={openPicker}
                style={{ backgroundColor: displayColor }}
                className="h-11 w-11 rounded-md border border-border shrink-0 hover:opacity-90 transition-opacity focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 focus:ring-offset-background"
                aria-label="Open color picker"
            />

            {/* Hex input */}
            <input
                type="text"
                value={inputVal}
                onChange={(e) => handleInput(e.target.value)}
                placeholder="#000000"
                maxLength={7}
                className="h-11 w-36 px-3 font-mono text-sm bg-background border border-border rounded-md focus:outline-none focus:ring-1 focus:ring-ring"
            />

            {popover}
        </div>
    );
}

type EditState = {
    id: string | null; // null = new label
    name: string;
    color: string;
    description: string;
};

function LabelRowSkeleton() {
    return (
        <div className="flex flex-col items-start gap-3 px-4 py-3.5 sm:flex-row sm:items-center sm:gap-4">
            <Skeleton className="h-5 w-28 rounded-full" />
            <Skeleton className="h-4 w-full max-w-xs sm:flex-1" />
            <div className="flex w-full justify-end gap-2 sm:w-auto">
                <Skeleton className="h-7 w-16" />
                <Skeleton className="h-7 w-16" />
            </div>
        </div>
    );
}

export default function LabelsPage() {
    const params = useParams();
    const user = params.user as string;
    const repo = params.repo as string;

    const labelsKey = user && repo ? `/api/repos/${user}/${repo}/labels` : null;
    const permissionsKey = user && repo ? `/api/repos/${user}/${repo}/permissions` : null;

    const { data, isLoading, error } = useSWR<LabelsResponse>(labelsKey, jsonFetcher);
    const { data: permissionsData } = useSWR<PermissionsResponse>(permissionsKey, jsonFetcher);

    const canManageIssues = permissionsData?.permissions.manageIssues ?? false;

    const { trigger: triggerCreate, isMutating: isCreating } = useSWRMutation(
        labelsKey,
        (url: string, { arg }: { arg: CreateLabelRequest }) => postJsonFetcher<CreateLabelRequest, Label>(url, { arg })
    );

    const { trigger: triggerUpdate, isMutating: isUpdating } = useSWRMutation(
        labelsKey,
        (url: string, { arg }: { arg: { id: string } & UpdateLabelRequest }) => {
            const { id, ...body } = arg;
            return putJsonFetcher<UpdateLabelRequest, Label>(`${url}/${id}`, { arg: body });
        }
    );

    const { trigger: triggerDelete } = useSWRMutation(labelsKey, (url: string, { arg }: { arg: string }) => deleteFetcher(`${url}/${arg}`));

    const [editState, setEditState] = useState<EditState | null>(null);
    const [deleteConfirm, setDeleteConfirm] = useState<string | null>(null);
    const [search, setSearch] = useState("");

    const labels: Label[] = data?.labels ?? [];

    const filtered = labels.filter(
        (l) => l.name.toLowerCase().includes(search.toLowerCase()) || (l.description ?? "").toLowerCase().includes(search.toLowerCase())
    );

    // Group scoped labels by scope prefix
    const scopeMap = new Map<string | null, Label[]>();
    for (const label of filtered) {
        const { scope } = parseLabelName(label.name);
        if (!scopeMap.has(scope)) {
            scopeMap.set(scope, []);
        }
        scopeMap.get(scope)!.push(label);
    }

    const groups = [
        ...(scopeMap.has(null) ? [{ scope: null, items: scopeMap.get(null)! }] : []),
        ...[...scopeMap.entries()]
            .filter(([s]) => s !== null)
            .sort(([a], [b]) => a!.localeCompare(b!))
            .map(([scope, items]) => ({ scope, items })),
    ];

    function startEdit(label: Label) {
        setEditState({ id: label.id, name: label.name, color: label.color, description: label.description ?? "" });
        setDeleteConfirm(null);
    }

    function startNew() {
        setEditState({ id: null, name: "", color: randomHex(), description: "" });
        setDeleteConfirm(null);
    }

    function cancelEdit() {
        setEditState(null);
    }

    async function saveEdit() {
        if (!editState || !editState.name.trim() || !isValidHex(editState.color)) {
            return;
        }
        if (editState.id === null) {
            await triggerCreate({ name: editState.name.trim(), color: editState.color, description: editState.description.trim() });
        } else {
            await triggerUpdate({
                id: editState.id,
                name: editState.name.trim(),
                color: editState.color,
                description: editState.description.trim(),
            });
        }
        setEditState(null);
    }

    async function deleteLabel(id: string) {
        await triggerDelete(id);
        setDeleteConfirm(null);
    }

    const isEditing = (id: string) => editState?.id === id;
    const isMutating = isCreating || isUpdating;

    return (
        <div className="flex flex-col h-screen bg-background text-foreground">
            <TopBar
                breadcrumb={[
                    { label: user, href: `/${user}` },
                    { label: repo, href: `/${user}/${repo}` },
                    { label: "Issues", href: `/${user}/${repo}/issues` },
                    { label: "Labels" },
                ]}
                navLinks={[
                    { label: "Code", href: `/${user}/${repo}`, icon: <Code className="h-[18px] w-[18px]" /> },
                    { label: "Issues", href: `/${user}/${repo}/issues`, icon: <AlertCircle className="h-[18px] w-[18px]" /> },
                    //{ label: "Merge Requests", href: `/${user}/${repo}/merge-requests`, icon: <GitMerge className="h-[18px] w-[18px]" /> },
                ]}
            />

            <main className="flex-1 overflow-y-auto">
                <div className="max-w-4xl mx-auto px-6 py-8 space-y-6">
                    {/* Header */}
                    <div className="flex flex-col items-start gap-4 sm:flex-row sm:items-center sm:justify-between">
                        <div className="min-w-0">
                            <h1 className="text-2xl font-semibold flex items-center gap-2">
                                <Tag className="h-5 w-5 text-muted-foreground" />
                                Labels
                            </h1>
                            <p className="text-sm text-muted-foreground mt-1">
                                {labels.length} label{labels.length !== 1 ? "s" : ""} · Use{" "}
                                <code className="font-mono text-xs bg-secondary px-1 rounded">scope::name</code> for scoped labels. Only one
                                label per scope can be applied at a time
                            </p>
                        </div>
                        {canManageIssues && (
                            <Button size="sm" className="w-full gap-2 sm:w-auto" onClick={startNew} disabled={editState?.id === null}>
                                <Plus className="h-4 w-4" />
                                New label
                            </Button>
                        )}
                    </div>

                    {/* New label form */}
                    {canManageIssues && editState?.id === null && (
                        <div className="border border-border rounded-lg">
                            <div className="px-4 py-3 border-b border-border bg-card rounded-t-lg">
                                <span className="font-medium text-sm">New label</span>
                            </div>
                            <LabelForm
                                state={editState}
                                onChange={setEditState}
                                onSave={saveEdit}
                                onCancel={cancelEdit}
                                isMutating={isMutating}
                            />
                        </div>
                    )}

                    {/* Search */}
                    <div className="relative max-w-sm">
                        <input
                            type="text"
                            value={search}
                            onChange={(e) => setSearch(e.target.value)}
                            placeholder="Filter labels…"
                            className="w-full h-9 pl-9 pr-3 bg-card border border-border rounded-md text-sm focus:outline-none focus:ring-1 focus:ring-ring placeholder:text-muted-foreground"
                        />
                        <svg
                            className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground pointer-events-none"
                            fill="none"
                            stroke="currentColor"
                            viewBox="0 0 24 24"
                        >
                            <path
                                strokeLinecap="round"
                                strokeLinejoin="round"
                                strokeWidth={2}
                                d="M21 21l-4.35-4.35M17 11A6 6 0 1 1 5 11a6 6 0 0 1 12 0z"
                            />
                        </svg>
                    </div>

                    {/* Loading skeleton */}
                    {isLoading && (
                        <div className="border border-border rounded-lg overflow-hidden divide-y divide-border">
                            <LabelRowSkeleton />
                            <LabelRowSkeleton />
                            <LabelRowSkeleton />
                            <LabelRowSkeleton />
                        </div>
                    )}

                    {/* Error state */}
                    {error && !isLoading && (
                        <div className="border border-border rounded-lg py-8 flex flex-col items-center gap-2 text-center">
                            <p className="text-sm text-destructive">Failed to load labels</p>
                        </div>
                    )}

                    {/* Label groups */}
                    {!isLoading &&
                        !error &&
                        (filtered.length === 0 ? (
                            <div className="border border-border rounded-lg py-16 flex flex-col items-center gap-2 text-center">
                                <Tag className="h-8 w-8 text-muted-foreground" />
                                <p className="text-sm font-medium">No labels found</p>
                                <p className="text-xs text-muted-foreground">Try a different search term or create a new label.</p>
                            </div>
                        ) : (
                            <div className="space-y-5">
                                {groups.map(({ scope, items }) => (
                                    <div key={scope ?? "__unscoped__"}>
                                        {scope && (
                                            <div className="flex items-center gap-2 mb-2">
                                                <span className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                                                    {scope}
                                                </span>
                                                <span className="text-xs text-muted-foreground">({items.length})</span>
                                            </div>
                                        )}
                                        <div className="border border-border rounded-lg overflow-hidden">
                                            {items.map((label, i) => (
                                                <div key={label.id}>
                                                    {/* Row */}
                                                    {!isEditing(label.id) ? (
                                                        <div
                                                            className={`flex flex-col items-start gap-2 px-4 py-3.5 transition-colors hover:bg-accent/30 sm:flex-row sm:items-center sm:gap-4 ${i > 0 ? "border-t border-border" : ""}`}
                                                        >
                                                            <div className="w-full min-w-0 sm:w-36 sm:shrink-0">
                                                                <LabelChip label={label} />
                                                            </div>
                                                            <p className="w-full min-w-0 truncate text-sm text-muted-foreground sm:flex-1">
                                                                {label.description}
                                                            </p>
                                                            <div className="flex w-full shrink-0 items-center justify-end gap-1 sm:w-auto">
                                                                {canManageIssues &&
                                                                    (deleteConfirm === label.id ? (
                                                                        <div className="flex items-center gap-2">
                                                                            <span className="text-xs text-muted-foreground">Delete?</span>
                                                                            <button
                                                                                onClick={() => deleteLabel(label.id)}
                                                                                className="flex items-center gap-1 px-2 py-1 text-xs text-red-500 hover:text-red-400 border border-red-500/30 rounded-md transition-colors"
                                                                            >
                                                                                <Check className="h-3 w-3" />
                                                                                Yes
                                                                            </button>
                                                                            <button
                                                                                onClick={() => setDeleteConfirm(null)}
                                                                                className="flex items-center gap-1 px-2 py-1 text-xs text-muted-foreground hover:text-foreground border border-border rounded-md transition-colors"
                                                                            >
                                                                                <X className="h-3 w-3" />
                                                                                No
                                                                            </button>
                                                                        </div>
                                                                    ) : (
                                                                        <>
                                                                            <button
                                                                                onClick={() => startEdit(label)}
                                                                                className="flex items-center gap-1.5 px-2.5 py-1.5 text-xs text-muted-foreground hover:text-foreground border border-transparent hover:border-border rounded-md transition-colors"
                                                                            >
                                                                                <Pencil className="h-3.5 w-3.5" />
                                                                                Edit
                                                                            </button>
                                                                            <button
                                                                                onClick={() => setDeleteConfirm(label.id)}
                                                                                className="flex items-center gap-1.5 px-2.5 py-1.5 text-xs text-red-500/70 hover:text-red-500 border border-transparent hover:border-red-500/30 rounded-md transition-colors"
                                                                            >
                                                                                <Trash2 className="h-3.5 w-3.5" />
                                                                                Delete
                                                                            </button>
                                                                        </>
                                                                    ))}
                                                            </div>
                                                        </div>
                                                    ) : (
                                                        <div className={`${i > 0 ? "border-t border-border" : ""} bg-accent/20`}>
                                                            <LabelForm
                                                                state={editState!}
                                                                onChange={setEditState}
                                                                onSave={saveEdit}
                                                                onCancel={cancelEdit}
                                                                isMutating={isMutating}
                                                            />
                                                        </div>
                                                    )}
                                                </div>
                                            ))}
                                        </div>
                                    </div>
                                ))}
                            </div>
                        ))}
                </div>
            </main>
        </div>
    );
}

function LabelForm({
    state,
    onChange,
    onSave,
    onCancel,
    isMutating,
}: {
    state: EditState;
    onChange: (s: EditState | null) => void;
    onSave: () => void;
    onCancel: () => void;
    isMutating: boolean;
}) {
    const canSave = state.name.trim().length > 0 && isValidHex(state.color);
    const { scope, value } = parseLabelName(state.name);
    const previewLabel: Label = { id: "", name: state.name || "preview", color: state.color, description: state.description };

    return (
        <div className="px-4 py-4 space-y-4">
            <div className="grid grid-cols-[1fr_1fr_auto] gap-4 items-start">
                {/* Name */}
                <div className="space-y-1.5">
                    <label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Name</label>
                    <input
                        type="text"
                        value={state.name}
                        onChange={(e) => onChange({ ...state, name: e.target.value })}
                        placeholder="e.g. bug or priority::high"
                        className="w-full h-9 px-3 text-sm bg-background border border-border rounded-md focus:outline-none focus:ring-1 focus:ring-ring"
                    />
                    {scope && (
                        <p className="text-[11px] text-muted-foreground">
                            Scoped label · scope: <span className="font-mono">{scope}</span> · value:{" "}
                            <span className="font-mono">{value}</span>
                        </p>
                    )}
                </div>

                {/* Description */}
                <div className="space-y-1.5">
                    <label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Description</label>
                    <input
                        type="text"
                        value={state.description}
                        onChange={(e) => onChange({ ...state, description: e.target.value })}
                        placeholder="Optional description"
                        className="w-full h-9 px-3 text-sm bg-background border border-border rounded-md focus:outline-none focus:ring-1 focus:ring-ring"
                    />
                </div>

                {/* Preview */}
                <div className="space-y-1.5 pt-5">
                    <LabelChip label={previewLabel} />
                </div>
            </div>

            {/* Color picker */}
            <div className="space-y-1.5">
                <label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Color</label>
                <ColorPicker value={state.color} onChange={(c) => onChange({ ...state, color: c })} />
            </div>

            {/* Actions */}
            <div className="flex items-center gap-2 pt-1">
                <Button size="sm" onClick={onSave} disabled={!canSave || isMutating} className="gap-2">
                    {isMutating ? <RefreshCw className="h-4 w-4 animate-spin" /> : <Check className="h-4 w-4" />}
                    {state.id === null ? "Create label" : "Save changes"}
                </Button>
                <Button size="sm" variant="ghost" onClick={onCancel} disabled={isMutating} className="gap-2 text-muted-foreground">
                    <X className="h-4 w-4" />
                    Cancel
                </Button>
            </div>
        </div>
    );
}

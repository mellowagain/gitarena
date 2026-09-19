"use client";

import { useState } from "react";
import useSWR from "swr";
import { ChevronRight } from "lucide-react";
import { Checkbox } from "@/components/ui/checkbox";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/utils";

const PERMISSIONS_KEY = "/api/tokens/permissions";

/** Display names for each permission group, keyed by the prefix before the first colon. */
const groupLabels: Record<string, string> = {
    contents: "Contents",
    repo: "Repository",
    releases: "Releases",
    issues: "Issues",
    org: "Organization",
    user: "User",
    ssh_keys: "SSH keys",
    sessions: "Sessions",
    stars: "Stars",
    events: "Events",
    audit_log: "Audit log",
    admin: "Administration",
};

interface PermissionGroup {
    key: string;
    label: string;
    permissions: string[];
}

function groupPermissions(permissions: string[]): PermissionGroup[] {
    const groups: PermissionGroup[] = [];

    for (const permission of permissions) {
        const key = permission.slice(0, permission.indexOf(":"));
        const existing = groups.find((group) => group.key === key);

        if (existing) {
            existing.permissions.push(permission);
        } else {
            groups.push({ key, label: groupLabels[key] ?? key.replaceAll("_", " "), permissions: [permission] });
        }
    }

    return groups;
}

interface TokenPermissionPickerProps {
    selected: string[];
    onChange: (permissions: string[]) => void;
}

/** Checkbox picker for the permissions a token grants, grouped by the resource they apply to. */
export function TokenPermissionPicker({ selected, onChange }: TokenPermissionPickerProps) {
    const { data: permissions, isLoading, error } = useSWR<string[]>(PERMISSIONS_KEY);
    const [expanded, setExpanded] = useState<string[]>(() => [
        ...new Set(selected.map((permission) => permission.slice(0, permission.indexOf(":")))),
    ]);

    if (isLoading) {
        return (
            <div className="border border-border rounded-md overflow-hidden">
                {[0, 1, 2, 3, 4, 5].map((i) => (
                    <div key={i} className={cn("flex items-center gap-2.5 px-4 py-3", i > 0 && "border-t border-border")}>
                        <Skeleton className="h-3.5 w-3.5" />
                        <Skeleton className="h-4 w-28" />
                        <Skeleton className="h-3 w-16 ml-auto" />
                    </div>
                ))}
            </div>
        );
    }

    if (error || !permissions) {
        return (
            <div className="border border-border rounded-md px-4 py-6 text-center text-sm text-muted-foreground">
                Failed to load the available permissions.
            </div>
        );
    }

    const groups = groupPermissions(permissions);

    function toggleExpanded(key: string) {
        setExpanded((current) => (current.includes(key) ? current.filter((group) => group !== key) : [...current, key]));
    }

    function togglePermission(permission: string) {
        onChange(selected.includes(permission) ? selected.filter((current) => current !== permission) : [...selected, permission]);
    }

    function toggleGroup(group: PermissionGroup, select: boolean) {
        onChange(
            select
                ? [...new Set([...selected, ...group.permissions])]
                : selected.filter((permission) => !group.permissions.includes(permission))
        );
    }

    return (
        <div className="border border-border rounded-md overflow-hidden">
            {groups.map((group, i) => {
                const isExpanded = expanded.includes(group.key);
                const count = group.permissions.filter((permission) => selected.includes(permission)).length;
                const allSelected = count === group.permissions.length;

                return (
                    <div key={group.key} className={cn(i > 0 && "border-t border-border")}>
                        <button
                            type="button"
                            onClick={() => toggleExpanded(group.key)}
                            aria-expanded={isExpanded}
                            className="w-full flex items-center gap-2.5 px-4 py-3 text-left hover:bg-accent/30 transition-colors"
                        >
                            <ChevronRight
                                className={cn("h-3.5 w-3.5 text-muted-foreground shrink-0 transition-transform", isExpanded && "rotate-90")}
                            />
                            <span className="text-sm font-medium">{group.label}</span>
                            <span className="ml-auto text-xs text-muted-foreground">{count > 0 ? `${count} selected` : "none"}</span>
                        </button>

                        {isExpanded && (
                            <div className="px-4 pb-4 pl-10">
                                <button
                                    type="button"
                                    onClick={() => toggleGroup(group, !allSelected)}
                                    className="text-xs text-muted-foreground hover:text-foreground transition-colors underline underline-offset-2 mb-3"
                                >
                                    {allSelected ? "Clear all" : "Select all"}
                                </button>

                                <div className="grid grid-cols-2 gap-x-4 gap-y-2.5">
                                    {group.permissions.map((permission) => (
                                        <label key={permission} className="flex items-center gap-2 cursor-pointer min-w-0">
                                            <Checkbox
                                                checked={selected.includes(permission)}
                                                onCheckedChange={() => togglePermission(permission)}
                                            />
                                            <span className="font-mono text-xs text-muted-foreground truncate">{permission}</span>
                                        </label>
                                    ))}
                                </div>
                            </div>
                        )}
                    </div>
                );
            })}
        </div>
    );
}

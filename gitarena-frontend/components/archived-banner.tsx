"use client";

import { Archive } from "lucide-react";

interface ArchivedBannerProps {
    archivedAt: string;
}

export function ArchivedBanner({ archivedAt }: ArchivedBannerProps) {
    const date = new Date(archivedAt).toLocaleDateString(undefined, {
        year: "numeric",
        month: "long",
        day: "numeric",
    });

    return (
        <div className="flex items-center gap-2 px-4 py-2 bg-amber-500/10 border-b border-amber-500/30 text-amber-700 dark:text-amber-400 text-sm">
            <Archive className="h-4 w-4 shrink-0" />
            <span>This repository was archived on {date}. It is now read-only.</span>
        </div>
    );
}

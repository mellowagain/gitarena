"use client";

import { Archive } from "lucide-react";
import { Alert, AlertDescription } from "@/components/ui/alert";

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
        <Alert variant="warning" className="rounded-none border-x-0 border-t-0 py-2">
            <Archive />
            <AlertDescription className="text-warning">This repository was archived on {date}. It is now read-only.</AlertDescription>
        </Alert>
    );
}

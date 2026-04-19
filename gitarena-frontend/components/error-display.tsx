"use client";

import { TopBar } from "@/components/top-bar";

interface ErrorDisplayProps {
    failed: string;
    error?: Error;
}

export function ErrorDisplay({ failed, error }: ErrorDisplayProps) {
    return (
        <div className="min-h-screen bg-background text-foreground flex flex-col">
            <TopBar />
            <div className="flex flex-1 items-center justify-center">
                <div className="text-center space-y-3 max-w-sm">
                    <p className="text-sm font-medium">Failed to load {failed}</p>
                    <p className="text-xs text-muted-foreground">Could not reach the server. Please check your connection and try again.</p>
                    {error?.message && <p className="text-xs font-mono text-muted-foreground/60">{error.message}</p>}
                    <button
                        onClick={() => window.location.reload()}
                        className="mt-2 px-3 py-1.5 text-xs rounded border border-border hover:bg-accent transition-colors"
                    >
                        Retry
                    </button>
                </div>
            </div>
        </div>
    );
}

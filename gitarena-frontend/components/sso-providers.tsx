"use client";

import useSWR from "swr";
import { KeyRound } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { ErrorDisplay } from "@/components/error-display";
import { FieldSeparator } from "@/components/ui/field";
import { GitHubIcon } from "@/components/github-icon";

function GitLabIcon({ className }: { className?: string }) {
    return (
        <svg className={className} viewBox="0 0 24 24" fill="currentColor">
            <path d="M22.65 14.39L12 22.13 1.35 14.39a.84.84 0 0 1-.3-.94l1.22-3.78 2.44-7.51A.42.42 0 0 1 4.82 2a.43.43 0 0 1 .58 0 .42.42 0 0 1 .11.18l2.44 7.49h8.1l2.44-7.51A.42.42 0 0 1 18.6 2a.43.43 0 0 1 .58 0 .42.42 0 0 1 .11.18l2.44 7.51L23 13.45a.84.84 0 0 1-.35.94z" />
        </svg>
    );
}

function BitbucketIcon({ className }: { className?: string }) {
    return (
        <svg className={className} viewBox="0 0 24 24" fill="currentColor">
            <path d="M.778 1.213a.768.768 0 0 0-.768.892l3.263 19.81c.084.5.515.868 1.022.873H19.95a.772.772 0 0 0 .77-.646l3.27-20.03a.768.768 0 0 0-.768-.891zM14.52 15.53H9.522L8.17 8.466h7.561z" />
        </svg>
    );
}

interface KnownProvider {
    label: string;
    icon: React.ComponentType<{ className?: string }>;
}

// Provider names come from lowercase setting keys, so only well-known ones get proper brand casing and an icon
const KNOWN_PROVIDERS: Record<string, KnownProvider> = {
    github: { label: "GitHub", icon: GitHubIcon },
    gitlab: { label: "GitLab", icon: GitLabIcon },
    bitbucket: { label: "Bitbucket", icon: BitbucketIcon },
};

export function SsoProviders() {
    const { data, isLoading, error } = useSWR<string[]>("/api/sso");

    if (!isLoading && !error && !data?.length) {
        return null;
    }

    return (
        <>
            <FieldSeparator className="mt-0 mb-6 *:data-[slot=field-separator-content]:px-4">or continue with</FieldSeparator>

            {isLoading && (
                <div className="grid grid-cols-2 gap-3">
                    <Skeleton className="h-11 rounded-lg" />
                    <Skeleton className="h-11 rounded-lg" />
                </div>
            )}

            {error && <ErrorDisplay failed="SSO providers" error={error} />}

            {data && data.length > 0 && (
                <div className="grid grid-cols-2 gap-3">
                    {data.map((name) => {
                        const known = KNOWN_PROVIDERS[name.toLowerCase()];
                        const Icon = known?.icon ?? KeyRound;
                        const label = known?.label ?? name.charAt(0).toUpperCase() + name.slice(1);

                        return (
                            <Button key={name} variant="outline" className="w-full h-11 gap-2 odd:last:col-span-2" asChild>
                                <a href={`/api/sso/${encodeURIComponent(name)}`}>
                                    <Icon className="h-4 w-4" />
                                    {label}
                                </a>
                            </Button>
                        );
                    })}
                </div>
            )}
        </>
    );
}

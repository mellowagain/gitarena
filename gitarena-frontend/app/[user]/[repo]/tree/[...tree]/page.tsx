"use client";

import { use } from "react";
import useSWR from "swr";
import { RepoPageContent, RepoPageSkeleton, type RepoMetadata } from "@/app/[user]/[repo]/page";
import { ErrorDisplay } from "@/components/error-display";

export default function RepoTreePage({ params }: { params: Promise<{ user: string; repo: string; tree: string[] }> }) {
    const { user, repo, tree } = use(params);
    const branch = decodeURIComponent(tree[0]);
    const file = tree.length > 1 ? tree.slice(1).map(decodeURIComponent).join("/") : null;

    const { data, error, isLoading } = useSWR<RepoMetadata>(`/api/repos/${user}/${repo}`);

    if (isLoading) {
        return <RepoPageSkeleton user={user} repo={repo} />;
    }

    if (error || !data) {
        return <ErrorDisplay failed="repo" error={error} />;
    }

    return <RepoPageContent user={user} repo={repo} meta={data} initialBranch={branch} initialFile={file} />;
}

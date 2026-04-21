"use client";

import React from "react";
import useSWR from "swr";
import { textFetcher, CodeBlockContent, CodeBlockSkeleton } from "@/components/code-block";
import { MarkdownRenderer, isMarkdown } from "@/components/markdown-renderer";
import { ErrorDisplay } from "@/components/error-display";

export function FileContent({
    user,
    repo,
    branch,
    filename,
    showSource = false,
    wrapLines = false,
}: {
    user: string;
    repo: string;
    branch: string | null;
    filename: string;
    showSource?: boolean;
    wrapLines?: boolean;
}) {
    const url = branch ? `http://localhost:8080/${user}/${repo}/tree/${branch}/~blob/${filename}` : null;
    const { data, error, isLoading } = useSWR<string>(url, textFetcher);

    if (isLoading || !branch) {
        return <CodeBlockSkeleton />;
    }

    if (error || !data) {
        return <ErrorDisplay failed={"file"} error={error} />;
    }

    if (isMarkdown(filename)) {
        return <MarkdownRenderer content={data} fileName={filename} showSource={showSource} wrapLines={wrapLines} />;
    }

    return <CodeBlockContent content={data} filename={filename} wrapLines={wrapLines} />;
}

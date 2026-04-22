"use client";

import React, { useEffect } from "react";
import useSWR from "swr";
import { CodeBlockContent, CodeBlockSkeleton } from "@/components/code-block";
import { MarkdownRenderer, isMarkdown } from "@/components/markdown-renderer";
import { ErrorDisplay } from "@/components/error-display";
import { Button } from "@/components/ui/button";
import { AlertTriangle, BinaryIcon, ExternalLink } from "lucide-react";

interface FileContentResponse {
    content: string | null;
    size: number;
    isBinary: boolean;
    isTruncated: boolean;
    commit: FileCommit;
}

export interface FileCommit {
    sha1: string;
    message: string;
    time: number;
    authorName: string;
    authorEmail: string;
    authorUid: number | null;
}

const jsonFetcher = (url: string) =>
    fetch(url).then((res) => {
        if (!res.ok) {
            throw new Error(res.statusText);
        }
        return res.json();
    });

interface FileContentProps {
    user: string;
    repo: string;
    branch: string | null;
    filename: string;
    showSource?: boolean;
    wrapLines?: boolean;
    setFileSize?: (size: number | null) => void;
    setCommit?: (commit: FileCommit | null) => void;
}

export function FileContent({
    user,
    repo,
    branch,
    filename,
    showSource = false,
    wrapLines = false,
    setFileSize,
    setCommit,
}: FileContentProps) {
    const url = branch ? `http://localhost:8080/api/repos/${user}/${repo}/branch/${branch}/files/${filename}` : null;

    const { data, error, isLoading } = useSWR<FileContentResponse>(url, jsonFetcher);

    useEffect(() => {
        if (data && setFileSize) {
            setFileSize(data.isTruncated ? null : data.size);
        }
        if (data && setCommit) {
            setCommit(data.commit);
        }
        if (!data && setCommit) {
            setCommit(null);
        }
    }, [data, setFileSize, setCommit]);

    if (isLoading || !branch) {
        return <CodeBlockSkeleton />;
    }

    if (error || !data) {
        return <ErrorDisplay failed={"file"} error={error} />;
    }

    if (data.isBinary) {
        return (
            <div className="flex flex-col items-center justify-center gap-3 py-16 text-muted-foreground">
                <BinaryIcon className="h-10 w-10 opacity-40" />
                <div className="text-center space-y-3 max-w-sm">
                    <p className="text-sm font-medium">Binary file</p>
                    <p className="text-xs text-muted-foreground">Binary files cannot be displayed on the frontend</p>
                    <Button asChild variant="outline" size="sm" className="mt-2">
                        <a href={`http://localhost:8080/${user}/${repo}/tree/${branch}/~blob/${filename}`} target="_blank" rel="noreferrer">
                            <ExternalLink className="h-3 w-3" />
                            View Raw
                        </a>
                    </Button>
                </div>
            </div>
        );
    }

    const content = data.content ?? "";

    const truncatedWarning = data.isTruncated && (
        <div className="flex items-center gap-2 px-4 py-2.5 border-b border-border bg-yellow-500/10 text-yellow-600 dark:text-yellow-400 text-sm shrink-0">
            <AlertTriangle className="h-4 w-4 shrink-0" />
            <span>
                Content has been truncated to 5 MB because the file is too large to display in full.{" "}
                <a
                    href={`http://localhost:8080/${user}/${repo}/tree/${branch}/~blob/${filename}`}
                    target="_blank"
                    rel="noreferrer"
                    className="underline underline-offset-2 hover:opacity-70 transition-opacity"
                >
                    View raw file
                </a>
            </span>
        </div>
    );

    if (isMarkdown(filename)) {
        return (
            <>
                {truncatedWarning}
                <MarkdownRenderer content={content} fileName={filename} showSource={showSource} wrapLines={wrapLines} />
            </>
        );
    }

    return (
        <>
            {truncatedWarning}
            <CodeBlockContent content={content} filename={filename} wrapLines={wrapLines} />
        </>
    );
}

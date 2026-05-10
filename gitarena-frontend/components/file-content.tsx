"use client";

import React, { useEffect } from "react";
import useSWR from "swr";
import { CodeBlockContent, CodeBlockSkeleton } from "@/components/code-block";
import { MarkdownRenderer, isMarkdown } from "@/components/markdown-renderer";
import { BlameView, type BlameHunk } from "@/components/blame-view";
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

interface BlameResponse {
    hunks: BlameHunk[];
}

export interface FileCommit {
    sha1: string;
    message: string;
    time: number;
    authorName: string;
    authorEmail: string;
    authorUid: string | null;
}

interface FileContentProps {
    user: string;
    repo: string;
    branch: string;
    filename: string;
    showSource?: boolean;
    showBlame?: boolean;
    wrapLines?: boolean;
    setFileSize?: (size: number | null) => void;
    setCommit?: (commit: FileCommit | null) => void;
    setIsBinary?: (isBinary: boolean) => void;
}

export function FileContent({
    user,
    repo,
    branch,
    filename,
    showSource = false,
    showBlame = false,
    wrapLines = false,
    setFileSize,
    setCommit,
    setIsBinary,
}: FileContentProps) {
    const { data, error, isLoading } = useSWR<FileContentResponse>(`/api/repos/${user}/${repo}/branch/${branch}/files/${filename}`);
    const { data: blameData, isLoading: isBlameLoading } = useSWR<BlameResponse>(
        showBlame && data && !data.isBinary ? `/api/repos/${user}/${repo}/branch/${branch}/blame/${filename}` : null
    );

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
        if (data && setIsBinary) {
            setIsBinary(data.isBinary);
        }
    }, [data, setFileSize, setCommit, setIsBinary]);

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

    if (showBlame) {
        if (isBlameLoading || !blameData) {
            return <CodeBlockSkeleton />;
        }
        return <BlameView user={user} repo={repo} hunks={blameData.hunks} content={content} filename={filename} wrapLines={wrapLines} />;
    }

    if (isMarkdown(filename)) {
        return (
            <>
                {truncatedWarning}
                <MarkdownRenderer
                    content={content}
                    fileName={filename}
                    user={user}
                    repo={repo}
                    branch={branch}
                    filePath={filename}
                    showSource={showSource}
                    wrapLines={wrapLines}
                />
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

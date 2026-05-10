"use client";

import React from "react";
import Link from "next/link";
import Image from "next/image";
import { PrismLight as SyntaxHighlighter } from "react-syntax-highlighter";
import { formatDistanceToNowStrict } from "date-fns";
import { shortLocale } from "@/lib/utils";
import { gitarenaTheme, detectLanguage } from "@/components/code-block";

export interface BlameHunk {
    commitId: string;
    authorName: string;
    authorUid: string | null;
    authorEmail: string;
    timestamp: number;
    summary: string;
    startLine: number;
    numLines: number;
}

interface BlameViewProps {
    user: string;
    repo: string;
    hunks: BlameHunk[];
    content: string;
    filename: string;
    wrapLines?: boolean;
}

type AstNode =
    | { type: "text"; value: string }
    | { type: "element"; tagName: string; properties: { className?: string[] }; children: AstNode[] };

type AstElement = Extract<AstNode, { type: "element" }>;

function renderNode(node: AstNode, stylesheet: Record<string, React.CSSProperties>, i: number): React.ReactNode {
    if (node.type === "text") {
        return node.value;
    }
    const style = (node.properties?.className ?? []).reduce<React.CSSProperties>(
        (acc, cls) => ({ ...acc, ...(stylesheet[cls] ?? {}) }),
        {}
    );
    return (
        <span key={i} style={style}>
            {node.children.map((child, j) => renderNode(child, stylesheet, j))}
        </span>
    );
}

function BlameAvatar({ authorName, authorUid, size = 16 }: { authorName: string; authorUid: string | null; size?: number }) {
    if (authorUid !== null) {
        return (
            <Image
                src={`/api/avatar/${authorUid}`}
                alt={authorName}
                width={size}
                height={size}
                className="rounded-full shrink-0 object-cover"
                style={{ width: size, height: size }}
            />
        );
    }
    return (
        <div className="rounded-full bg-secondary flex items-center justify-center shrink-0" style={{ width: size, height: size }}>
            <span className="text-[9px] font-medium text-muted-foreground">{authorName[0]?.toUpperCase() ?? "?"}</span>
        </div>
    );
}

/**
 * Age-based color matching git's `color.blame.highlightRecent`:
 * < 1 month → red, 1–12 months → interpolated, > 12 months → blue
 */
function ageColor(timestamp: number): string {
    const now = Date.now() / 1000;
    const ageSeconds = now - timestamp;

    const oneMonth = 30 * 24 * 3600;
    const twelveMonths = 365 * 24 * 3600;

    const red = [248, 113, 113] as const;
    const blue = [96, 165, 250] as const;

    if (ageSeconds <= oneMonth) {
        return `rgb(${red[0]},${red[1]},${red[2]})`;
    }

    if (ageSeconds >= twelveMonths) {
        return `rgb(${blue[0]},${blue[1]},${blue[2]})`;
    }

    const t = (ageSeconds - oneMonth) / (twelveMonths - oneMonth);
    const r = Math.round(red[0] + (blue[0] - red[0]) * t);
    const g = Math.round(red[1] + (blue[1] - red[1]) * t);
    const b = Math.round(red[2] + (blue[2] - red[2]) * t);

    return `rgb(${r},${g},${b})`;
}

export function BlameView({ user, repo, hunks, content, filename, wrapLines = false }: BlameViewProps) {
    const language = detectLanguage(filename);

    const hunkByLine = new Map<number, BlameHunk | null>();
    const hunkForLine = new Map<number, BlameHunk>();
    const posInHunk = new Map<number, { first: boolean; last: boolean }>();

    for (const hunk of hunks) {
        for (let i = 0; i < hunk.numLines; i++) {
            const line = hunk.startLine + i;
            hunkByLine.set(line, i === 0 ? hunk : null);
            hunkForLine.set(line, hunk);
            posInHunk.set(line, {
                first: i === 0,
                last: i === hunk.numLines - 1,
            });
        }
    }

    const uniqueAuthors = Array.from(
        hunks
            .reduce((map, hunk) => {
                if (!map.has(hunk.authorName)) {
                    map.set(hunk.authorName, { authorName: hunk.authorName, authorUid: hunk.authorUid });
                }
                return map;
            }, new Map<string, { authorName: string; authorUid: string | null }>())
            .values()
    );

    return (
        <SyntaxHighlighter
            language={language}
            style={gitarenaTheme}
            PreTag="div"
            renderer={({ rows, stylesheet }) => (
                <div className={`font-mono text-sm leading-relaxed pr-6 [font-variant-ligatures:none] ${wrapLines ? "" : "min-w-max"}`}>
                    <div className="flex items-center justify-between mb-3 ml-4 py-3 pr-6 border-b border-border/30">
                        <div className="flex items-center gap-3">
                            <div className="w-1 mr-3 shrink-0" />
                            <div className="flex items-center gap-2 text-sm text-muted-foreground/50 select-none">
                                <span>older</span>
                                <div
                                    className="h-2.5 w-36 rounded-full"
                                    style={{ background: "linear-gradient(to right, #60a5fa, #f87171)" }}
                                />
                                <span>newer</span>
                            </div>
                        </div>
                        <div className="flex items-center gap-2 select-none">
                            <span className="text-sm text-muted-foreground/50">
                                {uniqueAuthors.length} {uniqueAuthors.length === 1 ? "contributor" : "contributors"}
                            </span>
                            <div className="flex">
                                {uniqueAuthors.slice(0, 5).map((author, idx) => (
                                    <div
                                        key={author.authorName}
                                        className="h-5 w-5 rounded-full ring-2 ring-background overflow-hidden shrink-0"
                                        style={{ marginLeft: idx === 0 ? 0 : "-6px", zIndex: uniqueAuthors.length - idx }}
                                        title={author.authorName}
                                    >
                                        <BlameAvatar authorName={author.authorName} authorUid={author.authorUid} size={20} />
                                    </div>
                                ))}
                            </div>
                        </div>
                    </div>
                    {(rows as AstElement[]).map((row, i) => {
                        const lineNumber = i + 1;
                        const hunk = hunkByLine.get(lineNumber) ?? null;
                        const lineHunk = hunkForLine.get(lineNumber);
                        const pos = posInHunk.get(lineNumber);
                        const barColor = lineHunk ? ageColor(lineHunk.timestamp) : "transparent";

                        const barRadius = pos
                            ? pos.first && pos.last
                                ? "2px"
                                : pos.first
                                  ? "2px 2px 0 0"
                                  : pos.last
                                    ? "0 0 2px 2px"
                                    : "0"
                            : "0";

                        const isHunkStart = pos?.first === true && lineNumber > 1;
                        const borderClass = pos?.first ? "border-t border-border" : "";

                        return (
                            <div key={i} className={`flex hover:bg-accent/30 group${isHunkStart ? " mt-1" : ""} ${borderClass}`}>
                                <div
                                    className="w-1 shrink-0 mr-3 self-stretch"
                                    style={{ backgroundColor: barColor, borderRadius: barRadius }}
                                />

                                <div className="w-72 shrink-0 pr-4 border-r border-border/50 mr-4 overflow-hidden py-0.5">
                                    {hunk ? (
                                        <div className="flex flex-col gap-0.5">
                                            <div className="flex items-center gap-1.5 min-w-0">
                                                <Link
                                                    href={`/${user}/${repo}/commit/${hunk.commitId}`}
                                                    className="font-mono text-xs text-muted-foreground hover:text-foreground transition-colors shrink-0"
                                                >
                                                    {hunk.commitId.slice(0, 7)}
                                                </Link>
                                                <BlameAvatar authorName={hunk.authorName} authorUid={hunk.authorUid} />
                                                <span className="text-xs text-muted-foreground truncate">{hunk.authorName}</span>
                                            </div>
                                            <div className="flex items-center gap-1.5 min-w-0">
                                                <span className="text-xs text-muted-foreground/60 truncate flex-1">{hunk.summary}</span>
                                                <span className="text-xs text-muted-foreground/40 shrink-0">
                                                    {formatDistanceToNowStrict(new Date(hunk.timestamp * 1000), {
                                                        locale: shortLocale,
                                                        addSuffix: true,
                                                    })}
                                                </span>
                                            </div>
                                        </div>
                                    ) : (
                                        <div className="w-full" />
                                    )}
                                </div>

                                <span className="w-10 shrink-0 text-right pr-4 text-muted-foreground/40 select-none group-hover:text-muted-foreground/60 py-0.5">
                                    {lineNumber}
                                </span>

                                <span className={`py-0.5 ${wrapLines ? "whitespace-pre-wrap break-all" : "whitespace-pre"}`}>
                                    {row.children.map((node, j) => renderNode(node, stylesheet, j))}
                                </span>
                            </div>
                        );
                    })}
                </div>
            )}
        >
            {content}
        </SyntaxHighlighter>
    );
}

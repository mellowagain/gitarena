"use client";

import React from "react";
import { Prism as SyntaxHighlighter } from "react-syntax-highlighter";
import { AlertCircle } from "lucide-react";
import useSWR from "swr";
import * as linguistLanguages from "linguist-languages";
import type { Language } from "linguist-languages";

export const gitarenaTheme: Record<string, React.CSSProperties> = {
    'code[class*="language-"]': { color: "var(--foreground)", background: "none" },
    'pre[class*="language-"]': { color: "var(--foreground)", background: "none", margin: 0 },
    comment: { color: "var(--muted-foreground)", fontStyle: "italic" },
    prolog: { color: "var(--muted-foreground)" },
    doctype: { color: "var(--muted-foreground)" },
    cdata: { color: "var(--muted-foreground)" },
    punctuation: { color: "var(--muted-foreground)" },
    namespace: { opacity: 0.7 },
    keyword: { color: "#60a5fa" },
    "control-flow": { color: "#60a5fa" },
    operator: { color: "#60a5fa" },
    builtin: { color: "#60a5fa" },
    tag: { color: "#60a5fa" },
    property: { color: "#60a5fa" },
    selector: { color: "#60a5fa" },
    string: { color: "#fbbf24" },
    char: { color: "#fbbf24" },
    url: { color: "#fbbf24" },
    regex: { color: "#fbbf24" },
    "attr-value": { color: "#fbbf24" },
    "attr-name": { color: "#fbbf24" },
    number: { color: "#c084fc" },
    boolean: { color: "#c084fc" },
    constant: { color: "#c084fc" },
    function: { color: "#c084fc" },
    "class-name": { color: "#c084fc" },
    inserted: { color: "#86efac" },
    deleted: { color: "#f87171" },
    important: { color: "#f87171", fontWeight: "bold" },
    bold: { fontWeight: "bold" },
    italic: { fontStyle: "italic" },
};

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

const PRISM_NAME_OVERRIDES: Record<string, string> = {
    "c++": "cpp",
    "c#": "csharp",
    shell: "bash",
};

const TYPE_ORDER: Record<string, number> = { programming: 0, markup: 1, data: 2, prose: 3 };

function buildExtMap(): Map<string, string> {
    const byExt = new Map<string, Language[]>();

    for (const lang of Object.values(linguistLanguages) as Language[]) {
        for (const ext of lang.extensions ?? []) {
            const e = ext.slice(1).toLowerCase();
            const bucket = byExt.get(e);
            if (bucket) {
                bucket.push(lang);
            } else {
                byExt.set(e, [lang]);
            }
        }
    }

    const map = new Map<string, string>();

    for (const [ext, candidates] of byExt) {
        candidates.sort((a, b) => {
            const aAlias = (a.aliases ?? []).includes(ext) ? 0 : 1;
            const bAlias = (b.aliases ?? []).includes(ext) ? 0 : 1;
            if (aAlias !== bAlias) return aAlias - bAlias;

            const aPrimary = a.extensions?.[0]?.slice(1).toLowerCase() === ext ? 0 : 1;
            const bPrimary = b.extensions?.[0]?.slice(1).toLowerCase() === ext ? 0 : 1;
            if (aPrimary !== bPrimary) return aPrimary - bPrimary;

            return (TYPE_ORDER[a.type] ?? 9) - (TYPE_ORDER[b.type] ?? 9);
        });

        const name = candidates[0].name.toLowerCase();
        map.set(ext, PRISM_NAME_OVERRIDES[name] ?? name);
    }

    return map;
}

const EXT_TO_LANGUAGE = buildExtMap();

const textFetcher = (url: string) =>
    fetch(url).then((res) => {
        if (!res.ok) throw new Error(res.statusText);
        return res.text();
    });

export function CodeBlockContent({ content, filename, wrapLines = false }: { content: string; filename: string; wrapLines?: boolean }) {
    const ext = filename.split(".").pop()?.toLowerCase() ?? "";
    const language = EXT_TO_LANGUAGE.get(ext) ?? ext;

    return (
        <SyntaxHighlighter
            language={language}
            style={gitarenaTheme}
            PreTag="div"
            renderer={({ rows, stylesheet }) => (
                <div className={`font-mono text-sm leading-relaxed pr-6 [font-variant-ligatures:none] ${wrapLines ? "" : "min-w-max"}`}>
                    {(rows as AstElement[]).map((row, i) => (
                        <div key={i} className="flex hover:bg-accent/30 group py-0.5">
                            <span className="w-14 shrink-0 text-right pr-4 text-muted-foreground/40 select-none group-hover:text-muted-foreground/60 sticky left-0 bg-background">
                                {i + 1}
                            </span>
                            <span className={wrapLines ? "whitespace-pre-wrap break-all" : "whitespace-pre"}>
                                {row.children.map((node, j) => renderNode(node, stylesheet, j))}
                            </span>
                        </div>
                    ))}
                </div>
            )}
        >
            {content}
        </SyntaxHighlighter>
    );
}

export function CodeBlockSkeleton() {
    return (
        <div className="font-mono text-sm leading-relaxed pr-6 animate-pulse">
            {[52, 38, 65, 44, 71, 30, 58, 42, 67, 35, 60, 48, 73, 40, 55].map((w, i) => (
                <div key={i} className="flex items-center min-h-[1.625em] py-0.5">
                    <span className="w-14 shrink-0 flex justify-end pr-4">
                        <span className="h-[1em] w-5 rounded bg-accent" />
                    </span>
                    <span className="h-[1em] rounded bg-accent" style={{ width: `${w}%` }} />
                </div>
            ))}
        </div>
    );
}

/*

TODO:
- use actual error display component on line 191 instead of hand rolling it
- fetch latest commit and all branches and commit count for branch to show in sidebar top left
- change file endpoint from the raw ~blob to an actual json endpoint that also returns the file size and last commit info
- add history button to see history for a file

 */

export function CodeBlock({
    user,
    repo,
    branch,
    filename,
    wrapLines = false,
}: {
    user: string;
    repo: string;
    branch: string | null;
    filename: string;
    wrapLines?: boolean;
}) {
    const url = branch ? `http://localhost:8080/${user}/${repo}/tree/${branch}/~blob/${filename}` : null;

    const { data: content, error, isLoading } = useSWR<string>(url, textFetcher);

    if (isLoading || !branch) {
        return <CodeBlockSkeleton />;
    }

    if (error) {
        return (
            <div className="flex flex-col items-center justify-center h-full gap-3 text-muted-foreground py-16">
                <AlertCircle className="h-5 w-5 text-destructive" />
                <p className="text-sm font-medium">Failed to load file</p>
                <p className="text-xs font-mono text-muted-foreground/60">{error.message}</p>
            </div>
        );
    }

    return <CodeBlockContent content={content ?? ""} filename={filename} wrapLines={wrapLines} />;
}

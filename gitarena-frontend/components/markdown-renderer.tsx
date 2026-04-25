"use client";

import React from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Prism as SyntaxHighlighter } from "react-syntax-highlighter";
import { gitarenaTheme } from "@/components/code-block";
import { CodeBlockContent } from "@/components/code-block";

const MARKDOWN_EXTENSIONS = new Set(["md", "mdx", "markdown"]);

export function isMarkdown(filename: string): boolean {
    return MARKDOWN_EXTENSIONS.has(filename.split(".").pop()?.toLowerCase() ?? "");
}

function proxyImageUrl(url: string): string {
    if (/^data:image\//.test(url)) {
        return url;
    }
    const hex = Array.from(url)
        .map((c) => c.charCodeAt(0).toString(16).padStart(2, "0"))
        .join("");
    return `/api/proxy/${hex}`;
}

export function MarkdownRenderer({
    content,
    fileName,
    showSource = false,
    wrapLines = false,
}: {
    content: string;
    fileName: string;
    showSource?: boolean;
    wrapLines?: boolean;
}) {
    if (showSource) {
        return <CodeBlockContent content={content} filename={fileName} wrapLines={wrapLines} />;
    }

    return (
        <div className="p-8 space-y-4 text-sm leading-relaxed">
            <ReactMarkdown
                remarkPlugins={[remarkGfm]}
                components={{
                    h1: ({ children }) => <h1 className="text-2xl font-semibold text-foreground mt-6 first:mt-0 mb-2">{children}</h1>,
                    h2: ({ children }) => <h2 className="text-lg font-semibold text-foreground mt-5 first:mt-0 mb-1.5">{children}</h2>,
                    h3: ({ children }) => <h3 className="text-base font-semibold text-foreground mt-4 first:mt-0 mb-1">{children}</h3>,
                    p: ({ children }) => <p className="text-muted-foreground">{children}</p>,
                    a: ({ href, children }) => (
                        <a href={href} className="text-blue-400 hover:text-blue-300 underline underline-offset-2">
                            {children}
                        </a>
                    ),
                    ul: ({ children }) => <ul className="list-disc list-inside text-muted-foreground space-y-1">{children}</ul>,
                    ol: ({ children }) => <ol className="list-decimal list-inside text-muted-foreground space-y-1">{children}</ol>,
                    li: ({ children }) => <li>{children}</li>,
                    pre: ({ children }) => <>{children}</>,
                    code: ({ className, children }) => {
                        const match = /language-(\w+)/.exec(className ?? "");
                        if (match) {
                            return (
                                <SyntaxHighlighter
                                    language={match[1]}
                                    style={gitarenaTheme}
                                    customStyle={{ borderRadius: "0.375rem", fontSize: "0.875rem", border: "1px solid var(--border)" }}
                                    PreTag="div"
                                >
                                    {String(children).replace(/\n$/, "")}
                                </SyntaxHighlighter>
                            );
                        }
                        return <code className="font-mono text-xs bg-secondary px-1.5 py-0.5 rounded text-foreground">{children}</code>;
                    },
                    blockquote: ({ children }) => (
                        <blockquote className="border-l-2 border-border pl-4 text-muted-foreground italic">{children}</blockquote>
                    ),
                    hr: () => <hr className="border-border" />,
                    table: ({ children }) => (
                        <div className="overflow-x-auto">
                            <table className="w-full text-sm border-collapse">{children}</table>
                        </div>
                    ),
                    th: ({ children }) => (
                        <th className="border border-border px-4 py-2 text-left font-medium text-foreground bg-secondary">{children}</th>
                    ),
                    td: ({ children }) => <td className="border border-border px-4 py-2 text-muted-foreground">{children}</td>,
                    img: ({ src, alt }) => (
                        // eslint-disable-next-line @next/next/no-img-element
                        <img
                            src={typeof src === "string" ? proxyImageUrl(src) : undefined}
                            alt={alt}
                            className="inline-block align-middle"
                        />
                    ),
                }}
            >
                {content}
            </ReactMarkdown>
        </div>
    );
}

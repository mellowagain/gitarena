"use client";

import React from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import remarkFrontmatter from "remark-frontmatter";
import { parse as parseYaml } from "yaml";
import rehypeRaw from "rehype-raw";
import rehypeSanitize, { defaultSchema } from "rehype-sanitize";
import { Prism as SyntaxHighlighter } from "react-syntax-highlighter";
import { gitarenaTheme } from "@/components/code-block";
import { CodeBlockContent } from "@/components/code-block";
import { Checkbox } from "@/components/ui/checkbox";

const MARKDOWN_EXTENSIONS = new Set(["md", "mdx", "markdown"]);

// Only allow specific HTML tags that are safe and useful in markdown content.
// rehype-raw passes all HTML through; rehype-sanitize then strips everything not in this list.
// We extend defaultSchema (which already allows h1-h6, ul, ol, li, p, a, code, etc.) with
// additional tags useful in markdown: details/summary for collapsibles, kbd for keyboard keys, etc.
const sanitizeSchema = {
    ...defaultSchema,
    tagNames: [...(defaultSchema.tagNames ?? []), "frontmatter", "mdxesm", "details", "summary", "kbd", "sub", "sup", "ins", "mark"],
    attributes: {
        ...defaultSchema.attributes,
        div: [...(defaultSchema.attributes?.div ?? []), "align"],
        p: [...(defaultSchema.attributes?.p ?? []), "align"],
        td: [...(defaultSchema.attributes?.td ?? []), "colspan", "rowspan"],
        th: [...(defaultSchema.attributes?.th ?? []), "colspan", "rowspan"],
    },
    strip: ["script", "style"],
};

export function isMarkdown(filename: string): boolean {
    return MARKDOWN_EXTENSIONS.has(filename.split(".").pop()?.toLowerCase() ?? "");
}

// After rehype-raw, a <details> block in markdown looks like:
//   details element (with summary child)
//   list/paragraph siblings  ← these should be inside <details>
//   </details> becomes a raw node or nothing
//
// This plugin re-parents those siblings into the preceding <details> element.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
function rehypeDetails(): (tree: any) => void {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    return (tree: any): void => {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        function processChildren(children: any[]): any[] {
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            const result: any[] = [];
            let i = 0;
            while (i < children.length) {
                const node = children[i];
                if (node.type === "element" && node.tagName === "details") {
                    i++;
                    while (i < children.length) {
                        const sibling = children[i];
                        // Stop at another details or heading element
                        if (sibling.type === "element" && ["details", "h1", "h2", "h3", "h4", "h5", "h6"].includes(sibling.tagName)) {
                            break;
                        }
                        // Stop at a raw </details> closing tag
                        if (sibling.type === "raw" && /^\s*<\/details>\s*$/i.test(sibling.value)) {
                            i++;
                            break;
                        }
                        node.children.push(sibling);
                        i++;
                    }
                    result.push(node);
                } else {
                    if (node.type === "element" && node.children) {
                        node.children = processChildren(node.children);
                    }
                    result.push(node);
                    i++;
                }
            }
            return result;
        }
        tree.children = processChildren(tree.children);
    };
}

// Replaces the leading YAML frontmatter node (from remark-frontmatter) with a <frontmatter> element holding the raw YAML.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
function remarkFrontmatterElement(): (tree: any) => void {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    return (tree: any): void => {
        const first = tree.children[0];
        if (first?.type === "yaml") {
            tree.children[0] = {
                type: "paragraph",
                children: [],
                position: first.position,
                data: { hName: "frontmatter", hChildren: [{ type: "text", value: first.value }] },
            };
        }
    };
}

// MDX treats top-level lines starting with `import` / `export` as ESM, not prose.
// Turn those paragraphs into <mdxesm> elements holding the original source text.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
function remarkMdxEsm(): (tree: any, file: any) => void {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    return (tree: any, file: any): void => {
        const source = String(file);
        for (const node of tree.children) {
            if (node.type !== "paragraph") {
                continue;
            }
            const value = source.slice(node.position.start.offset, node.position.end.offset);
            if (/^(import|export)\s/.test(value)) {
                node.data = { hName: "mdxesm", hChildren: [{ type: "text", value }] };
            }
        }
    };
}

function FrontmatterValue({ value }: { value: unknown }) {
    if (Array.isArray(value)) {
        return (
            <div className="flex flex-wrap gap-1.5">
                {value.map((item, i) => (
                    <span key={i} className="font-mono text-xs bg-secondary border border-border rounded px-1.5 py-0.5 text-foreground">
                        {typeof item === "object" && item !== null ? JSON.stringify(item) : String(item)}
                    </span>
                ))}
            </div>
        );
    }
    if (typeof value === "object" && value !== null) {
        return <code className="font-mono text-xs text-foreground break-all">{JSON.stringify(value)}</code>;
    }
    if (typeof value === "string") {
        return <span className="text-foreground">{value}</span>;
    }
    return <code className="font-mono text-xs text-foreground">{String(value)}</code>;
}

function Frontmatter({ yaml }: { yaml: string }) {
    let data: unknown;
    try {
        data = parseYaml(yaml);
    } catch {
        // Invalid YAML is shown as-is below
    }

    if (typeof data !== "object" || data === null || Array.isArray(data)) {
        return (
            <SyntaxHighlighter language="yaml" style={gitarenaTheme} customStyle={codeBlockStyle} PreTag="div">
                {yaml}
            </SyntaxHighlighter>
        );
    }

    return (
        <dl className="grid grid-cols-[max-content_1fr] gap-px bg-border border border-border rounded-md overflow-hidden mb-6">
            {Object.entries(data).map(([key, value]) => (
                <React.Fragment key={key}>
                    <dt className="bg-card px-4 py-2 font-mono text-xs text-muted-foreground flex items-center">{key}</dt>
                    <dd className="bg-card px-4 py-2 min-w-0 flex items-center">
                        <FrontmatterValue value={value} />
                    </dd>
                </React.Fragment>
            ))}
        </dl>
    );
}

const codeBlockStyle: React.CSSProperties = {
    borderRadius: "0.375rem",
    fontSize: "0.875rem",
    border: "1px solid var(--border)",
    padding: "1rem 1.25rem",
    marginBottom: "1rem",
    overflowX: "auto",
};

function resolveImageUrl(url: string, user: string, repo: string, branch: string, filePath: string): string {
    if (/^data:image\//.test(url)) {
        return url;
    }
    if (/^https?:\/\//.test(url)) {
        const hex = Array.from(url)
            .map((c) => c.charCodeAt(0).toString(16).padStart(2, "0"))
            .join("");
        return `/api/proxy/${hex}`;
    }
    // Relative path — resolve against the directory of the markdown file, serve raw bytes via blob URL
    const dir = filePath.includes("/") ? filePath.slice(0, filePath.lastIndexOf("/")) : "";
    const resolved = dir ? `${dir}/${url}` : url;
    const backendUrl = process.env.NEXT_PUBLIC_API_URL ?? "";
    return `${backendUrl}/${user}/${repo}/tree/${branch}/~blob/${resolved}`;
}

export function MarkdownRenderer({
    content,
    fileName = "",
    user = "",
    repo = "",
    branch = "",
    filePath = "",
    showSource = false,
    wrapLines = false,
    className = "p-8 space-y-4 text-sm leading-relaxed",
}: {
    content: string;
    fileName?: string;
    user?: string;
    repo?: string;
    branch?: string;
    filePath?: string;
    showSource?: boolean;
    wrapLines?: boolean;
    className?: string;
}) {
    const isMdx = fileName.toLowerCase().endsWith(".mdx");
    // Frontmatter only exists in files, not in issue comments or release notes
    const remarkPlugins = [remarkGfm, ...(fileName ? [remarkFrontmatter, remarkFrontmatterElement] : []), ...(isMdx ? [remarkMdxEsm] : [])];

    if (showSource) {
        return <CodeBlockContent content={content} filename={fileName} wrapLines={wrapLines} />;
    }

    return (
        <div className={className}>
            <ReactMarkdown
                remarkPlugins={remarkPlugins}
                rehypePlugins={[rehypeRaw, rehypeDetails, [rehypeSanitize, sanitizeSchema]]}
                components={{
                    // Custom elements emitted by remarkFrontmatterElement / remarkMdxEsm
                    ...({
                        frontmatter: ({ children }: { children?: React.ReactNode }) => <Frontmatter yaml={String(children)} />,
                        mdxesm: ({ children }: { children?: React.ReactNode }) => (
                            <SyntaxHighlighter
                                language="jsx"
                                style={gitarenaTheme}
                                customStyle={{
                                    ...codeBlockStyle,
                                    border: "1px dashed var(--border)",
                                    background: "transparent",
                                    padding: "0.5rem 1rem",
                                    fontSize: "0.8rem",
                                    opacity: 0.7,
                                }}
                                PreTag="div"
                            >
                                {String(children)}
                            </SyntaxHighlighter>
                        ),
                    } as object),
                    h1: ({ children }) => <h1 className="text-2xl font-semibold text-foreground mt-6 first:mt-0 mb-2">{children}</h1>,
                    h2: ({ children }) => <h2 className="text-lg font-semibold text-foreground mt-5 first:mt-0 mb-1.5">{children}</h2>,
                    h3: ({ children }) => <h3 className="text-base font-semibold text-foreground mt-4 first:mt-0 mb-1">{children}</h3>,
                    p: ({ children }) => <p className="text-muted-foreground">{children}</p>,
                    a: ({ href, children }) => (
                        <a href={href} className="text-blue-400 hover:text-blue-300 underline underline-offset-2">
                            {children}
                        </a>
                    ),
                    ul: ({ children }) => <ul className="list-disc list-outside pl-5 text-muted-foreground space-y-1">{children}</ul>,
                    ol: ({ children }) => <ol className="list-decimal list-outside pl-5 text-muted-foreground space-y-1">{children}</ol>,
                    li: ({ className, children }) => (
                        <li className={className?.includes("task-list-item") ? "list-none [&_[data-slot=checkbox]]:-ml-5" : undefined}>
                            {children}
                        </li>
                    ),
                    input: ({ type, checked }) =>
                        type === "checkbox" ? <Checkbox checked={checked} className="mr-2 align-middle" aria-label="Task" /> : null,
                    pre: ({ children }) => <>{children}</>,
                    code: ({ className, children }) => {
                        const match = /language-(\w+)/.exec(className ?? "");
                        const isBlock = match || String(children).includes("\n");
                        if (isBlock) {
                            return (
                                <SyntaxHighlighter
                                    language={match ? match[1] : "text"}
                                    style={gitarenaTheme}
                                    customStyle={codeBlockStyle}
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
                    kbd: ({ children }) => (
                        <kbd className="font-mono text-xs bg-secondary border border-border rounded px-1.5 py-0.5 shadow-[0_1px_0_1px] shadow-border text-foreground">
                            {children}
                        </kbd>
                    ),
                    mark: ({ children }) => (
                        <mark className="bg-yellow-200 text-yellow-900 dark:bg-yellow-400/30 dark:text-yellow-200 rounded px-0.5">
                            {children}
                        </mark>
                    ),
                    img: ({ src, alt }) => (
                        // eslint-disable-next-line @next/next/no-img-element
                        <img
                            src={typeof src === "string" ? resolveImageUrl(src, user, repo, branch, filePath) : undefined}
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

"use client";

import React from "react";
import { PrismLight as SyntaxHighlighter } from "react-syntax-highlighter";
import * as prismLanguages from "react-syntax-highlighter/dist/esm/languages/prism/index.js";
import * as linguistLanguages from "linguist-languages";
import type { Language } from "linguist-languages";

// Register all bundled Prism languages. Some grammars depend on others and may fail if registered
// out of order; those are silently skipped (they're uncommon and will fall back to plain text).
for (const [name, syntax] of Object.entries(prismLanguages)) {
    try {
        SyntaxHighlighter.registerLanguage(name, syntax as Parameters<typeof SyntaxHighlighter.registerLanguage>[1]);
    } catch {
        // dependency not yet registered — skip
    }
}

// Register Svelte grammar (markup-based with Svelte-specific block/expression patterns)
function svelte(Prism: {
    languages: {
        svelte?: unknown;
        extend: (base: string, def: object) => object;
    };
}) {
    if (Prism.languages.svelte) {
        return;
    }
    const blocks = "(if|else if|await|then|catch|each|html|debug)";
    Prism.languages.svelte = Prism.languages.extend("markup", {
        each: {
            pattern: new RegExp("{[#/]each(?:(?:\\{(?:(?:\\{(?:[^{}])*\\})|(?:[^{}]))*\\})|(?:[^{}]))*}"),
            inside: {
                keyword: /[#/]each|as/,
                punctuation: /{|}/,
            },
        },
        block: {
            pattern: new RegExp("{[#:/@]" + blocks + "(?:(?:\\{(?:(?:\\{(?:[^{}])*\\})|(?:[^{}]))*\\})|(?:[^{}]))*}"),
            inside: {
                punctuation: /^{|}$/,
                keyword: [new RegExp("[#:/@]" + blocks + "( )*"), /as/, /then/],
            },
        },
        tag: {
            pattern:
                /<\/?(?!\d)[^\s>\/=$<%]+(?:\s(?:\s*[^\s>\/=]+(?:\s*=\s*(?:(?:"[^"]*"|'[^']*'|[^\s'">=]+(?=[\s>]))|(?:"[^"]*"|'[^']*'|{[\s\S]+?}(?=[\s/>])))|(?=[\s/>])))+)?\s*\/?>/i,
            greedy: true,
            inside: {
                tag: { pattern: /^<\/?[^\s>\/]+/i, inside: { punctuation: /^<\/?/, namespace: /^[^\s>\/:]+:/ } },
                "attr-value": {
                    pattern: /=\s*(?:"[^"]*"|'[^']*'|[^\s'">=]+)/i,
                    inside: { punctuation: [/^=/, { pattern: /^(\s*)["']|["']$/, lookbehind: true }] },
                },
                punctuation: /\/?>/,
                "attr-name": { pattern: /[^\s>\/]+/, inside: { namespace: /^[^\s>\/:]+:/ } },
            },
        },
        "language-javascript": {
            pattern: /\{(?:(?:\{(?:(?:\{(?:[^{}])*\})|(?:[^{}]))*\})|(?:[^{}]))*\}/,
            lookbehind: true,
        },
    });
}
svelte.displayName = "svelte";
svelte.aliases = [] as string[];
SyntaxHighlighter.registerLanguage("svelte", svelte as Parameters<typeof SyntaxHighlighter.registerLanguage>[1]);

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
    html: "markup",
    sqlpl: "sql",
    tsql: "sql",
    plpgsql: "sql",
    // filename-detected languages whose linguist names differ from Prism's
    "vim script": "vim",
    batchfile: "batch",
    starlark: "python",
    snakemake: "python",
    "alpine abuild": "bash",
    "ant build system": "xml",
    soong: "json",
    xmake: "lua",
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
            if (aAlias !== bAlias) {
                return aAlias - bAlias;
            }

            const aPrimary = a.extensions?.[0]?.slice(1).toLowerCase() === ext ? 0 : 1;
            const bPrimary = b.extensions?.[0]?.slice(1).toLowerCase() === ext ? 0 : 1;
            if (aPrimary !== bPrimary) {
                return aPrimary - bPrimary;
            }

            return (TYPE_ORDER[a.type] ?? 9) - (TYPE_ORDER[b.type] ?? 9);
        });

        const name = candidates[0].name.toLowerCase();
        map.set(ext, PRISM_NAME_OVERRIDES[name] ?? name);
    }

    return map;
}

function buildFilenameMap(): Map<string, string> {
    const byFilename = new Map<string, Language[]>();

    for (const lang of Object.values(linguistLanguages) as Language[]) {
        for (const fn of lang.filenames ?? []) {
            const bucket = byFilename.get(fn);
            if (bucket) {
                bucket.push(lang);
            } else {
                byFilename.set(fn, [lang]);
            }
        }
    }

    const map = new Map<string, string>();

    for (const [fn, candidates] of byFilename) {
        candidates.sort((a, b) => {
            const aAlias = (a.aliases ?? []).map((s) => s.toLowerCase()).includes(fn.toLowerCase()) ? 0 : 1;
            const bAlias = (b.aliases ?? []).map((s) => s.toLowerCase()).includes(fn.toLowerCase()) ? 0 : 1;
            if (aAlias !== bAlias) {
                return aAlias - bAlias;
            }

            const aPrimary = a.filenames?.[0] === fn ? 0 : 1;
            const bPrimary = b.filenames?.[0] === fn ? 0 : 1;
            if (aPrimary !== bPrimary) {
                return aPrimary - bPrimary;
            }

            return (TYPE_ORDER[a.type] ?? 9) - (TYPE_ORDER[b.type] ?? 9);
        });

        const name = candidates[0].name.toLowerCase();
        map.set(fn, PRISM_NAME_OVERRIDES[name] ?? name);
    }

    return map;
}

const EXT_TO_LANGUAGE = buildExtMap();

const FILENAME_TO_LANGUAGE = buildFilenameMap();

export function detectLanguage(filename: string): string {
    const basename = filename.split("/").pop() ?? filename;
    const ext = basename.split(".").pop()?.toLowerCase() ?? "";
    return FILENAME_TO_LANGUAGE.get(basename) ?? EXT_TO_LANGUAGE.get(ext) ?? ext;
}

export function CodeBlockContent({ content, filename, wrapLines = false }: { content: string; filename: string; wrapLines?: boolean }) {
    const basename = filename.split("/").pop() ?? filename;
    const ext = basename.split(".").pop()?.toLowerCase() ?? "";
    const language = FILENAME_TO_LANGUAGE.get(basename) ?? EXT_TO_LANGUAGE.get(ext) ?? ext;

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

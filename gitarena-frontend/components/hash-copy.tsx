"use client";

import { useState } from "react";
import { Check, Copy } from "lucide-react";

export function HashCopy({ shortHash, fullHash }: { shortHash: string; fullHash: string }) {
    const [copied, setCopied] = useState(false);
    function copy(e: React.MouseEvent) {
        e.preventDefault();
        navigator.clipboard.writeText(fullHash);
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
    }
    return (
        <button
            onClick={copy}
            title="Copy full hash"
            className="group/hash flex items-center gap-1.5 font-mono text-[11px] text-muted-foreground hover:text-foreground transition-colors px-2 py-1 rounded bg-secondary border border-border/60 hover:border-border shrink-0"
        >
            {copied ? (
                <Check className="h-3 w-3 text-green-500 shrink-0" />
            ) : (
                <Copy className="h-3 w-3 hidden group-hover/hash:block shrink-0" />
            )}
            {shortHash}
        </button>
    );
}

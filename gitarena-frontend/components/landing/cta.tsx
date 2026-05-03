"use client";

import { useState } from "react";
import Link from "next/link";
import { Button } from "@/components/ui/button";
import { ArrowRight, Terminal, Copy, Check } from "lucide-react";

const INSTALL_COMMANDS = "wget https://get.git.mari.zip/docker-compose.yml && docker compose up -d";

export function CTA() {
    const [copied, setCopied] = useState(false);

    async function handleCopy() {
        await navigator.clipboard.writeText(INSTALL_COMMANDS);
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
    }

    return (
        <section id="cta" className="px-6 py-24 border-t border-border">
            <div className="max-w-6xl mx-auto">
                <div className="grid gap-12 lg:grid-cols-2 items-center">
                    {/* Left: Copy */}
                    <div>
                        <h2 className="text-3xl font-bold tracking-tight sm:text-4xl">
                            Ready to take control
                            <br />
                            <span className="text-muted-foreground">of your code?</span>
                        </h2>
                        <p className="mt-4 text-lg text-muted-foreground max-w-lg">
                            Deploy GitArena in minutes. Self-host your repositories with a platform that respects your resources and
                            privacy.
                        </p>

                        <div className="flex flex-wrap items-center gap-3 mt-8">
                            <Link href="/register">
                                <Button size="lg" className="gap-2">
                                    Start with invite code
                                    <ArrowRight className="w-4 h-4" />
                                </Button>
                            </Link>
                            <a href="/docs/quickstart">
                                <Button size="lg" variant="outline" className="gap-2">
                                    <Terminal className="w-4 h-4" />
                                    Install Guide
                                </Button>
                            </a>
                        </div>
                    </div>

                    {/* Right: Terminal */}
                    <div className="rounded-lg border border-border bg-[#0d0d0d] overflow-hidden text-sm font-mono">
                        <div className="flex items-center gap-2 px-4 py-2.5 border-b border-border bg-white/5">
                            <div className="flex gap-1.5">
                                <div className="w-3 h-3 rounded-full bg-red-500/60" />
                                <div className="w-3 h-3 rounded-full bg-yellow-500/60" />
                                <div className="w-3 h-3 rounded-full bg-green-500/60" />
                            </div>
                            <span className="text-xs text-muted-foreground ml-2">bash</span>
                            <button
                                onClick={handleCopy}
                                className="ml-auto text-muted-foreground hover:text-white transition-colors"
                                title="Copy commands"
                            >
                                {copied ? <Check className="w-3.5 h-3.5 text-green-400" /> : <Copy className="w-3.5 h-3.5" />}
                            </button>
                        </div>
                        <div className="p-5 space-y-1 leading-relaxed">
                            <p>
                                <span className="text-muted-foreground">~ $ </span>
                                <span className="text-white">wget https://get.git.mari.zip/docker-compose.yml</span>
                            </p>
                            <p className="mt-3">
                                <span className="text-muted-foreground">~ $ </span>
                                <span className="text-white">docker compose up -d</span>
                            </p>
                            <p className="text-muted-foreground">[+] Running 3/3</p>
                            <p className="text-muted-foreground">
                                &nbsp;✔ Container gitarena <span className="text-green-400">Started</span>
                            </p>
                            <p className="mt-3">
                                <span className="text-muted-foreground">~ $ </span>
                                <span className="animate-pulse">▊</span>
                            </p>
                        </div>
                    </div>
                </div>
            </div>
        </section>
    );
}

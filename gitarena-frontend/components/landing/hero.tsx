"use client";

import { Button } from "@/components/ui/button";
import { ArrowRight, Github, GitBranch, FileText, Folder } from "lucide-react";

export function Hero() {
    return (
        <section className="relative px-6 py-24 lg:py-32">
            <div className="max-w-6xl mx-auto">
                <div className="grid gap-16 lg:grid-cols-2 lg:gap-12 items-center">
                    {/* Left: Copy */}
                    <div>
                        <div className="inline-flex items-center gap-2 px-3 py-1 mb-8 text-sm rounded-full bg-card text-muted-foreground border border-border">
                            <span className="w-1.5 h-1.5 rounded-full bg-foreground animate-pulse" />
                            Now in public beta
                        </div>

                        <h1 className="text-4xl font-bold tracking-tight sm:text-5xl lg:text-6xl">
                            <span className="text-foreground">Ship code</span>
                            <br />
                            <span className="text-muted-foreground">without the bloat.</span>
                        </h1>

                        <p className="mt-6 text-lg leading-relaxed text-muted-foreground max-w-lg">
                            GitArena is a lightweight, self-hosted Git platform with built-in issue tracking and code review. A 50MB binary
                            that runs anywhere.
                        </p>

                        <div className="flex flex-wrap items-center gap-3 mt-8">
                            <Button size="lg" className="gap-2">
                                Get Started
                                <ArrowRight className="w-4 h-4" />
                            </Button>
                            <Button size="lg" variant="outline" className="gap-2">
                                <Github className="w-4 h-4" />
                                View Source
                            </Button>
                        </div>

                        <p className="mt-6 text-sm text-muted-foreground">Free and open source. Deploy in under a minute.</p>
                    </div>

                    {/* Right: Interactive Product Preview */}
                    <div className="relative">
                        <div className="rounded-lg border border-border bg-card overflow-hidden shadow-2xl shadow-black/20">
                            {/* Mock window header */}
                            <div className="flex items-center gap-2 px-4 py-3 border-b border-border bg-secondary/30">
                                <div className="flex gap-1.5">
                                    <div className="w-3 h-3 rounded-full bg-muted-foreground/20" />
                                    <div className="w-3 h-3 rounded-full bg-muted-foreground/20" />
                                    <div className="w-3 h-3 rounded-full bg-muted-foreground/20" />
                                </div>
                                <div className="flex-1 mx-8">
                                    <div className="h-6 rounded bg-secondary/50 flex items-center justify-center">
                                        <span className="text-xs text-muted-foreground font-mono">git.example.com/acme/api</span>
                                    </div>
                                </div>
                            </div>

                            {/* Mock repo content */}
                            <div className="p-4">
                                <div className="flex items-center gap-3 mb-4">
                                    <div className="flex items-center gap-2 px-3 py-1.5 rounded bg-secondary text-xs">
                                        <GitBranch className="w-3.5 h-3.5" />
                                        <span>main</span>
                                    </div>
                                    <span className="text-xs text-muted-foreground">4 branches</span>
                                    <span className="text-xs text-muted-foreground">12 tags</span>
                                </div>

                                {/* File list */}
                                <div className="space-y-1">
                                    {[
                                        { icon: Folder, name: "src", time: "2 hours ago" },
                                        { icon: Folder, name: "tests", time: "1 day ago" },
                                        { icon: FileText, name: "Cargo.toml", time: "3 hours ago" },
                                        { icon: FileText, name: "README.md", time: "5 days ago" },
                                    ].map((file) => (
                                        <div
                                            key={file.name}
                                            className="flex items-center justify-between py-2 px-3 rounded hover:bg-accent/50 group cursor-pointer"
                                        >
                                            <div className="flex items-center gap-3">
                                                <file.icon className="w-4 h-4 text-muted-foreground" />
                                                <span className="text-sm">{file.name}</span>
                                            </div>
                                            <span className="text-xs text-muted-foreground">{file.time}</span>
                                        </div>
                                    ))}
                                </div>
                            </div>
                        </div>

                        {/* Decorative element - subtle gradient overlay */}
                        <div className="absolute -inset-4 -z-10 rounded-2xl bg-gradient-to-b from-muted/20 to-transparent blur-xl opacity-50" />
                    </div>
                </div>
            </div>
        </section>
    );
}

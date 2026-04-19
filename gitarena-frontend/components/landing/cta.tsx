import { Button } from "@/components/ui/button";
import { ArrowRight, Terminal } from "lucide-react";

export function CTA() {
    return (
        <section className="px-6 py-24 border-t border-border">
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
                            Deploy GitArena in minutes. Self-host your repositories with a platform that respects your resources.
                        </p>

                        <div className="flex flex-wrap items-center gap-3 mt-8">
                            <Button size="lg" className="gap-2">
                                Start for Free
                                <ArrowRight className="w-4 h-4" />
                            </Button>
                            <Button size="lg" variant="outline" className="gap-2">
                                <Terminal className="w-4 h-4" />
                                Install Guide
                            </Button>
                        </div>
                    </div>

                    {/* Right: Terminal */}
                    <div className="rounded-lg border border-border bg-card overflow-hidden">
                        <div className="flex items-center gap-2 px-4 py-3 border-b border-border bg-secondary/30">
                            <div className="flex gap-1.5">
                                <div className="w-3 h-3 rounded-full bg-muted-foreground/20" />
                                <div className="w-3 h-3 rounded-full bg-muted-foreground/20" />
                                <div className="w-3 h-3 rounded-full bg-muted-foreground/20" />
                            </div>
                            <span className="text-xs text-muted-foreground font-mono ml-2">terminal</span>
                        </div>
                        <div className="p-4 font-mono text-sm">
                            <p className="text-muted-foreground">
                                <span className="text-foreground">$</span> docker run -d -p 3000:3000 gitarena/gitarena:latest
                            </p>
                            <p className="text-muted-foreground/50 mt-2">
                                <span className="text-muted-foreground">$</span> curl localhost:3000/health
                            </p>
                            <p className="text-muted-foreground/70 mt-1">
                                {"{"}&quot;status&quot;:&quot;ok&quot;,&quot;version&quot;:&quot;0.1.0&quot;{"}"}
                            </p>
                        </div>
                    </div>
                </div>
            </div>
        </section>
    );
}

import { GitBranch, MessageSquare, GitPullRequest, Shield, Zap, Server } from "lucide-react";

const features = [
    {
        icon: GitBranch,
        title: "Version Control",
        description: "Full Git support with a web interface for browsing commits, branches, and file history.",
    },
    {
        icon: MessageSquare,
        title: "Issue Tracking",
        description: "Built-in issue management with labels, milestones, and assignments.",
    },
    {
        icon: GitPullRequest,
        title: "Code Review",
        description: "Inline comments, review requests, and merge checks for quality control.",
    },
    {
        icon: Zap,
        title: "Blazing Fast",
        description: "Instant page loads and real-time updates, even with large repositories.",
    },
    {
        icon: Server,
        title: "Self-Hosted",
        description: "Your code, your server. Full control over your data and infrastructure.",
    },
    {
        icon: Shield,
        title: "Cross-Platform",
        description: "Runs on Linux, macOS, Windows, and ARM. Single binary deployment.",
    },
];

export function Features() {
    return (
        <section id="features" className="px-6 py-24 border-t border-border">
            <div className="max-w-6xl mx-auto">
                <div className="max-w-xl mb-16">
                    <p className="mb-3 text-xs font-medium uppercase tracking-widest text-muted-foreground">Features</p>
                    <h2 className="text-3xl font-bold tracking-tight sm:text-4xl">
                        Everything you need.
                        <br />
                        <span className="text-muted-foreground">Nothing you don&apos;t.</span>
                    </h2>
                </div>

                {/* Asymmetric 2-column layout instead of 3-column grid */}
                <div className="grid gap-px bg-border rounded-lg overflow-hidden md:grid-cols-2">
                    {features.map((feature) => (
                        <div key={feature.title} className="p-6 bg-card hover:bg-accent/30">
                            <feature.icon className="w-5 h-5 text-foreground mb-4" />
                            <h3 className="text-base font-semibold mb-2">{feature.title}</h3>
                            <p className="text-sm leading-relaxed text-muted-foreground">{feature.description}</p>
                        </div>
                    ))}
                </div>
            </div>
        </section>
    );
}

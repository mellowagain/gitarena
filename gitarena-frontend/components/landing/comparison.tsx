import { Check, X } from "lucide-react";

const comparisons = [
    { feature: "Self-hosted", gitarena: true, gitlab: true, github: false, gitea: true },
    { feature: "Lightweight (<100MB)", gitarena: true, gitlab: false, github: "n/a", gitea: true },
    { feature: "ARM64 support", gitarena: true, gitlab: "partial", github: "n/a", gitea: true },
    { feature: "Built-in CI/CD", gitarena: "roadmap", gitlab: true, github: true, gitea: "partial" },
    { feature: "Issue tracking", gitarena: true, gitlab: true, github: true, gitea: true },
    { feature: "Code review", gitarena: true, gitlab: true, github: true, gitea: true },
    { feature: "Single binary deploy", gitarena: true, gitlab: false, github: "n/a", gitea: true },
    { feature: "Low memory usage", gitarena: true, gitlab: false, github: "n/a", gitea: true },
];

function StatusIcon({ status }: { status: boolean | string }) {
    if (status === true) {
        return <Check className="w-4 h-4 text-foreground" />;
    }
    if (status === false) {
        return <X className="w-4 h-4 text-muted-foreground/30" />;
    }
    return <span className="text-xs text-muted-foreground">{status}</span>;
}

export function Comparison() {
    return (
        <section id="comparison" className="px-6 py-24">
            <div className="max-w-6xl mx-auto">
                <div className="max-w-xl mb-16">
                    <p className="mb-3 text-xs font-medium uppercase tracking-widest text-muted-foreground">Compare</p>
                    <h2 className="text-3xl font-bold tracking-tight sm:text-4xl">How GitArena stacks up.</h2>
                    <p className="mt-4 text-muted-foreground">See how GitArena compares to other popular Git hosting solutions.</p>
                </div>

                <div className="overflow-hidden rounded-lg border border-border bg-card">
                    <div className="overflow-x-auto">
                        <table className="w-full">
                            <thead>
                                <tr className="border-b border-border bg-secondary/30">
                                    <th className="px-6 py-4 text-sm font-medium text-left text-muted-foreground">Feature</th>
                                    <th className="px-6 py-4 text-sm font-semibold text-center text-foreground">GitArena</th>
                                    <th className="px-6 py-4 text-sm font-medium text-center text-muted-foreground">GitLab</th>
                                    <th className="px-6 py-4 text-sm font-medium text-center text-muted-foreground">GitHub</th>
                                    <th className="px-6 py-4 text-sm font-medium text-center text-muted-foreground">Gitea</th>
                                </tr>
                            </thead>
                            <tbody>
                                {comparisons.map((row, index) => (
                                    <tr key={row.feature} className={index !== comparisons.length - 1 ? "border-b border-border" : ""}>
                                        <td className="px-6 py-4 text-sm text-foreground">{row.feature}</td>
                                        <td className="px-6 py-4">
                                            <div className="flex justify-center">
                                                <StatusIcon status={row.gitarena} />
                                            </div>
                                        </td>
                                        <td className="px-6 py-4">
                                            <div className="flex justify-center">
                                                <StatusIcon status={row.gitlab} />
                                            </div>
                                        </td>
                                        <td className="px-6 py-4">
                                            <div className="flex justify-center">
                                                <StatusIcon status={row.github} />
                                            </div>
                                        </td>
                                        <td className="px-6 py-4">
                                            <div className="flex justify-center">
                                                <StatusIcon status={row.gitea} />
                                            </div>
                                        </td>
                                    </tr>
                                ))}
                            </tbody>
                        </table>
                    </div>
                </div>
            </div>
        </section>
    );
}

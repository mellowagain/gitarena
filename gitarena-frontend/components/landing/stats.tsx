const stats = [
    { value: "10x", label: "Faster than GitLab" },
    { value: "50MB", label: "Binary size" },
    { value: "ARM64", label: "Native support" },
    { value: "100%", label: "Open source" },
];

export function Stats() {
    return (
        <section className="px-6 py-16 border-y border-border bg-card/30">
            <div className="max-w-6xl mx-auto">
                {/* Left-aligned stats in a row */}
                <div className="flex flex-wrap gap-x-12 gap-y-6 md:gap-x-16 lg:gap-x-20">
                    {stats.map((stat) => (
                        <div key={stat.label}>
                            <p className="text-3xl font-bold tracking-tight text-foreground">{stat.value}</p>
                            <p className="mt-1 text-sm text-muted-foreground">{stat.label}</p>
                        </div>
                    ))}
                </div>
            </div>
        </section>
    );
}

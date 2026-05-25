export type Priority = "urgent" | "high" | "medium" | "low" | "none";

export const priorityConfig: Record<Priority, { bars: number; color: string; label: string }> = {
    urgent: { bars: 4, color: "bg-red-500", label: "Urgent" },
    high: { bars: 3, color: "bg-orange-500", label: "High" },
    medium: { bars: 2, color: "bg-yellow-500", label: "Medium" },
    low: { bars: 1, color: "bg-blue-500", label: "Low" },
    none: { bars: 0, color: "bg-muted", label: "No priority" },
};

export function PriorityIndicator({ priority }: { priority: Priority }) {
    const config = priorityConfig[priority];
    return (
        <div className="flex items-end gap-0.5 h-4 w-4" title={config.label}>
            {[1, 2, 3, 4].map((bar) => (
                <div
                    key={bar}
                    className={`w-0.5 rounded-full ${bar <= config.bars ? config.color : "bg-muted-foreground/20"}`}
                    style={{ height: `${bar * 25}%` }}
                />
            ))}
        </div>
    );
}

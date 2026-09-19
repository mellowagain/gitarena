import { ReactNode } from "react";
import { LucideIcon } from "lucide-react";
import { cn } from "@/lib/utils";

interface EmptyStateProps {
    icon?: LucideIcon;
    title: string;
    hint?: ReactNode;
    action?: ReactNode;
    bordered?: boolean;
    className?: string;
}

export function EmptyState({ icon: Icon, title, hint, action, bordered = false, className }: EmptyStateProps) {
    return (
        <div
            className={cn(
                "flex flex-col items-center justify-center py-20 text-center",
                bordered && "border border-border rounded-lg",
                className
            )}
        >
            {Icon && <Icon className="h-10 w-10 text-muted-foreground/40 mb-3" />}
            <p className="font-medium">{title}</p>
            {hint && <p className="text-sm text-muted-foreground mt-1">{hint}</p>}
            {action && <div className="mt-4">{action}</div>}
        </div>
    );
}

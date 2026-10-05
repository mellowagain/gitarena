import { X } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Spinner } from "@/components/ui/spinner";

interface LabelBadgeProps {
    name: string;
    color: string;
    onRemove?: () => void;
    removing?: boolean;
}

export function LabelBadge({ name, color, onRemove, removing }: LabelBadgeProps) {
    const scopedIndex = name.indexOf("::");
    const isScoped = scopedIndex !== -1;

    const removeButton = onRemove && (
        <span className="grid grid-cols-[0fr] self-stretch transition-all duration-150 group-hover/label:grid-cols-[1fr]">
            <span className="flex items-center overflow-hidden">
                <button
                    type="button"
                    className={`${isScoped ? "mx-1.5" : "ml-0.5"} rounded opacity-0 transition-opacity group-hover/label:opacity-100`}
                    aria-label="Remove label"
                    style={{ color }}
                    onClick={onRemove}
                    disabled={removing}
                >
                    {removing ? <Spinner className="size-2.5" /> : <X className="size-2.5" />}
                </button>
            </span>
        </span>
    );

    if (!isScoped) {
        return (
            <Badge className="group/label border-transparent font-normal" style={{ backgroundColor: `${color}20`, color }}>
                {name}
                {removeButton}
            </Badge>
        );
    }

    return (
        <span
            className="group/label inline-flex shrink-0 items-stretch overflow-hidden rounded-md text-xs"
            style={{ backgroundColor: `${color}20`, color }}
        >
            <span className="px-2 py-0.5 font-medium" style={{ backgroundColor: `${color}35` }}>
                {name.slice(0, scopedIndex)}
            </span>
            <span className="px-2 py-0.5">{name.slice(scopedIndex + 2)}</span>
            {removeButton}
        </span>
    );
}

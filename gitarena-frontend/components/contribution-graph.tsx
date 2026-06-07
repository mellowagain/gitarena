"use client";

import { Fragment } from "react";
import { format } from "date-fns";
import useSWR from "swr";
import { jsonFetcher } from "@/lib/fetchers";
import { Skeleton } from "@/components/ui/skeleton";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";

interface ContributionDay {
    date: string;
    count: number;
}

interface ContributionsResponse {
    contributions: ContributionDay[];
}

function levelClass(count: number, maxCount: number): string {
    if (count === 0 || maxCount === 0) return "bg-secondary";
    const ratio = count / maxCount;
    if (ratio <= 0.25) return "bg-foreground/15";
    if (ratio <= 0.5) return "bg-foreground/35";
    if (ratio <= 0.75) return "bg-foreground/60";
    return "bg-foreground/85";
}

interface ContributionGraphProps {
    username: string;
    year?: number;
}

export function ContributionGraph({ username, year }: ContributionGraphProps) {
    const url = year ? `/api/users/${username}/contributions?year=${year}` : `/api/users/${username}/contributions`;

    const { data, isLoading } = useSWR<ContributionsResponse>(url, jsonFetcher);

    if (isLoading) {
        return <Skeleton className="h-[120px] w-full" />;
    }

    if (!data || data.contributions.length === 0) {
        return (
            <div className="h-[120px] flex items-center justify-center">
                <p className="text-sm text-muted-foreground">No contributions in this period</p>
            </div>
        );
    }

    const contributions = data.contributions;
    const maxCount = Math.max(...contributions.map((d) => d.count), 0);

    const firstDate = new Date(contributions[0].date + "T00:00:00Z");
    const dayOfWeek = (firstDate.getUTCDay() + 6) % 7; // 0=Mon ... 6=Sun

    const padded: (ContributionDay | null)[] = [...Array(dayOfWeek).fill(null), ...contributions];

    const weeks: (ContributionDay | null)[][] = [];
    for (let i = 0; i < padded.length; i += 7) {
        weeks.push(padded.slice(i, i + 7));
    }

    if (weeks.length > 0 && weeks[weeks.length - 1].length < 7) {
        while (weeks[weeks.length - 1].length < 7) {
            weeks[weeks.length - 1].push(null);
        }
    }

    const dayLabels = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    const monthNames = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    const legendLevels = ["bg-secondary", "bg-foreground/15", "bg-foreground/35", "bg-foreground/60", "bg-foreground/85"];

    const monthLabels: { label: string; col: number }[] = [];
    let lastMonth = -1;
    for (let w = 0; w < weeks.length; w++) {
        const firstNonNull = weeks[w].find((d) => d !== null);
        if (firstNonNull) {
            const d = new Date(firstNonNull.date + "T00:00:00Z");
            const month = d.getUTCMonth();
            if (month !== lastMonth) {
                monthLabels.push({ label: monthNames[month], col: w });
                lastMonth = month;
            }
        }
    }

    return (
        <TooltipProvider>
            <div className="w-full">
                <div className="grid gap-[3px]" style={{ gridTemplateColumns: `auto repeat(${weeks.length}, 1fr)` }}>
                    <div />
                    {weeks.map((_, wi) => {
                        const ml = monthLabels.find((m) => m.col === wi);
                        return (
                            <div key={wi} className="text-[10px] text-muted-foreground leading-none truncate">
                                {ml ? ml.label : ""}
                            </div>
                        );
                    })}

                    {Array.from({ length: 7 }, (_, di) => (
                        <Fragment key={di}>
                            <div className="text-[10px] text-muted-foreground leading-none flex items-center justify-end pr-1">
                                {di % 2 === 1 ? dayLabels[di] : ""}
                            </div>
                            {weeks.map((week, wi) => {
                                const day = week[di];
                                if (!day) {
                                    return <div key={wi} className="aspect-square w-full" />;
                                }
                                const count = day.count;
                                const dateStr = format(new Date(day.date + "T00:00:00Z"), "EEEE, d MMMM yyyy");
                                const tile = (
                                    <div
                                        key={wi}
                                        className={`aspect-square w-full rounded-sm ${levelClass(count, maxCount)} transition-opacity hover:opacity-70`}
                                    />
                                );
                                if (count === 0) {
                                    return tile;
                                }
                                return (
                                    <Tooltip key={wi}>
                                        <TooltipTrigger asChild>{tile}</TooltipTrigger>
                                        <TooltipContent>
                                            <p>
                                                {count} contribution{count !== 1 ? "s" : ""} on {dateStr}
                                            </p>
                                        </TooltipContent>
                                    </Tooltip>
                                );
                            })}
                        </Fragment>
                    ))}
                </div>
                <div className="flex items-center justify-end gap-1.5 mt-2">
                    <span className="text-[10px] text-muted-foreground">Less</span>
                    {legendLevels.map((cls) => (
                        <div key={cls} className={`w-[11px] h-[11px] rounded-sm ${cls}`} />
                    ))}
                    <span className="text-[10px] text-muted-foreground">More</span>
                </div>
            </div>
        </TooltipProvider>
    );
}

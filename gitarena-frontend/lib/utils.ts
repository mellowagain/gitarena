import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";
import { enUS } from "date-fns/locale";
import type { Locale } from "date-fns";

export function cn(...inputs: ClassValue[]) {
    return twMerge(clsx(inputs));
}

const shortDistanceLocale: Record<string, string> = {
    xSeconds: "s",
    xMinutes: "min",
    xHours: "h",
    xDays: "d",
    xWeeks: "w",
    xMonths: "mo",
    xYears: "y",
};

export const shortLocale: Locale = {
    ...enUS,
    formatDistance: (token, count) => `${count}${shortDistanceLocale[token]}`,
};

import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";
import { enUS } from "date-fns/locale";
import type { Locale } from "date-fns";

export function cn(...inputs: ClassValue[]) {
    return twMerge(clsx(inputs));
}

const shortDistanceLocale: Record<string, string> = {
    lessThanXSeconds: "s",
    xSeconds: "s",
    halfAMinute: "30s",
    lessThanXMinutes: "min",
    xMinutes: "min",
    aboutXHours: "h",
    xHours: "h",
    xDays: "d",
    aboutXWeeks: "w",
    xWeeks: "w",
    aboutXMonths: "mo",
    xMonths: "mo",
    aboutXYears: "y",
    xYears: "y",
    overXYears: "y",
    almostXYears: "y",
};

export const shortLocale: Locale = {
    ...enUS,
    formatDistance: (token, count) => {
        if (token === "halfAMinute") return shortDistanceLocale[token];
        return `${count}${shortDistanceLocale[token]}`;
    },
};

/**
 * Extracts the creation timestamp embedded in a UUIDv7 string.
 * UUIDv7 stores a 48-bit Unix timestamp in milliseconds in the first 12 hex digits.
 */
export function uuidToDate(uuid: string): Date {
    const hex = uuid.replace(/-/g, "").slice(0, 12);
    const ms = parseInt(hex, 16);
    return new Date(ms);
}

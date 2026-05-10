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

/**
 * Extracts the creation timestamp embedded in a UUIDv7 string.
 * UUIDv7 stores a 48-bit Unix timestamp in milliseconds in the first 12 hex digits.
 */
export function uuidToDate(uuid: string): Date {
    const hex = uuid.replace(/-/g, "").slice(0, 12);
    const ms = parseInt(hex, 16);
    return new Date(ms);
}

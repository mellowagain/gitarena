"use client";

import { useState } from "react";
import * as allLangs from "linguist-languages";

type LinguistEntry = {
    color?: string;
    group?: string;
};

type Language = {
    name: string;
    percentage: number;
    color: string;
};

function languageFallbackColor(name: string): string {
    let hash = 0;
    for (let i = 0; i < name.length; i++) {
        hash = name.charCodeAt(i) + ((hash << 5) - hash);
    }
    return `hsl(${Math.abs(hash) % 360}, 60%, 55%)`;
}

function languageColor(name: string): string {
    return (allLangs as Record<string, LinguistEntry>)[name]?.color ?? languageFallbackColor(name);
}

function groupLanguages(raw: Record<string, number>): Record<string, number> {
    const grouped: Record<string, number> = {};

    for (const [name, bytes] of Object.entries(raw)) {
        const entry = (allLangs as Record<string, LinguistEntry>)[name];
        const parent = entry?.group ?? name;
        grouped[parent] = (grouped[parent] ?? 0) + bytes;
    }

    return grouped;
}

function computeLanguages(raw: Record<string, number>): Language[] {
    const total = Object.values(raw).reduce((sum, bytes) => sum + bytes, 0);
    if (total === 0) {
        return [];
    }

    return Object.entries(raw)
        .sort(([, a], [, b]) => b - a)
        .map(([name, bytes]) => ({
            name,
            percentage: Math.round((bytes / total) * 1000) / 10,
            color: languageColor(name),
        }));
}

export function LanguageBar({ languages, grouped = true }: { languages: Record<string, number>; grouped?: boolean }) {
    const [hoveredLang, setHoveredLang] = useState<string | null>(null);
    const resolved = grouped ? groupLanguages(languages) : languages;
    const computed = computeLanguages(resolved);

    return (
        <div className="space-y-2.5">
            <div className="flex h-2.5 rounded-full overflow-hidden">
                {computed.map((lang) => (
                    <div
                        key={lang.name}
                        className="h-full transition-opacity hover:opacity-80"
                        style={{ width: lang.percentage > 0 ? `${lang.percentage}%` : "2px", backgroundColor: lang.color }}
                        onMouseEnter={() => setHoveredLang(lang.name)}
                        onMouseLeave={() => setHoveredLang(null)}
                    />
                ))}
            </div>
            <div className="flex flex-wrap gap-x-4 gap-y-1.5">
                {computed.map((lang) => (
                    <div
                        key={lang.name}
                        className={`flex items-center gap-2 text-sm transition-opacity ${
                            hoveredLang && hoveredLang !== lang.name ? "opacity-40" : ""
                        }`}
                    >
                        <span className="w-2.5 h-2.5 rounded-full" style={{ backgroundColor: lang.color }} />
                        <span className="text-foreground">{lang.name}</span>
                        <span className="text-muted-foreground">{lang.percentage > 0 ? `${lang.percentage}%` : "< 0.1%"}</span>
                    </div>
                ))}
            </div>
        </div>
    );
}

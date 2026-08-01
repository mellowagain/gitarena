"use client";

import { useSyncExternalStore } from "react";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { cn } from "@/lib/utils";

const AVATAR_UPDATE_EVENT = "gitarena:avatar-update";
const avatarVersions = new Map<string, string>();

interface AvatarUpdateDetail {
    userId: string;
    version: string;
}

function subscribeToAvatarUpdates(onStoreChange: () => void) {
    window.addEventListener(AVATAR_UPDATE_EVENT, onStoreChange);
    return () => window.removeEventListener(AVATAR_UPDATE_EVENT, onStoreChange);
}

function getAvatarVersion(userId?: string | null): string | null {
    if (!userId) {
        return null;
    }

    const cachedVersion = avatarVersions.get(userId);
    if (cachedVersion) {
        return cachedVersion;
    }

    const storedVersion = sessionStorage.getItem(`${AVATAR_UPDATE_EVENT}:${userId}`);
    if (storedVersion) {
        avatarVersions.set(userId, storedVersion);
    }

    return storedVersion;
}

const sizeClasses = {
    xs: "h-4 w-4 text-[9px]",
    sm: "h-5 w-5 text-[10px]",
    md: "h-6 w-6 text-xs",
    lg: "h-8 w-8 text-sm",
    xl: "h-10 w-10 text-sm",
} as const;

interface UserAvatarProps {
    userId?: string | null;
    username: string;
    size?: keyof typeof sizeClasses;
    className?: string;
}

export function UserAvatar({ userId, username, size = "md", className }: UserAvatarProps) {
    const cacheVersion = useSyncExternalStore(
        subscribeToAvatarUpdates,
        () => getAvatarVersion(userId),
        () => null
    );

    const avatarUrl = userId ? `/api/avatar/${userId}${cacheVersion ? `?v=${encodeURIComponent(cacheVersion)}` : ""}` : null;

    return (
        <Avatar className={cn("border border-border", sizeClasses[size], className)}>
            {avatarUrl && <AvatarImage src={avatarUrl} alt={`${username}'s avatar`} className="object-cover" />}
            <AvatarFallback className="bg-secondary font-medium">{username ? username[0].toUpperCase() : "?"}</AvatarFallback>
        </Avatar>
    );
}

export function announceAvatarUpdate(userId: string, version: string) {
    avatarVersions.set(userId, version);
    sessionStorage.setItem(`${AVATAR_UPDATE_EVENT}:${userId}`, version);
    window.dispatchEvent(new CustomEvent<AvatarUpdateDetail>(AVATAR_UPDATE_EVENT, { detail: { userId, version } }));
}

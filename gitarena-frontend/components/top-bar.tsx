"use client";

import Link from "next/link";
import React, { useState, useRef, useEffect, useCallback } from "react";
import { useRouter } from "next/navigation";
import { Bell, Plus, BookOpen, Search, Users, ExternalLink, ShieldCheck, ChevronDown, UserRound } from "lucide-react";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuSeparator,
    DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Button } from "@/components/ui/button";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { UserAvatar } from "@/components/ui/user-avatar";
import { useAuth } from "@/hooks/use-auth";
import { toast } from "sonner";

export type BreadcrumbItem = { label: string; href: string } | { label: string; href?: undefined };

export type NavLink = {
    label: string;
    href: string;
    icon: React.ReactNode;
    active?: boolean;
    external?: boolean;
};

type TopBarProps = {
    breadcrumb?: BreadcrumbItem[];
    search?: {
        placeholder: string;
        scope?: {
            label: string;
            prefix: string;
        };
    };
    navLinks?: NavLink[];
    hasNotifications?: boolean;
};

function SearchBar({ search }: { search: NonNullable<TopBarProps["search"]> }) {
    const router = useRouter();
    const inputRef = useRef<HTMLInputElement>(null);
    const containerRef = useRef<HTMLDivElement>(null);
    const [value, setValue] = useState("");
    const [showDropdown, setShowDropdown] = useState(false);

    const navigate = useCallback(
        (withScope: boolean) => {
            const q = withScope && search.scope ? `${search.scope.prefix} ${value}`.trim() : value.trim();
            if (!q) {
                return;
            }
            setShowDropdown(false);
            router.push(`/search?query=${encodeURIComponent(q)}`);
        },
        [router, search.scope, value]
    );

    function handleKeyDown(e: React.KeyboardEvent<HTMLInputElement>) {
        if (e.key === "Enter") {
            e.preventDefault();
            if (search.scope) {
                // Dropdown is already visible from focus; Enter defaults to scoped search
                navigate(true);
            } else {
                navigate(false);
            }
        } else if (e.key === "Escape") {
            setShowDropdown(false);
            inputRef.current?.blur();
        }
    }

    // Close dropdown when clicking outside
    useEffect(() => {
        function handleClickOutside(e: MouseEvent) {
            if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
                setShowDropdown(false);
            }
        }
        document.addEventListener("mousedown", handleClickOutside);
        return () => document.removeEventListener("mousedown", handleClickOutside);
    }, []);

    // Focus input on "/" keypress (when not already in an input)
    useEffect(() => {
        function handleSlash(e: KeyboardEvent) {
            if (e.key !== "/") {
                return;
            }
            const tag = (e.target as HTMLElement).tagName;
            if (tag === "INPUT" || tag === "TEXTAREA" || (e.target as HTMLElement).isContentEditable) {
                return;
            }
            e.preventDefault();
            inputRef.current?.focus();
        }
        document.addEventListener("keydown", handleSlash);
        return () => document.removeEventListener("keydown", handleSlash);
    }, []);

    return (
        <div ref={containerRef} className="flex-1 max-w-lg relative">
            <div className="relative">
                <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground pointer-events-none" />
                <input
                    ref={inputRef}
                    type="text"
                    value={value}
                    onChange={(e) => setValue(e.target.value)}
                    onKeyDown={handleKeyDown}
                    onFocus={() => {
                        if (search.scope) {
                            setShowDropdown(true);
                        }
                    }}
                    placeholder={search.placeholder}
                    className="w-full h-9 pl-9 pr-10 bg-card border border-border rounded-md text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring text-sm"
                />
                <kbd className="absolute right-3 top-1/2 -translate-y-1/2 px-1.5 py-0.5 text-[11px] text-muted-foreground bg-secondary rounded border border-border">
                    /
                </kbd>
            </div>

            {showDropdown && search.scope && (
                <div className="absolute top-full mt-1 w-full bg-popover border border-border rounded-md shadow-md z-50 overflow-hidden">
                    <button
                        className="w-full flex items-center gap-2 px-3 py-2.5 text-sm hover:bg-accent transition-colors text-left"
                        onClick={() => navigate(true)}
                    >
                        <Search className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                        <span>
                            Search in <span className="font-mono font-medium">{search.scope.label}</span>
                        </span>
                    </button>
                    <div className="border-t border-border" />
                    <button
                        className="w-full flex items-center gap-2 px-3 py-2.5 text-sm hover:bg-accent transition-colors text-left text-muted-foreground"
                        onClick={() => navigate(false)}
                    >
                        <Search className="h-3.5 w-3.5 shrink-0" />
                        <span>Search all of GitArena</span>
                    </button>
                </div>
            )}
        </div>
    );
}

export function TopBar({ breadcrumb, search, navLinks, hasNotifications = false }: TopBarProps) {
    const router = useRouter();
    const { user, isAuthenticated, logout } = useAuth();
    const mobileBreadcrumb = breadcrumb?.at(-1);
    const activeNavLink = navLinks?.find((link) => link.active) ?? navLinks?.[0];

    async function handleSignOut() {
        try {
            await logout();
            router.push("/about");
        } catch {
            toast.error("Sign out failed. Please try again.");
        }
    }

    return (
        <header className="border-b border-border shrink-0 sticky top-0 z-40 bg-background">
            <div className="flex h-14 w-full min-w-0 items-center gap-2 px-3 lg:hidden">
                <div className="flex min-w-0 flex-1 items-center gap-2">
                    <Link href="/" className="shrink-0 text-base font-semibold tracking-tight hover:opacity-80">
                        GITARENA
                    </Link>
                    {mobileBreadcrumb && (
                        <>
                            <span className="shrink-0 select-none text-lg text-muted-foreground/40">/</span>
                            {mobileBreadcrumb.href ? (
                                <Link
                                    href={mobileBreadcrumb.href}
                                    className="min-w-0 truncate text-base font-medium transition-opacity hover:opacity-80"
                                >
                                    {mobileBreadcrumb.label}
                                </Link>
                            ) : (
                                <span className="min-w-0 truncate text-base font-medium">{mobileBreadcrumb.label}</span>
                            )}
                        </>
                    )}
                </div>

                <nav className="ml-auto flex shrink-0 items-center gap-1">
                    {search && (
                        <Popover>
                            <PopoverTrigger asChild>
                                <button
                                    aria-label="Search"
                                    className="flex h-9 w-9 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-accent/50 hover:text-foreground"
                                >
                                    <Search className="h-[18px] w-[18px]" />
                                </button>
                            </PopoverTrigger>
                            <PopoverContent align="end" sideOffset={8} className="w-[calc(100vw-1.5rem)] p-3">
                                <SearchBar search={search} />
                            </PopoverContent>
                        </Popover>
                    )}

                    {activeNavLink && navLinks && (
                        <DropdownMenu modal={false}>
                            <DropdownMenuTrigger asChild>
                                <Button
                                    variant="ghost"
                                    size="sm"
                                    className="h-9 max-w-24 gap-1 px-2 text-sm text-foreground"
                                    aria-label="Page navigation"
                                >
                                    <span className="truncate">{activeNavLink.label}</span>
                                    <ChevronDown className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
                                </Button>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent align="end" className="w-48">
                                {navLinks.map((link) => (
                                    <DropdownMenuItem key={`${link.href}-${link.label}`} asChild>
                                        {link.external ? (
                                            <a href={link.href} className="flex items-center gap-2">
                                                {link.icon}
                                                {link.label}
                                            </a>
                                        ) : (
                                            <Link href={link.href} className="flex items-center gap-2">
                                                {link.icon}
                                                {link.label}
                                            </Link>
                                        )}
                                    </DropdownMenuItem>
                                ))}
                            </DropdownMenuContent>
                        </DropdownMenu>
                    )}

                    {isAuthenticated && user ? (
                        <DropdownMenu modal={false}>
                            <DropdownMenuTrigger asChild>
                                <button
                                    aria-label="Account menu"
                                    className="flex h-9 w-9 items-center justify-center rounded-full transition-all hover:ring-2 hover:ring-ring"
                                >
                                    <UserAvatar userId={user.id} username={user.username} size="lg" className="size-9" />
                                </button>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent align="end" className="w-56">
                                <div className="px-3 py-2">
                                    <div className="flex items-center gap-2">
                                        <span className="font-medium">{user.username}</span>
                                        {user.admin && (
                                            <span className="inline-flex items-center gap-1 rounded border border-border bg-secondary px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
                                                <ShieldCheck className="h-2.5 w-2.5" />
                                                Admin
                                            </span>
                                        )}
                                    </div>
                                </div>
                                <DropdownMenuSeparator />
                                <DropdownMenuItem asChild>
                                    <Link href="/notifications" className="flex items-center gap-2">
                                        <Bell className="h-4 w-4" />
                                        Notifications
                                        {hasNotifications && <span className="ml-auto h-1.5 w-1.5 rounded-full bg-primary" />}
                                    </Link>
                                </DropdownMenuItem>
                                <DropdownMenuItem asChild>
                                    <Link href="/new" className="flex items-center gap-2">
                                        <BookOpen className="h-4 w-4" />
                                        New repository
                                    </Link>
                                </DropdownMenuItem>
                                <DropdownMenuItem asChild>
                                    <Link href="/import" className="flex items-center gap-2">
                                        <ExternalLink className="h-4 w-4" />
                                        Mirror repository
                                    </Link>
                                </DropdownMenuItem>
                                <DropdownMenuItem asChild>
                                    <Link href="/orgs/new" className="flex items-center gap-2">
                                        <Users className="h-4 w-4" />
                                        New organization
                                    </Link>
                                </DropdownMenuItem>
                                <DropdownMenuSeparator />
                                <DropdownMenuItem asChild>
                                    <Link href={`/${user.username}`}>Profile</Link>
                                </DropdownMenuItem>
                                <DropdownMenuItem asChild>
                                    <Link href="/settings">Settings</Link>
                                </DropdownMenuItem>
                                {user.admin && (
                                    <DropdownMenuItem asChild>
                                        <Link href="/admin">Admin panel</Link>
                                    </DropdownMenuItem>
                                )}
                                <DropdownMenuSeparator />
                                <DropdownMenuItem variant="destructive" onClick={handleSignOut}>
                                    Sign out
                                </DropdownMenuItem>
                            </DropdownMenuContent>
                        </DropdownMenu>
                    ) : (
                        <DropdownMenu modal={false}>
                            <DropdownMenuTrigger asChild>
                                <button
                                    aria-label="Account menu"
                                    className="flex h-9 w-9 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-accent/50 hover:text-foreground"
                                >
                                    <UserRound className="h-[18px] w-[18px]" />
                                </button>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent align="end" className="w-44">
                                <DropdownMenuItem asChild>
                                    <Link href="/login">Sign in</Link>
                                </DropdownMenuItem>
                                <DropdownMenuItem asChild>
                                    <Link href="/register">Create account</Link>
                                </DropdownMenuItem>
                            </DropdownMenuContent>
                        </DropdownMenu>
                    )}
                </nav>
            </div>

            <div className="hidden h-14 items-center justify-between gap-5 px-5 lg:flex">
                <div className="flex items-center gap-2 shrink-0">
                    <Link href="/" className="text-base font-semibold tracking-tight hover:opacity-80 shrink-0">
                        GITARENA
                    </Link>
                    {breadcrumb &&
                        breadcrumb.map((item, i) => (
                            <span key={i} className="contents">
                                <span className="text-muted-foreground/40 text-lg select-none">/</span>
                                {item.href ? (
                                    <Link
                                        href={item.href}
                                        className={
                                            i === breadcrumb.length - 1
                                                ? "text-base font-medium hover:opacity-80 transition-opacity"
                                                : "text-base text-muted-foreground hover:text-foreground transition-colors"
                                        }
                                    >
                                        {item.label}
                                    </Link>
                                ) : (
                                    <span className="text-base font-medium">{item.label}</span>
                                )}
                            </span>
                        ))}
                </div>

                {search && <SearchBar search={search} />}

                <nav className="flex items-center gap-1 shrink-0 ml-auto">
                    {navLinks &&
                        navLinks.map((link, i) =>
                            link.external ? (
                                <a key={i} href={link.href}>
                                    <Button
                                        variant="ghost"
                                        size="sm"
                                        className={`${link.active ? "text-foreground" : "text-muted-foreground"} h-10 px-4 gap-2.5 text-base`}
                                    >
                                        {link.icon}
                                        <span className="hidden sm:inline">{link.label}</span>
                                    </Button>
                                </a>
                            ) : (
                                <Link key={i} href={link.href}>
                                    <Button
                                        variant="ghost"
                                        size="sm"
                                        className={`${link.active ? "text-foreground" : "text-muted-foreground"} h-10 px-4 gap-2.5 text-base`}
                                    >
                                        {link.icon}
                                        <span className="hidden sm:inline">{link.label}</span>
                                    </Button>
                                </Link>
                            )
                        )}

                    {navLinks && navLinks.length > 0 && <div className="w-px h-7 bg-border mx-2" />}

                    {isAuthenticated && user ? (
                        <>
                            <button
                                className="relative flex items-center justify-center h-9 w-9 text-muted-foreground hover:text-foreground hover:bg-accent/50 rounded-md transition-colors"
                                onClick={() => router.push("/notifications")}
                                title="Notifications"
                            >
                                <Bell className="h-[18px] w-[18px]" />
                                {hasNotifications && <span className="absolute top-1.5 right-1.5 w-1.5 h-1.5 bg-blue-500 rounded-full" />}
                            </button>

                            <DropdownMenu>
                                <DropdownMenuTrigger asChild>
                                    <button
                                        className="flex items-center justify-center h-9 w-9 text-muted-foreground hover:text-foreground hover:bg-accent/50 rounded-md transition-colors"
                                        title="Create new"
                                    >
                                        <Plus className="h-[18px] w-[18px]" />
                                    </button>
                                </DropdownMenuTrigger>
                                <DropdownMenuContent align="end" className="w-52">
                                    <DropdownMenuItem asChild>
                                        <Link href="/new" className="flex items-center gap-2">
                                            <BookOpen className="h-4 w-4" />
                                            New repository
                                        </Link>
                                    </DropdownMenuItem>
                                    <DropdownMenuItem asChild>
                                        <Link href="/import" className="flex items-center gap-2">
                                            <ExternalLink className="h-4 w-4" />
                                            Mirror repository
                                        </Link>
                                    </DropdownMenuItem>
                                    <DropdownMenuSeparator />
                                    <DropdownMenuItem>
                                        <Link href="/orgs/new" className="flex items-center gap-2">
                                            <Users className="h-4 w-4 mr-2" />
                                            New organization
                                        </Link>
                                    </DropdownMenuItem>
                                </DropdownMenuContent>
                            </DropdownMenu>

                            <DropdownMenu>
                                <DropdownMenuTrigger asChild>
                                    <button className="flex items-center justify-center h-9 w-9 rounded-full hover:ring-2 hover:ring-ring transition-all ml-1">
                                        <UserAvatar userId={user.id} username={user.username} size="lg" className="size-9" />
                                    </button>
                                </DropdownMenuTrigger>
                                <DropdownMenuContent align="end" className="w-56">
                                    <div className="px-3 py-2">
                                        <div className="flex items-center gap-2">
                                            <span className="font-medium">{user.username}</span>
                                            {user.admin && (
                                                <span className="inline-flex items-center gap-1 px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wider border border-border rounded text-muted-foreground bg-secondary">
                                                    <ShieldCheck className="h-2.5 w-2.5" />
                                                    Admin
                                                </span>
                                            )}
                                        </div>
                                        <div className="text-xs text-muted-foreground font-mono mt-0.5">{user.username}</div>
                                    </div>
                                    <DropdownMenuSeparator />
                                    <DropdownMenuItem asChild>
                                        <Link href={`/${user.username}`}>Profile</Link>
                                    </DropdownMenuItem>
                                    <DropdownMenuItem asChild>
                                        <Link href="/settings">Settings</Link>
                                    </DropdownMenuItem>
                                    {user.admin && (
                                        <DropdownMenuItem asChild>
                                            <Link href="/admin">Admin panel</Link>
                                        </DropdownMenuItem>
                                    )}
                                    <DropdownMenuSeparator />
                                    <DropdownMenuItem className="text-red-500" onClick={handleSignOut}>
                                        Sign out
                                    </DropdownMenuItem>
                                </DropdownMenuContent>
                            </DropdownMenu>
                        </>
                    ) : (
                        <>
                            <Link href="/login" className="hidden sm:block">
                                <Button
                                    variant="ghost"
                                    size="sm"
                                    className="h-9 px-4 text-base text-muted-foreground hover:text-foreground"
                                >
                                    Sign in
                                </Button>
                            </Link>
                            <Link href="/register">
                                <Button variant="secondary" size="sm" className="h-9 px-3 sm:px-4 text-base">
                                    Sign up
                                </Button>
                            </Link>
                        </>
                    )}
                </nav>
            </div>
        </header>
    );
}

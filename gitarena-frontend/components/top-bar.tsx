"use client";

import Link from "next/link";
import React from "react";
import { Bell, Plus, BookOpen, Search, Users, ExternalLink, ShieldCheck } from "lucide-react";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuSeparator,
    DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Button } from "@/components/ui/button";

export type BreadcrumbItem = { label: string; href: string } | { label: string; href?: undefined };

export type NavLink = {
    label: string;
    href: string;
    icon: React.ReactNode;
    active?: boolean;
};

type TopBarProps = {
    breadcrumb?: BreadcrumbItem[];
    search?: {
        placeholder: string;
    };
    navLinks?: NavLink[];
    hasNotifications?: boolean;
};

const currentUser = {
    name: "Mari",
    username: "mellowagain",
    isAdmin: true,
};

export function TopBar({ breadcrumb, search, navLinks, hasNotifications = false }: TopBarProps) {
    return (
        <header className="border-b border-border shrink-0 sticky top-0 z-40 bg-background">
            <div className="flex h-14 items-center justify-between px-5 gap-5">
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
                                        className="text-base text-muted-foreground hover:text-foreground transition-colors"
                                    >
                                        {item.label}
                                    </Link>
                                ) : (
                                    <span className="text-base font-medium">{item.label}</span>
                                )}
                            </span>
                        ))}
                </div>

                {search && (
                    <div className="flex-1 max-w-lg">
                        <div className="relative">
                            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground pointer-events-none" />
                            <input
                                type="text"
                                placeholder={search.placeholder}
                                className="w-full h-9 pl-9 pr-10 bg-card border border-border rounded-md text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring text-sm"
                            />
                            <kbd className="absolute right-3 top-1/2 -translate-y-1/2 px-1.5 py-0.5 text-[11px] text-muted-foreground bg-secondary rounded border border-border">
                                /
                            </kbd>
                        </div>
                    </div>
                )}

                <nav className="flex items-center gap-1 shrink-0 ml-auto">
                    {navLinks &&
                        navLinks.map((link, i) => (
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
                        ))}

                    {navLinks && navLinks.length > 0 && <div className="w-px h-7 bg-border mx-2" />}

                    <button
                        className="relative flex items-center justify-center h-9 w-9 text-muted-foreground hover:text-foreground hover:bg-accent/50 rounded-md transition-colors"
                        onClick={() => (window.location.href = "/notifications")}
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
                                <Users className="h-4 w-4 mr-2" />
                                New organization
                            </DropdownMenuItem>
                        </DropdownMenuContent>
                    </DropdownMenu>

                    <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                            <button className="flex items-center justify-center h-9 w-9 rounded-full bg-secondary text-base font-medium hover:ring-2 hover:ring-ring transition-all ml-1">
                                {currentUser.name[0].toUpperCase()}
                            </button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end" className="w-56">
                            <div className="px-3 py-2">
                                <div className="flex items-center gap-2">
                                    <span className="font-medium">{currentUser.name}</span>
                                    {currentUser.isAdmin && (
                                        <span className="inline-flex items-center gap-1 px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wider border border-border rounded text-muted-foreground bg-secondary">
                                            <ShieldCheck className="h-2.5 w-2.5" />
                                            Admin
                                        </span>
                                    )}
                                </div>
                                <div className="text-xs text-muted-foreground font-mono mt-0.5">{currentUser.username}</div>
                            </div>
                            <DropdownMenuSeparator />
                            <DropdownMenuItem asChild>
                                <Link href={`/${currentUser.username}`}>Profile</Link>
                            </DropdownMenuItem>
                            <DropdownMenuItem asChild>
                                <Link href="/settings">Settings</Link>
                            </DropdownMenuItem>
                            {currentUser.isAdmin && (
                                <DropdownMenuItem asChild>
                                    <Link href="/admin">Admin panel</Link>
                                </DropdownMenuItem>
                            )}
                            <DropdownMenuSeparator />
                            <DropdownMenuItem className="text-red-500">Sign out</DropdownMenuItem>
                        </DropdownMenuContent>
                    </DropdownMenu>
                </nav>
            </div>
        </header>
    );
}

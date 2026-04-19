"use client";

import { useState } from "react";
import Link from "next/link";
import { Button } from "@/components/ui/button";
import { Menu, X } from "lucide-react";

const navLinks = [
    { href: "#features", label: "Features" },
    { href: "#comparison", label: "Compare" },
    { href: "/docs", label: "Docs" },
    { href: "/pricing", label: "Pricing" },
];

export function Navbar() {
    const [mobileMenuOpen, setMobileMenuOpen] = useState(false);

    return (
        <header className="sticky top-0 z-50 border-b border-border bg-background/80 backdrop-blur-xl">
            <nav className="flex items-center justify-between h-16 max-w-6xl px-6 mx-auto">
                {/* Logo */}
                <Link href="/" className="flex items-center gap-2">
                    <div className="flex items-center justify-center w-8 h-8 font-bold rounded-lg bg-foreground text-background">G</div>
                    <span className="text-lg font-semibold">GitArena</span>
                </Link>

                {/* Desktop Navigation */}
                <div className="items-center hidden gap-8 md:flex">
                    {navLinks.map((link) => (
                        <Link
                            key={link.href}
                            href={link.href}
                            className="text-sm transition-colors text-muted-foreground hover:text-foreground"
                        >
                            {link.label}
                        </Link>
                    ))}
                </div>

                {/* Desktop Actions */}
                <div className="items-center hidden gap-3 md:flex">
                    <Button variant="ghost" size="sm">
                        Sign In
                    </Button>
                    <Button size="sm">Get Started</Button>
                </div>

                {/* Mobile Menu Button */}
                <button className="p-2 md:hidden" onClick={() => setMobileMenuOpen(!mobileMenuOpen)} aria-label="Toggle menu">
                    {mobileMenuOpen ? <X className="w-5 h-5" /> : <Menu className="w-5 h-5" />}
                </button>
            </nav>

            {/* Mobile Menu */}
            {mobileMenuOpen && (
                <div className="px-6 pb-6 border-b md:hidden border-border bg-background">
                    <div className="flex flex-col gap-4">
                        {navLinks.map((link) => (
                            <Link
                                key={link.href}
                                href={link.href}
                                className="text-sm transition-colors text-muted-foreground hover:text-foreground"
                                onClick={() => setMobileMenuOpen(false)}
                            >
                                {link.label}
                            </Link>
                        ))}
                        <div className="flex flex-col gap-2 pt-4 border-t border-border">
                            <Button variant="ghost" size="sm">
                                Sign In
                            </Button>
                            <Button size="sm">Get Started</Button>
                        </div>
                    </div>
                </div>
            )}
        </header>
    );
}

"use client";

import { GitBranch, ChevronDown, History } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuRadioGroup,
    DropdownMenuRadioItem,
    DropdownMenuSeparator,
    DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import Link from "next/link";
import useSWR from "swr";
import { ErrorDisplay } from "@/components/error-display";

interface BranchInfo {
    name: string;
    commitCount: number;
    ahead: number;
    behind: number;
}

interface BranchesResponse {
    branches: BranchInfo[];
}

export interface BranchBarProps {
    user: string;
    repo: string;
    defaultBranch: string;
    selectedBranch: string;
    onBranchChange: (branch: string) => void;
}

export function BranchBar({ user, repo, defaultBranch, selectedBranch, onBranchChange }: BranchBarProps) {
    const { data, error, isLoading } = useSWR<BranchesResponse>(`http://localhost:8080/api/repos/${user}/${repo}/branches`);

    if (isLoading) {
        return <BranchBarSkeleton />;
    }

    if (error || !data) {
        return <ErrorDisplay failed={"branches"} error={error} />;
    }

    const defaultBranchInfo = data.branches.find((b) => b.name === defaultBranch);
    const otherBranches = data.branches.filter((b) => b.name !== defaultBranch).sort((a, b) => a.name.localeCompare(b.name));
    const selectedBranchInfo = data.branches.find((b) => b.name === selectedBranch);

    return (
        <div className="flex items-center gap-2">
            <DropdownMenu>
                <DropdownMenuTrigger asChild>
                    <Button variant="secondary" size="sm" className="flex-1 justify-between h-9">
                        <span className="flex items-center gap-2">
                            <GitBranch className="h-4 w-4" />
                            {selectedBranch}
                        </span>
                        <ChevronDown className="h-3.5 w-3.5 opacity-50" />
                    </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="start" className="w-56">
                    <DropdownMenuRadioGroup value={selectedBranch} onValueChange={onBranchChange}>
                        {defaultBranchInfo && (
                            <DropdownMenuRadioItem value={defaultBranchInfo.name}>{defaultBranchInfo.name}</DropdownMenuRadioItem>
                        )}
                        {defaultBranchInfo && otherBranches.length > 0 && <DropdownMenuSeparator />}
                        {otherBranches.map((b) => (
                            <DropdownMenuRadioItem key={b.name} value={b.name}>
                                {b.name}
                            </DropdownMenuRadioItem>
                        ))}
                    </DropdownMenuRadioGroup>
                </DropdownMenuContent>
            </DropdownMenu>
            <Link
                href="#"
                className="flex items-center gap-1.5 px-2.5 h-9 text-sm text-muted-foreground hover:text-foreground transition-colors border border-border rounded-md hover:bg-accent/50"
            >
                <History className="h-3.5 w-3.5" />

                {selectedBranchInfo && <span>{selectedBranchInfo.commitCount}</span>}
            </Link>
        </div>
    );
}

export function BranchBarSkeleton() {
    return (
        <div className="flex items-center gap-2 animate-pulse">
            <div className="h-9 flex-1 rounded bg-accent" />
            <div className="h-9 w-14 rounded bg-accent" />
        </div>
    );
}

"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import useSWRMutation from "swr/mutation";
import { toast } from "sonner";
import { TopBar } from "@/components/top-bar";
import { Button } from "@/components/ui/button";
import { Building2, Loader2 } from "lucide-react";
import { postJsonFetcher } from "@/lib/fetchers";

interface CreateOrgRequest {
    name: string;
    description?: string;
}

interface CreateOrgResponse {
    id: string;
    name: string;
}

const NAME_PATTERN = /^[a-z0-9_-]+$/;

export default function NewOrganizationPage() {
    const router = useRouter();

    const [name, setName] = useState("");
    const [description, setDescription] = useState("");

    const { trigger, isMutating } = useSWRMutation<CreateOrgResponse, Error, string, CreateOrgRequest>("/api/orgs", postJsonFetcher);

    const nameError = name
        ? name.length > 32
            ? "Name must be 32 characters or fewer"
            : !NAME_PATTERN.test(name)
              ? "Name may only contain lowercase letters, numbers, hyphens, and underscores"
              : null
        : null;

    const descriptionError = description.length > 256 ? "Description must be 256 characters or fewer" : null;

    const canSubmit = !!name && !nameError && !descriptionError && !isMutating;

    async function handleSubmit(e: React.FormEvent) {
        e.preventDefault();
        if (!canSubmit) {
            return;
        }

        try {
            const org = await trigger({
                name,
                ...(description ? { description } : {}),
            });
            router.push(`/${org.name}`);
        } catch (err) {
            toast.error(err instanceof Error ? err.message : "Failed to create organization");
        }
    }

    return (
        <div className="min-h-screen bg-background flex flex-col">
            <TopBar breadcrumb={[{ label: "New organization" }]} />

            <main className="flex-1 flex items-start justify-center p-8 lg:p-12">
                <div className="w-full max-w-lg">
                    <div className="flex items-center gap-3 mb-8">
                        <div className="flex items-center justify-center h-12 w-12 rounded-lg bg-card border border-border">
                            <Building2 className="h-6 w-6 text-muted-foreground" />
                        </div>
                        <div>
                            <h1 className="text-2xl font-semibold">Create an organization</h1>
                            <p className="text-muted-foreground">Organizations are shared accounts for collaborative work.</p>
                        </div>
                    </div>

                    <form onSubmit={handleSubmit} className="space-y-6">
                        <div className="space-y-2">
                            <label className="text-sm font-medium">
                                Organization name <span className="text-red-500">*</span>
                            </label>
                            <input
                                type="text"
                                value={name}
                                onChange={(e) => setName(e.target.value.toLowerCase())}
                                placeholder="my-organization"
                                maxLength={32}
                                className="w-full h-11 px-4 bg-card border border-border rounded-md text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
                            />
                            {nameError && <p className="text-sm text-red-500">{nameError}</p>}
                            <p className="text-xs text-muted-foreground">
                                Lowercase letters, numbers, hyphens, and underscores only. Max 32 characters.
                            </p>
                        </div>

                        <div className="space-y-2">
                            <label className="text-sm font-medium flex items-center gap-2">
                                Description <span className="text-muted-foreground font-normal">(optional)</span>
                            </label>
                            <textarea
                                value={description}
                                onChange={(e) => setDescription(e.target.value)}
                                placeholder="A short description of your organization"
                                rows={3}
                                maxLength={256}
                                className="w-full px-4 py-3 bg-card border border-border rounded-md text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring resize-none"
                            />
                            {descriptionError && <p className="text-sm text-red-500">{descriptionError}</p>}
                            <p className="text-xs text-muted-foreground text-right">{description.length}/256</p>
                        </div>

                        <Button type="submit" className="w-full h-11" disabled={!canSubmit}>
                            {isMutating ? (
                                <>
                                    <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                                    Creating…
                                </>
                            ) : (
                                "Create organization"
                            )}
                        </Button>
                    </form>
                </div>
            </main>
        </div>
    );
}

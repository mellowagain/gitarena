"use client";

import { useState } from "react";
import useSWRMutation from "swr/mutation";
import { Trash2 } from "lucide-react";
import { deleteFetcher } from "@/lib/fetchers";
import { buttonVariants } from "@/components/ui/button";
import { Field, FieldDescription } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Spinner } from "@/components/ui/spinner";
import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
} from "@/components/ui/alert-dialog";

interface DeleteUserDialogProps {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    url: string;
    username: string;
    title: string;
    description: string;
    onDeleted: () => Promise<unknown> | void;
}

export function DeleteUserDialog({ open, onOpenChange, url, username, title, description, onDeleted }: DeleteUserDialogProps) {
    const [confirmInput, setConfirmInput] = useState("");

    const {
        trigger: deleteUser,
        isMutating: isDeleting,
        error,
        reset,
    } = useSWRMutation<void, Error, string>(url, deleteFetcher, {
        onSuccess: async () => {
            await onDeleted();
            onOpenChange(false);
        },
    });

    return (
        <AlertDialog
            open={open}
            onOpenChange={(next) => {
                if (isDeleting) {
                    return;
                }

                onOpenChange(next);

                if (!next) {
                    setConfirmInput("");
                    reset();
                }
            }}
        >
            <AlertDialogContent>
                <AlertDialogHeader>
                    <AlertDialogTitle>{title}</AlertDialogTitle>
                    <AlertDialogDescription>{description}</AlertDialogDescription>
                </AlertDialogHeader>
                <Field className="gap-1.5">
                    <FieldDescription id="user-delete-confirm-label">
                        Please type <code className="font-mono font-semibold text-foreground">{username}</code> to confirm.
                    </FieldDescription>
                    <Input
                        aria-labelledby="user-delete-confirm-label"
                        value={confirmInput}
                        onChange={(e) => setConfirmInput(e.target.value)}
                        placeholder={username}
                        className="border-destructive/40 focus-visible:ring-destructive/40"
                    />
                </Field>
                {error && <p className="text-sm text-destructive">{error.message}</p>}
                <AlertDialogFooter>
                    <AlertDialogCancel disabled={isDeleting}>Cancel</AlertDialogCancel>
                    <AlertDialogAction
                        onClick={(e) => {
                            e.preventDefault();
                            deleteUser();
                        }}
                        disabled={confirmInput !== username || isDeleting}
                        className={buttonVariants({ variant: "destructive" })}
                    >
                        {isDeleting ? <Spinner /> : <Trash2 />}I understand, delete this account
                    </AlertDialogAction>
                </AlertDialogFooter>
            </AlertDialogContent>
        </AlertDialog>
    );
}

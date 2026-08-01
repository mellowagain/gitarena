"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import Cropper, { type Area, type Point } from "react-easy-crop";
import "react-easy-crop/react-easy-crop.css";
import { FlipHorizontal2, FlipVertical2, Plus, RotateCcw, RotateCw, Trash2, Undo2 } from "lucide-react";
import useSWR from "swr";
import useSWRMutation from "swr/mutation";
import { toast } from "sonner";

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
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { announceAvatarUpdate } from "@/components/ui/user-avatar";
import { Button, buttonVariants } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Slider } from "@/components/ui/slider";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { deleteFetcher, postEmptyFetcher } from "@/lib/fetchers";
import type { InstanceConfig } from "@/lib/instance-config";

const AVATAR_ENDPOINT = "/api/avatar";
const AVATAR_SIZE = 256;
const MAX_FILE_SIZE = 25 * 1024 * 1024;
const WEBP_QUALITY = 0.9;
const ACCEPTED_FILE_TYPES = [
    ".jpg",
    ".jpeg",
    ".png",
    ".webp",
    ".avif",
    ".heic",
    ".heif",
    "image/jpeg",
    "image/png",
    "image/webp",
    "image/avif",
    "image/heic",
    "image/heif",
].join(",");
const SUPPORTED_MIME_TYPES = new Set(["image/jpeg", "image/png", "image/webp", "image/avif", "image/heic", "image/heif"]);
const SUPPORTED_EXTENSIONS = new Set(["jpg", "jpeg", "png", "webp", "avif", "heic", "heif"]);

interface AvatarManagementProps {
    userId: string;
    username: string;
}

interface UploadAvatarResponse {
    uploadUrl: string;
}

interface Flip {
    horizontal: boolean;
    vertical: boolean;
}

function getExtension(fileName: string): string {
    return fileName.split(".").pop()?.toLowerCase() ?? "";
}

function isHeicFile(file: File): boolean {
    const extension = getExtension(file.name);
    return file.type === "image/heic" || file.type === "image/heif" || extension === "heic" || extension === "heif";
}

function validateFile(file: File): void {
    const supportedByMime = SUPPORTED_MIME_TYPES.has(file.type.toLowerCase());
    const supportedByExtension = SUPPORTED_EXTENSIONS.has(getExtension(file.name));

    if (!supportedByMime && !supportedByExtension) {
        throw new Error("Choose a JPEG, PNG, WebP, AVIF, HEIC, or HEIF image.");
    }

    if (file.size > MAX_FILE_SIZE) {
        throw new Error("Avatar images must be 25 MB or smaller.");
    }
}

function loadImage(source: string): Promise<HTMLImageElement> {
    return new Promise((resolve, reject) => {
        const image = new Image();

        image.onload = () => {
            if (image.naturalWidth === 0 || image.naturalHeight === 0) {
                reject(new Error("The selected image has no usable pixels."));
                return;
            }

            resolve(image);
        };
        image.onerror = () => reject(new Error("The selected image could not be decoded."));
        image.src = source;
    });
}

async function prepareImage(file: File): Promise<string> {
    validateFile(file);

    let imageBlob: Blob = file;

    if (isHeicFile(file)) {
        try {
            const { heicTo, isHeic } = await import("heic-to/csp");

            if (!(await isHeic(file))) {
                throw new Error("Invalid HEIC or HEIF image");
            }

            imageBlob = await heicTo({ blob: file, type: "image/png" });
        } catch {
            throw new Error("The HEIC or HEIF image could not be decoded.");
        }
    }

    const imageUrl = URL.createObjectURL(imageBlob);

    try {
        await loadImage(imageUrl);
        return imageUrl;
    } catch {
        URL.revokeObjectURL(imageUrl);
        throw new Error("The selected image is malformed or is not supported by this browser.");
    }
}

function rotatedSize(width: number, height: number, rotation: number): { width: number; height: number } {
    const radians = (rotation * Math.PI) / 180;

    return {
        width: Math.abs(Math.cos(radians) * width) + Math.abs(Math.sin(radians) * height),
        height: Math.abs(Math.sin(radians) * width) + Math.abs(Math.cos(radians) * height),
    };
}

async function renderAvatar(source: string, crop: Area, rotation: number, flip: Flip): Promise<Blob> {
    const image = await loadImage(source);
    const canvas = document.createElement("canvas");
    const context = canvas.getContext("2d");

    if (!context) {
        throw new Error("This browser could not prepare the avatar image.");
    }

    canvas.width = AVATAR_SIZE;
    canvas.height = AVATAR_SIZE;
    context.imageSmoothingEnabled = true;
    context.imageSmoothingQuality = "high";

    const rotationRadians = (rotation * Math.PI) / 180;
    const bounds = rotatedSize(image.naturalWidth, image.naturalHeight, rotation);

    context.scale(AVATAR_SIZE / crop.width, AVATAR_SIZE / crop.height);
    context.translate(-crop.x, -crop.y);
    context.translate(bounds.width / 2, bounds.height / 2);
    context.rotate(rotationRadians);
    context.scale(flip.horizontal ? -1 : 1, flip.vertical ? -1 : 1);
    context.drawImage(image, -image.naturalWidth / 2, -image.naturalHeight / 2);

    const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, "image/webp", WEBP_QUALITY));

    if (!blob || blob.type !== "image/webp") {
        throw new Error("This browser could not encode the avatar as WebP.");
    }

    return blob;
}

async function uploadAvatarFetcher(url: string, { arg }: { arg: Blob }): Promise<void> {
    const { uploadUrl } = await postEmptyFetcher<UploadAvatarResponse>(url);
    let response: Response;

    try {
        response = await fetch(uploadUrl, {
            method: "PUT",
            headers: { "Content-Type": "image/webp" },
            body: arg,
        });
    } catch {
        throw new Error("Could not reach object storage to upload the avatar.");
    }

    if (!response.ok) {
        throw new Error("Object storage rejected the avatar upload.");
    }
}

function errorMessage(error: unknown, fallback: string): string {
    return error instanceof Error ? error.message : fallback;
}

export function AvatarManagement({ userId, username }: AvatarManagementProps) {
    const { data: instanceConfig, isLoading: isConfigLoading, error: configError } = useSWR<InstanceConfig>("/api");
    const [cacheVersion, setCacheVersion] = useState<string | null>(null);
    const [hasAvatar, setHasAvatar] = useState(false);
    const [editorOpen, setEditorOpen] = useState(false);
    const [removeOpen, setRemoveOpen] = useState(false);
    const [isPreparing, setIsPreparing] = useState(false);
    const [imageUrl, setImageUrl] = useState<string | null>(null);
    const [crop, setCrop] = useState<Point>({ x: 0, y: 0 });
    const [croppedArea, setCroppedArea] = useState<Area | null>(null);
    const [zoom, setZoom] = useState(1);
    const [rotation, setRotation] = useState(0);
    const [flip, setFlip] = useState<Flip>({ horizontal: false, vertical: false });
    const fileInputRef = useRef<HTMLInputElement>(null);
    const imageUrlRef = useRef<string | null>(null);
    const preparationIdRef = useRef(0);
    const uploadInFlightRef = useRef(false);
    const deleteInFlightRef = useRef(false);

    const { trigger: uploadAvatar, isMutating: isUploading } = useSWRMutation<void, Error, string, Blob>(
        AVATAR_ENDPOINT,
        uploadAvatarFetcher
    );
    const { trigger: deleteAvatar, isMutating: isDeleting } = useSWRMutation(AVATAR_ENDPOINT, deleteFetcher);
    const busy = isPreparing || isUploading || isDeleting;
    const uploadUnavailable = Boolean(configError) || (!isConfigLoading && instanceConfig?.objectStorageAvailable !== true);

    const resetEditor = useCallback(() => {
        setCrop({ x: 0, y: 0 });
        setCroppedArea(null);
        setZoom(1);
        setRotation(0);
        setFlip({ horizontal: false, vertical: false });
    }, []);

    const releaseImageUrl = useCallback(() => {
        if (imageUrlRef.current) {
            URL.revokeObjectURL(imageUrlRef.current);
            imageUrlRef.current = null;
        }

        setImageUrl(null);
    }, []);

    useEffect(() => {
        const frame = window.requestAnimationFrame(() => {
            const version = crypto.randomUUID();
            setCacheVersion(version);
            announceAvatarUpdate(userId, version);
        });

        return () => {
            window.cancelAnimationFrame(frame);
            preparationIdRef.current += 1;

            if (imageUrlRef.current) {
                URL.revokeObjectURL(imageUrlRef.current);
            }
        };
    }, [userId]);

    const handleFileChange = async (event: React.ChangeEvent<HTMLInputElement>) => {
        const file = event.currentTarget.files?.[0];
        event.currentTarget.value = "";

        if (!file) {
            return;
        }

        const preparationId = preparationIdRef.current + 1;
        preparationIdRef.current = preparationId;
        setIsPreparing(true);

        try {
            const preparedUrl = await prepareImage(file);

            if (preparationIdRef.current !== preparationId) {
                URL.revokeObjectURL(preparedUrl);
                return;
            }

            releaseImageUrl();
            imageUrlRef.current = preparedUrl;
            setImageUrl(preparedUrl);
            resetEditor();
            setEditorOpen(true);
        } catch (error) {
            toast.error(errorMessage(error, "The avatar image could not be prepared."));
        } finally {
            if (preparationIdRef.current === preparationId) {
                setIsPreparing(false);
            }
        }
    };

    const handleEditorOpenChange = (open: boolean) => {
        if (!open && isUploading) {
            return;
        }

        setEditorOpen(open);

        if (!open) {
            releaseImageUrl();
            resetEditor();
        }
    };

    const handleCropComplete = useCallback((_croppedArea: Area, croppedAreaPixels: Area) => {
        setCroppedArea(croppedAreaPixels);
    }, []);

    const handleUpload = async () => {
        if (!imageUrl || !croppedArea || uploadUnavailable || uploadInFlightRef.current || deleteInFlightRef.current) {
            return;
        }

        uploadInFlightRef.current = true;

        try {
            const avatar = await renderAvatar(imageUrl, croppedArea, rotation, flip);
            await uploadAvatar(avatar);
            const version = crypto.randomUUID();
            setHasAvatar(false);
            setCacheVersion(version);
            announceAvatarUpdate(userId, version);
            setEditorOpen(false);
            releaseImageUrl();
            resetEditor();
            toast.success("Avatar updated");
        } catch (error) {
            toast.error(errorMessage(error, "Failed to upload avatar."));
        } finally {
            uploadInFlightRef.current = false;
        }
    };

    const handleDelete = async (event: React.MouseEvent<HTMLButtonElement>) => {
        event.preventDefault();

        if (uploadInFlightRef.current || deleteInFlightRef.current) {
            return;
        }

        deleteInFlightRef.current = true;

        try {
            await deleteAvatar();
            const version = crypto.randomUUID();
            setHasAvatar(false);
            setCacheVersion(version);
            announceAvatarUpdate(userId, version);
            setRemoveOpen(false);
            toast.success("Avatar removed");
        } catch (error) {
            toast.error(errorMessage(error, "Failed to remove avatar."));
        } finally {
            deleteInFlightRef.current = false;
        }
    };

    const initial = username[0]?.toUpperCase() ?? "?";
    const avatarUrl = cacheVersion ? `/api/avatar/${userId}?v=${encodeURIComponent(cacheVersion)}` : null;
    const cropTransform = `translate(${crop.x}px, ${crop.y}px) rotate(${rotation}deg) scale(${zoom}) scaleX(${flip.horizontal ? -1 : 1}) scaleY(${flip.vertical ? -1 : 1})`;

    return (
        <>
            <input
                ref={fileInputRef}
                type="file"
                accept={ACCEPTED_FILE_TYPES}
                disabled={busy || isConfigLoading || uploadUnavailable}
                onChange={handleFileChange}
                className="sr-only"
            />

            <div className="flex flex-col gap-4 sm:flex-row sm:items-center">
                <Avatar className="size-16 border border-border">
                    {avatarUrl && (
                        <AvatarImage
                            key={cacheVersion}
                            src={avatarUrl}
                            alt={`${username}'s avatar`}
                            onLoad={() => setHasAvatar(true)}
                            onError={() => setHasAvatar(false)}
                        />
                    )}
                    <AvatarFallback className="bg-secondary text-xl font-semibold">{initial}</AvatarFallback>
                </Avatar>

                <div className="flex flex-col items-start gap-2">
                    <div className="flex flex-wrap gap-2">
                        {isConfigLoading ? (
                            <Skeleton className="h-8 w-32 rounded-md" />
                        ) : (
                            <Button
                                type="button"
                                variant="outline"
                                size="sm"
                                disabled={busy || uploadUnavailable}
                                onClick={() => fileInputRef.current?.click()}
                            >
                                {isPreparing ? <Spinner /> : <Plus />}
                                Upload avatar
                            </Button>
                        )}

                        {hasAvatar && (
                            <Button type="button" variant="destructive" size="sm" disabled={busy} onClick={() => setRemoveOpen(true)}>
                                {isDeleting ? <Spinner /> : <Trash2 />}
                                Remove avatar
                            </Button>
                        )}
                    </div>
                    <p className="text-xs text-muted-foreground">
                        {configError
                            ? "Avatar uploads are unavailable right now."
                            : instanceConfig?.objectStorageAvailable === false
                              ? "Avatar uploads are disabled on this instance."
                              : "JPEG, PNG, WebP, AVIF, HEIC, or HEIF. Max 25 MB."}
                    </p>
                </div>
            </div>

            <Dialog open={editorOpen} onOpenChange={handleEditorOpenChange}>
                <DialogContent
                    className="gap-5 p-4 data-[state=closed]:zoom-out-100 data-[state=open]:zoom-in-100 sm:max-w-2xl sm:p-6"
                    showCloseButton={!isUploading}
                >
                    <DialogHeader>
                        <DialogTitle>Edit avatar</DialogTitle>
                        <DialogDescription>Drag or pinch the image to choose the part that will appear in your avatar.</DialogDescription>
                    </DialogHeader>

                    <div className="relative h-[min(55vh,22rem)] min-h-64 overflow-hidden rounded-lg border border-border bg-card sm:h-[26rem]">
                        {imageUrl && (
                            <Cropper
                                image={imageUrl}
                                crop={crop}
                                zoom={zoom}
                                rotation={rotation}
                                aspect={1}
                                cropShape="round"
                                showGrid={false}
                                objectFit="contain"
                                roundCropAreaPixels
                                transform={cropTransform}
                                onCropChange={setCrop}
                                onZoomChange={setZoom}
                                onCropComplete={handleCropComplete}
                                classes={{ containerClassName: "bg-card", cropAreaClassName: "border-foreground/70" }}
                                disableAutomaticStylesInjection
                            />
                        )}
                    </div>

                    <div className="space-y-4">
                        <div className="flex items-center gap-3">
                            <span className="w-12 text-sm text-muted-foreground">Zoom</span>
                            <Slider
                                aria-label="Avatar zoom"
                                value={[zoom]}
                                min={1}
                                max={3}
                                step={0.01}
                                disabled={isUploading}
                                onValueChange={([value]) => setZoom(value)}
                            />
                            <span className="w-11 text-right font-mono text-xs text-muted-foreground">{Math.round(zoom * 100)}%</span>
                        </div>

                        <div className="flex flex-wrap items-center justify-between gap-3">
                            <div className="flex gap-2">
                                <Tooltip>
                                    <TooltipTrigger asChild>
                                        <Button
                                            type="button"
                                            variant="outline"
                                            size="icon-sm"
                                            aria-label="Rotate left 90 degrees"
                                            disabled={isUploading}
                                            onClick={() => setRotation((value) => value - 90)}
                                        >
                                            <RotateCcw />
                                        </Button>
                                    </TooltipTrigger>
                                    <TooltipContent>Rotate left</TooltipContent>
                                </Tooltip>
                                <Tooltip>
                                    <TooltipTrigger asChild>
                                        <Button
                                            type="button"
                                            variant="outline"
                                            size="icon-sm"
                                            aria-label="Rotate right 90 degrees"
                                            disabled={isUploading}
                                            onClick={() => setRotation((value) => value + 90)}
                                        >
                                            <RotateCw />
                                        </Button>
                                    </TooltipTrigger>
                                    <TooltipContent>Rotate right</TooltipContent>
                                </Tooltip>
                                <Tooltip>
                                    <TooltipTrigger asChild>
                                        <Button
                                            type="button"
                                            variant={flip.horizontal ? "secondary" : "outline"}
                                            size="icon-sm"
                                            aria-label="Flip horizontally"
                                            aria-pressed={flip.horizontal}
                                            disabled={isUploading}
                                            onClick={() => setFlip((value) => ({ ...value, horizontal: !value.horizontal }))}
                                        >
                                            <FlipHorizontal2 />
                                        </Button>
                                    </TooltipTrigger>
                                    <TooltipContent>Flip horizontally</TooltipContent>
                                </Tooltip>
                                <Tooltip>
                                    <TooltipTrigger asChild>
                                        <Button
                                            type="button"
                                            variant={flip.vertical ? "secondary" : "outline"}
                                            size="icon-sm"
                                            aria-label="Flip vertically"
                                            aria-pressed={flip.vertical}
                                            disabled={isUploading}
                                            onClick={() => setFlip((value) => ({ ...value, vertical: !value.vertical }))}
                                        >
                                            <FlipVertical2 />
                                        </Button>
                                    </TooltipTrigger>
                                    <TooltipContent>Flip vertically</TooltipContent>
                                </Tooltip>
                            </div>

                            <Button type="button" variant="ghost" size="sm" disabled={isUploading} onClick={resetEditor}>
                                <Undo2 />
                                Reset
                            </Button>
                        </div>
                    </div>

                    <DialogFooter>
                        <Button type="button" variant="outline" disabled={isUploading} onClick={() => handleEditorOpenChange(false)}>
                            Cancel
                        </Button>
                        <Button type="button" disabled={!croppedArea || isUploading || uploadUnavailable} onClick={handleUpload}>
                            {isUploading && <Spinner />}
                            Save avatar
                        </Button>
                    </DialogFooter>
                </DialogContent>
            </Dialog>

            <AlertDialog open={removeOpen} onOpenChange={(open) => !isDeleting && setRemoveOpen(open)}>
                <AlertDialogContent>
                    <AlertDialogHeader>
                        <AlertDialogTitle>Remove avatar?</AlertDialogTitle>
                        <AlertDialogDescription>
                            Your username initial will be shown until you upload another avatar.
                        </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel disabled={isDeleting}>Cancel</AlertDialogCancel>
                        <AlertDialogAction
                            className={buttonVariants({ variant: "destructive" })}
                            disabled={isDeleting || isUploading}
                            onClick={handleDelete}
                        >
                            {isDeleting && <Spinner />}
                            Remove avatar
                        </AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>
        </>
    );
}

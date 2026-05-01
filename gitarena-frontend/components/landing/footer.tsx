import Link from "next/link";
import { InstanceConfig } from "@/lib/instance-config";

interface FooterProps {
    apiInfo?: InstanceConfig | null;
}

export function Footer({ apiInfo }: FooterProps) {
    const repoUrl = apiInfo?.repository ?? "https://github.com/mellowagain/gitarena";
    const releaseUrl = apiInfo?.version ? `${repoUrl}/releases/tag/v${apiInfo.version}` : repoUrl;
    const shortCommit = apiInfo?.commit?.slice(0, 7);

    return (
        <footer className="px-6 py-12 border-t border-border">
            <div className="max-w-6xl mx-auto">
                <div className="flex flex-col gap-6 md:flex-row md:items-center md:justify-between">
                    {/* Brand + version */}
                    <div className="flex items-center gap-4">
                        <a
                            href={repoUrl}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="text-base font-semibold tracking-tight hover:opacity-80"
                        >
                            GITARENA
                        </a>
                        {apiInfo && (
                            <span className="text-sm text-muted-foreground font-mono">
                                <a
                                    href={releaseUrl}
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    className="hover:text-foreground transition-colors"
                                >
                                    v{apiInfo.version}
                                </a>
                                {shortCommit && (
                                    <>
                                        {" "}
                                        ·{" "}
                                        <a
                                            href={`${repoUrl}/commit/${apiInfo.commit}`}
                                            target="_blank"
                                            rel="noopener noreferrer"
                                            className="hover:text-foreground transition-colors"
                                        >
                                            {shortCommit}
                                        </a>
                                    </>
                                )}
                            </span>
                        )}
                    </div>

                    {/* Links */}
                    <div className="flex flex-wrap items-center gap-x-6 gap-y-2 text-sm text-muted-foreground">
                        <Link href="/docs/api-reference/introduction" className="hover:text-foreground transition-colors">
                            API reference
                        </Link>
                        <Link href="/privacy" className="hover:text-foreground transition-colors">
                            Privacy policy
                        </Link>
                        <Link href="/terms" className="hover:text-foreground transition-colors">
                            Terms of service
                        </Link>
                        <Link href="/dmca" className="hover:text-foreground transition-colors">
                            DMCA
                        </Link>
                    </div>
                </div>

                <div className="pt-6 mt-6 border-t border-border">
                    <p className="text-xs text-muted-foreground">Open source under the MIT License.</p>
                </div>
            </div>
        </footer>
    );
}

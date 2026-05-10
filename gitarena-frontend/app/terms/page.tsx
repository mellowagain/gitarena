import Link from "next/link";
import { GitBranch, FileText, ChevronRight, ShieldCheck, Ban, Gavel, TriangleAlert, Mail } from "lucide-react";

const tldrPoints = [
    { icon: ShieldCheck, text: "You must be 16 or older to use GitArena. By using the Service you agree to these terms." },
    { icon: ShieldCheck, text: "You own your code. We store it to provide the Service and claim no intellectual property rights over it." },
    { icon: Ban, text: "Do not use GitArena to distribute malware, infringe copyright, or carry out illegal activity." },
    {
        icon: TriangleAlert,
        text: "We may suspend or terminate accounts that violate these terms. We will give notice where it is safe to do so.",
    },
    { icon: Gavel, text: "The Service is provided 'as is'. Our liability is limited to the extent permitted by Swiss law." },
    { icon: Mail, text: "Legal notices and copyright takedown requests go to gitarena_legal@mari.zip." },
];

type Section = { id: string; title: string; content: string[] };

const sections: Section[] = [
    {
        id: "acceptance",
        title: "1. Acceptance of terms",
        content: [
            'By accessing or using GitArena at git.mari.zip (the "Service") you agree to be bound by these Terms of Service ("Terms"). If you do not agree, do not use the Service.',
            "We may update these Terms at any time. Material changes will be communicated by email to registered users at least 14 days before they take effect. Continued use after the effective date constitutes acceptance.",
        ],
    },
    {
        id: "operator",
        title: "2. Operator",
        content: [
            "The Service is operated by a private individual resident in the Canton of Zurich, Switzerland. Contact: gitarena_legal@mari.zip.",
            "These Terms are governed by Swiss law, specifically the Swiss Code of Obligations (OR) and the Swiss Federal Act on Data Protection (DSG), as well as the EU General Data Protection Regulation (GDPR) where applicable. Any dispute shall be submitted to the exclusive jurisdiction of the courts of the Canton of Zurich, Switzerland, unless mandatory consumer-protection law in your jurisdiction provides otherwise.",
        ],
    },
    {
        id: "eligibility",
        title: "3. Eligibility",
        content: [
            "You must be at least 16 years old to use the Service.",
            "By creating an account you confirm that all information you provide is accurate and that you have the legal capacity to enter into this agreement.",
        ],
    },
    {
        id: "accounts",
        title: "4. Accounts",
        content: [
            "You are responsible for maintaining the security of your account credentials. Notify us immediately at gitarena_legal@mari.zip if you suspect unauthorised access to your account.",
            "You may not use another user's account without their explicit permission.",
            "We reserve the right to reclaim usernames that infringe a trademark, are being held in bad faith, or have had no activity for more than 24 months.",
        ],
    },
    {
        id: "your-content",
        title: "5. Your content and intellectual property",
        content: [
            "You retain all intellectual property rights to the content you create and push to the Service. By uploading content you grant us a limited, worldwide, royalty-free licence to store, display, and serve that content solely as necessary to provide the Service to you.",
            "You represent that you have all rights required to upload your content and that it does not infringe the rights of any third party.",
            "We do not use your code or repository content to train machine-learning models or for any purpose beyond operating the Service.",
        ],
    },
    {
        id: "acceptable-use",
        title: "6. Acceptable use",
        content: [
            "You must not use the Service to: distribute malware or malicious code; conduct phishing or fraud; infringe the intellectual property of others; harass or threaten any person; circumvent access controls or exploit vulnerabilities; generate unsolicited bulk communications.",
            "Public repositories must not contain content that is illegal under Swiss law or the law of the Federal Republic of Germany, that is sexually exploitative of minors, or that is designed to facilitate real-world violence.",
            "Automated use (mirroring, scripted cloning, etc.) must stay within reasonable limits. We may rate-limit or suspend access if automated use disrupts the Service for other users.",
        ],
    },
    {
        id: "availability",
        title: "7. Service availability",
        content: [
            "We aim to keep the Service available but cannot guarantee uptime. The Service may be suspended for maintenance, security incidents, or circumstances outside our control.",
            "We are not liable for loss or damage resulting from downtime, data loss, or service interruption.",
        ],
    },
    {
        id: "termination",
        title: "8. Termination",
        content: [
            "You may delete your account at any time from your account settings. Deletion is permanent; your personal data and repository data will be removed within 30 days.",
            "Accounts created without verifying the associated email address within 24 hours will be locked and scheduled for deletion 7 days after registration. You may prevent deletion by verifying your email before the 7-day period expires.",
            "We may suspend or terminate your account if we determine you have violated these Terms. We will aim to give advance notice unless doing so would create a security risk or legal liability.",
            "Provisions that by their nature survive termination — intellectual property, limitation of liability, and governing law — remain in effect after termination.",
        ],
    },
    {
        id: "liability",
        title: "9. Limitation of liability",
        content: [
            "The Service is provided 'as is' and 'as available', without warranties of any kind. We do not warrant that the Service will be uninterrupted, error-free, or free of harmful components.",
            "To the extent permitted by Swiss law, we are not liable for any indirect, incidental, or consequential damages arising from your use of, or inability to use, the Service — including loss of data or loss of revenue.",
            "To the maximum extent permitted by applicable law, our total aggregate liability for any claims arising under or related to these Terms shall be limited to direct damages actually suffered.",
            "Nothing in these Terms limits or excludes liability for gross negligence, intentional misconduct, death or personal injury, or any liability that cannot be lawfully excluded or limited under Swiss law or applicable data protection legislation.",
        ],
    },
    {
        id: "contact",
        title: "10. Contact",
        content: [
            "Legal enquiries and copyright takedown requests: gitarena_legal@mari.zip. For copyright takedowns you may also use the form at git.mari.zip/takedown.",
        ],
    },
];

export default function TermsPage() {
    return (
        <div className="min-h-screen bg-background text-foreground font-sans">
            {/* Nav */}
            <header className="sticky top-0 z-40 border-b border-border bg-background/95 backdrop-blur">
                <div className="max-w-6xl mx-auto px-6 h-12 flex items-center justify-between gap-4">
                    <div className="flex items-center gap-6">
                        <Link href="/" className="flex items-center gap-2 font-semibold text-sm">
                            <GitBranch className="h-4 w-4" />
                            GitArena
                        </Link>
                    </div>
                    <div className="flex items-center gap-2">
                        <Link href="/login" className="px-3 py-1.5 text-sm text-muted-foreground hover:text-foreground transition-colors">
                            Sign in
                        </Link>
                        <Link
                            href="/register"
                            className="px-3 py-1.5 text-sm bg-foreground text-background rounded-md hover:opacity-90 transition-opacity font-medium"
                        >
                            Get started
                        </Link>
                    </div>
                </div>
            </header>

            <main>
                {/* Hero */}
                <section className="border-b border-border">
                    <div className="max-w-3xl mx-auto px-6 py-14">
                        <div className="flex items-center gap-2 text-xs text-muted-foreground mb-4">
                            <Link href="/" className="hover:text-foreground transition-colors">
                                Home
                            </Link>
                            <ChevronRight className="h-3 w-3" />
                            <span>Terms of Service</span>
                        </div>
                        <div className="flex items-start gap-4">
                            <div className="mt-1 h-10 w-10 flex items-center justify-center rounded-md border border-border bg-secondary shrink-0">
                                <FileText className="h-5 w-5 text-muted-foreground" />
                            </div>
                            <div>
                                <h1 className="text-2xl font-semibold tracking-tight mb-1">Terms of Service</h1>
                                <p className="text-sm text-muted-foreground">
                                    Last updated: 2 May 2026 &nbsp;·&nbsp; Effective: 2 May 2026
                                </p>
                            </div>
                        </div>
                    </div>
                </section>

                {/* TL;DR */}
                <section className="border-b border-border bg-secondary/30">
                    <div className="max-w-3xl mx-auto px-6 py-10">
                        <div className="flex items-center gap-2 mb-5">
                            <span className="inline-flex items-center px-2.5 py-0.5 text-xs font-medium tracking-wider uppercase border border-border rounded bg-background text-muted-foreground">
                                TL;DR
                            </span>
                            <p className="text-sm text-muted-foreground">The short version — read the full terms below for details.</p>
                        </div>
                        <div className="grid sm:grid-cols-2 gap-3">
                            {tldrPoints.map(({ icon: Icon, text }) => (
                                <div key={text} className="flex items-start gap-3 px-4 py-3 border border-border rounded-md bg-background">
                                    <Icon className="h-4 w-4 text-muted-foreground shrink-0 mt-0.5" />
                                    <p className="text-sm leading-relaxed text-foreground/80">{text}</p>
                                </div>
                            ))}
                        </div>
                    </div>
                </section>

                {/* Full terms */}
                <section>
                    <div className="max-w-3xl mx-auto px-6 py-12 flex gap-10">
                        {/* TOC */}
                        <aside className="hidden lg:block w-48 shrink-0">
                            <div className="sticky top-20">
                                <p className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-3">Contents</p>
                                <nav className="space-y-1">
                                    {sections.map((s) => (
                                        <a
                                            key={s.id}
                                            href={`#${s.id}`}
                                            className="block text-xs text-muted-foreground hover:text-foreground transition-colors py-0.5"
                                        >
                                            {s.title}
                                        </a>
                                    ))}
                                </nav>
                            </div>
                        </aside>

                        {/* Body */}
                        <div className="flex-1 min-w-0 space-y-10">
                            {sections.map((s) => (
                                <div key={s.id} id={s.id}>
                                    <h2 className="text-base font-semibold mb-3">{s.title}</h2>
                                    <div className="space-y-3">
                                        {s.content.map((para, i) => (
                                            <p key={i} className="text-sm text-muted-foreground leading-relaxed">
                                                {para}
                                            </p>
                                        ))}
                                    </div>
                                </div>
                            ))}

                            <div className="flex items-start gap-3 px-4 py-4 border border-border rounded-md bg-secondary/40">
                                <Mail className="h-4 w-4 text-muted-foreground shrink-0 mt-0.5" />
                                <div>
                                    <p className="text-sm font-medium mb-0.5">Legal enquiries</p>
                                    <p className="text-sm text-muted-foreground">
                                        Email{" "}
                                        <a
                                            href="mailto:gitarena_legal@mari.zip"
                                            className="underline underline-offset-2 hover:text-foreground transition-colors"
                                        >
                                            gitarena_legal@mari.zip
                                        </a>
                                        . For copyright takedown requests use{" "}
                                        <Link
                                            href="/takedown"
                                            className="underline underline-offset-2 hover:text-foreground transition-colors"
                                        >
                                            the takedown form
                                        </Link>
                                        .
                                    </p>
                                </div>
                            </div>
                        </div>
                    </div>
                </section>
            </main>

            {/* Footer */}
            <footer className="border-t border-border">
                <div className="max-w-6xl mx-auto px-6 py-8 flex flex-col md:flex-row items-start md:items-center justify-between gap-6 text-sm text-muted-foreground">
                    <div className="flex items-center gap-2 font-semibold text-foreground">
                        <GitBranch className="h-4 w-4" />
                        GitArena
                    </div>
                    <nav className="flex flex-wrap gap-x-6 gap-y-2">
                        {[
                            { label: "Privacy", href: "/privacy" },
                            { label: "Terms", href: "/terms" },
                            { label: "Takedown", href: "/takedown" },
                        ].map(({ label, href }) => (
                            <Link key={label} href={href} className="hover:text-foreground transition-colors">
                                {label}
                            </Link>
                        ))}
                    </nav>
                    <p className="text-xs">&copy; 2020 - present GitArena. MIT License.</p>
                </div>
            </footer>
        </div>
    );
}

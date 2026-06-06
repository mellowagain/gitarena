import Link from "next/link";
import { GitBranch, Shield, Eye, Database, Lock, Bell, UserX, Mail, Server, ChevronRight } from "lucide-react";

const tldrPoints = [
    {
        icon: Eye,
        text: "We collect your username, email, hashed password, IP address, user agent, SSH keys, passkeys, and the content you push to repositories.",
    },
    {
        icon: Server,
        text: "The frontend runs on Vercel (US) with no persistent storage. The backend is on Tencent Cloud Frankfurt (EU). The database is Aiven PostgreSQL 17 on DigitalOcean Amsterdam (EU). Release assets are stored on Cloudflare R2 (EU).",
    },
    {
        icon: Database,
        text: "Logs are sent to New Relic (EU region) and kept for 30 days. Traces and metrics are kept for 8 days. Infrastructure metadata (no PII) is kept for 395 days. Account data is kept until you delete your account.",
    },
    {
        icon: Lock,
        text: "Passwords are hashed with Argon2id. All traffic is encrypted with TLS",
    },
    {
        icon: Bell,
        text: "We send transactional emails only (verification, security alerts). Emails are delivered via Resend (EU region). No marketing emails unless you ask for them.",
    },
    { icon: UserX, text: "You can delete your account at any time. We will remove your personal data within 30 days. You own your code." },
];

type Section = {
    id: string;
    title: string;
    content: string[];
};

const sections: Section[] = [
    {
        id: "controller",
        title: "1. Controller and applicable law",
        content: [
            'The service git.mari.zip ("GitArena", "the Service") is operated by a private individual resident in the Canton of Zurich, Switzerland ("the Operator").',
            "Because the Operator is based in Switzerland, the Swiss Federal Act on Data Protection (DSG/nDSG, in force since 1 September 2023) applies to the processing of your personal data.",
            "The backend and all persistent data are hosted on a server located in Frankfurt am Main, Germany. Germany is an EU member state, so the General Data Protection Regulation (GDPR, Regulation (EU) 2016/679) also applies to that processing.",
            "Where the two frameworks overlap, this policy applies the stricter requirement. Contact for data-protection matters: gitarena_legal@mari.zip.",
        ],
    },
    {
        id: "what-we-collect",
        title: "2. What we collect and why",
        content: [
            "Account data — username, email address (or addresses), and a hashed password (Argon2id, never stored in plaintext). This is required to create and authenticate your account.",
            "Session data — a random session token stored as a cookie, plus your IP address and the User-Agent string sent by your browser. These are recorded each time you make an authenticated request and are used for security monitoring and abuse prevention.",
            "SSH keys and passkeys — the public half of any SSH key you upload, and the public-key credential for any WebAuthn/passkey you register. Private keys never leave your device.",
            "Repository content — code, commits, and git objects are stored as bare repositories on disk on the backend server. Issues, merge request comments, and repository metadata are stored in the database. This content is used solely to provide the Service and is not used for any other purpose.",
            "Operational telemetry — HTTP request traces, performance metrics, and application logs are exported via OpenTelemetry to New Relic. These records may contain IP addresses and request paths. Retention periods on New Relic: logs 30 days, traces 8 days, metrics 8 days. Infrastructure metadata (host-level, no PII) is retained for 395 days.",
            "We do not use tracking pixels, third-party analytics scripts, or behavioural-advertising technologies of any kind.",
        ],
    },
    {
        id: "legal-basis",
        title: "3. Legal basis for processing",
        content: [
            "Under the GDPR, we process your personal data on the following legal bases:",
            "Contract performance (Art. 6(1)(b) GDPR) — account data, session data, SSH keys, passkeys, and repository content are processed to deliver the Service you signed up for.",
            "Legitimate interests (Art. 6(1)(f) GDPR) — operational telemetry, IP logging, and user-agent logging are processed to detect abuse, maintain security, and diagnose technical issues. You may object to this processing; see section 7.",
            "Under the Swiss DSG, processing is lawful where there is a justifying reason (Rechtfertigungsgrund), including contractual necessity and overriding legitimate interests (Art. 31 DSG), which correspond to the bases above.",
            "We have assessed that these legitimate interests are not overridden by your interests or fundamental rights in the circumstances.",
        ],
    },
    {
        id: "infrastructure",
        title: "4. Infrastructure and sub-processors",
        content: [
            "Frontend — hosted on Vercel, Inc. (US). Vercel serves the static frontend application only. No personal data is stored persistently on Vercel infrastructure; all data requests are proxied to the Frankfurt backend. The transfer of any incidental personal data (such as IP addresses in request logs) to the US is governed by Vercel's Data Processing Agreement incorporating EU Standard Contractual Clauses (Art. 46(2)(c) GDPR).",
            "Backend — the application server is hosted on Tencent Cloud in Frankfurt am Main, Germany (EU/EEA). Repository contents (the actual files, commits, and git objects) are stored on disk on this server as bare git repositories. Tencent Cloud acts as a data processor under Art. 28 GDPR.",
            "Database — account data, sessions, SSH keys, passkeys, issues, merge request comments, and repository metadata are stored in an Aiven-managed PostgreSQL 17 instance running on DigitalOcean in Amsterdam, the Netherlands (EU/EEA). Both Aiven and DigitalOcean act as sub-processors under Art. 28 GDPR.",
            "Observability — logs, metrics, and traces are sent to New Relic, Inc., EU data region (data stored within the EU/EEA). New Relic processes data under a Data Processing Agreement.",
            "Email delivery — transactional emails (account verification, security notices) are sent via Resend, Inc. Resend's SMTP infrastructure is used solely to transmit emails you explicitly trigger. Your email address and the content of those messages are processed by Resend to deliver them. Resend acts as a data processor under Art. 28 GDPR. See resend.com/legal/dpa for their Data Processing Agreement.",
            "Object storage — release assets (files attached to releases, such as binaries and archives) are stored on Cloudflare R2, operated by Cloudflare, Inc. R2 stores data in the EU (Cloudflare's European storage region). Cloudflare acts as a data processor under Art. 28 GDPR. Only files you explicitly upload to a release are stored here; no personal data is derived from them.",
            "We do not sell, rent, or trade personal data to any third party. We will disclose data to law-enforcement authorities only when required by a valid legal process, and will notify you beforehand where permitted by law.",
        ],
    },
    {
        id: "retention",
        title: "5. Data retention",
        content: [
            "Account data (username, email, SSH keys, passkeys, repository content) is retained for as long as your account is active. When you delete your account, we remove your personal data and all repository data within 30 days.",
            "Session records (IP address, user agent, session token) are deleted when a session expires or is revoked, and in any case when the associated account is deleted.",
            "Operational telemetry on New Relic is retained as follows: logs 30 days, traces 8 days, metrics 8 days. Infrastructure metadata (no PII) is retained for 395 days.",
            "Retention may be extended where required by applicable law or to defend against a legal claim.",
        ],
    },
    {
        id: "security",
        title: "6. Security",
        content: [
            "All data in transit is encrypted with TLS 1.2 or higher. Passwords are hashed with Argon2id with a unique per-user salt before storage and are never retrievable in plaintext.",
            "Sessions are identified by a randomly generated token. SSH and WebAuthn private keys never leave your device.",
            "We encourage responsible disclosure of security vulnerabilities. Please report issues to gitarena_legal@mari.zip.",
            "No system is completely immune to attack. Use a strong, unique password and consider registering a passkey or SSH key instead of relying solely on a password.",
        ],
    },
    {
        id: "your-rights",
        title: "7. Your rights",
        content: [
            "Under the GDPR (and the corresponding provisions of the Swiss DSG), you have the following rights regarding your personal data:",
            "Access (Art. 15 GDPR / Art. 25 DSG) — you can request a copy of the personal data we hold about you.",
            "Rectification (Art. 16 GDPR / Art. 32 DSG) — you can correct inaccurate data. Most account data can be updated directly in your account settings.",
            "Erasure (Art. 17 GDPR / Art. 32 DSG) — you can ask us to delete your personal data. You can also delete your account directly from account settings.",
            "Restriction of processing (Art. 18 GDPR) — you can ask us to restrict processing in certain circumstances.",
            "Data portability (Art. 20 GDPR) — you can export your repositories and account data from your account settings at any time.",
            "Objection (Art. 21 GDPR) — you can object to processing based on legitimate interests (see section 3).",
            "To exercise any right, email gitarena_legal@mari.zip. We will respond within 30 days. If you believe we have not handled your data correctly, you may lodge a complaint with the Swiss Federal Data Protection and Information Commissioner (FDPIC), the German supervisory authority (Hessischer Beauftragter für Datenschutz und Informationsfreiheit, for the Frankfurt backend), or the Dutch supervisory authority (Autoriteit Persoonsgegevens, for the Amsterdam database).",
        ],
    },
    {
        id: "cookies",
        title: "8. Cookies",
        content: [
            "We set one cookie: a session cookie (HttpOnly, Secure, SameSite=Lax) that authenticates your session. This cookie is strictly necessary for the Service to function.",
            "We do not set advertising cookies, analytics cookies, or any third-party cookies. No cookie consent banner is shown because we use only strictly necessary cookies.",
        ],
    },
    {
        id: "changes",
        title: "9. Changes to this policy",
        content: [
            "We will notify registered users by email at least 14 days before any material change to this policy takes effect. Minor corrections or clarifications may be made without notice.",
            "Continued use of the Service after a change is effective constitutes acceptance of the updated policy. If you disagree with a change you may delete your account before it takes effect.",
        ],
    },
    {
        id: "contact",
        title: "10. Contact",
        content: ["Data-protection enquiries: gitarena_legal@mari.zip. We will respond within 30 days."],
    },
];

export default function PrivacyPage() {
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
                            <span>Privacy Policy</span>
                        </div>
                        <div className="flex items-start gap-4">
                            <div className="mt-1 h-10 w-10 flex items-center justify-center rounded-md border border-border bg-secondary shrink-0">
                                <Shield className="h-5 w-5 text-muted-foreground" />
                            </div>
                            <div>
                                <h1 className="text-2xl font-semibold tracking-tight mb-1">Privacy Policy</h1>
                                <p className="text-sm text-muted-foreground">
                                    Last updated: 6 June 2026 &nbsp;·&nbsp; Effective: 6 June 2026
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
                            <p className="text-sm text-muted-foreground">The short version — read the full policy below for details.</p>
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

                {/* Full policy */}
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

                            {/* Contact callout */}
                            <div className="flex items-start gap-3 px-4 py-4 border border-border rounded-md bg-secondary/40">
                                <Mail className="h-4 w-4 text-muted-foreground shrink-0 mt-0.5" />
                                <div>
                                    <p className="text-sm font-medium mb-0.5">Questions?</p>
                                    <p className="text-sm text-muted-foreground">
                                        Email{" "}
                                        <a
                                            href="mailto:gitarena_legal@mari.zip"
                                            className="underline underline-offset-2 hover:text-foreground transition-colors"
                                        >
                                            gitarena_legal@mari.zip
                                        </a>{" "}
                                        and we will respond within 30 days.
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

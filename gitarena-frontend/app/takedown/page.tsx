"use client";

import { useState } from "react";
import Link from "next/link";
import { GitBranch, ChevronRight, Scale, CheckCircle2, TriangleAlert, FileSearch, Send } from "lucide-react";

type Step = { title: string; body: string };

const steps: Step[] = [
    {
        title: "Confirm you are the rights holder",
        body: "Only the copyright owner or an agent authorised in writing may submit a takedown notice. Counter-notices may be submitted by the alleged infringer.",
    },
    {
        title: "Identify the infringing content",
        body: "Collect the exact URLs of the repository, file(s), or commit(s) that infringe your copyright. Incomplete or vague notices cannot be acted on.",
    },
    {
        title: "Submit the form below",
        body: "We will acknowledge receipt within 2 business days and act on valid notices within 5 business days.",
    },
];

const whatHappens = [
    "We disable access to the identified content.",
    "We notify the repository owner of the notice.",
    "The owner may submit a counter-notice within 14 days.",
    "If no counter-notice is received, the content remains down. If a valid counter-notice is received, we restore access after 10–14 business days unless you obtain a court order.",
];

type FormState = "idle" | "submitting" | "success" | "error";

export default function TakedownPage() {
    const [form, setForm] = useState({
        yourName: "",
        companyName: "",
        email: "",
        workDescription: "",
        infringingUrls: "",
        originalUrls: "",
        goodFaith: false,
        accuracy: false,
        signature: "",
    });

    const [formState, setFormState] = useState<FormState>("idle");

    function handleChange(e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) {
        const target = e.target;
        const value =
            target instanceof HTMLInputElement && target.type === "checkbox" ? (target as HTMLInputElement).checked : target.value;
        setForm((prev) => ({ ...prev, [target.name]: value }));
    }

    function handleSubmit(e: React.FormEvent) {
        e.preventDefault();
        setFormState("submitting");

        const body = [
            `Full name: ${form.yourName}`,
            form.companyName ? `Company: ${form.companyName}` : null,
            `Reply-to: ${form.email}`,
            "",
            "== Copyrighted work ==",
            form.workDescription,
            "",
            "== Infringing URLs ==",
            form.infringingUrls,
            form.originalUrls ? `\n== Original work location ==\n${form.originalUrls}` : null,
            "",
            "== Declarations ==",
            "Good-faith belief: yes",
            "Accuracy / authorisation: yes",
            "",
            `Electronic signature: ${form.signature}`,
        ]
            .filter((l) => l !== null)
            .join("\n");

        const subject = encodeURIComponent(`Copyright takedown notice — ${form.yourName}`);
        const bodyEncoded = encodeURIComponent(body);
        window.location.href = `mailto:gitarena_legal@mari.zip?subject=${subject}&body=${bodyEncoded}`;

        // Give the mail client a moment to open, then show success state
        setTimeout(() => setFormState("success"), 800);
    }

    const allRequired =
        form.yourName && form.email && form.workDescription && form.infringingUrls && form.goodFaith && form.accuracy && form.signature;

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
                            <span>Copyright Takedown</span>
                        </div>
                        <div className="flex items-start gap-4">
                            <div className="mt-1 h-10 w-10 flex items-center justify-center rounded-md border border-border bg-secondary shrink-0">
                                <Scale className="h-5 w-5 text-muted-foreground" />
                            </div>
                            <div>
                                <h1 className="text-2xl font-semibold tracking-tight mb-1">Copyright Takedown Notice</h1>
                                <p className="text-sm text-muted-foreground">
                                    To report content on GitArena that infringes your copyright, complete the form below. Notices are
                                    processed under the Swiss Copyright Act (URG) and EU Directive 2000/31/EC. We also accept notices
                                    drafted in DMCA format as a courtesy.
                                </p>
                            </div>
                        </div>
                    </div>
                </section>

                {/* How it works */}
                <section className="border-b border-border bg-secondary/30">
                    <div className="max-w-3xl mx-auto px-6 py-10">
                        <h2 className="text-sm font-semibold uppercase tracking-widest text-muted-foreground mb-5">How it works</h2>
                        <div className="space-y-3">
                            {steps.map((step, i) => (
                                <div
                                    key={step.title}
                                    className="flex items-start gap-4 px-4 py-4 border border-border rounded-md bg-background"
                                >
                                    <div className="h-6 w-6 flex items-center justify-center rounded-full bg-secondary border border-border text-xs font-semibold shrink-0 mt-0.5">
                                        {i + 1}
                                    </div>
                                    <div>
                                        <p className="text-sm font-medium mb-0.5">{step.title}</p>
                                        <p className="text-sm text-muted-foreground leading-relaxed">{step.body}</p>
                                    </div>
                                </div>
                            ))}
                        </div>

                        {/* What happens */}
                        <div className="mt-5 px-4 py-4 border border-border rounded-md bg-background">
                            <div className="flex items-center gap-2 mb-3">
                                <FileSearch className="h-4 w-4 text-muted-foreground" />
                                <p className="text-sm font-medium">What happens after submission</p>
                            </div>
                            <ul className="space-y-2">
                                {whatHappens.map((item) => (
                                    <li key={item} className="flex items-start gap-2 text-sm text-muted-foreground leading-relaxed">
                                        <CheckCircle2 className="h-3.5 w-3.5 text-muted-foreground/60 shrink-0 mt-0.5" />
                                        {item}
                                    </li>
                                ))}
                            </ul>
                        </div>

                        {/* Warning */}
                        <div className="mt-4 flex items-start gap-3 px-4 py-3 border border-amber-500/30 rounded-md bg-amber-500/5">
                            <TriangleAlert className="h-4 w-4 text-amber-500 shrink-0 mt-0.5" />
                            <p className="text-sm text-muted-foreground leading-relaxed">
                                Submitting a false or bad-faith notice may expose you to liability for damages under Swiss and EU law. If
                                you are unsure whether the content infringes your copyright, consult a lawyer before proceeding.
                            </p>
                        </div>
                    </div>
                </section>

                {/* Form */}
                <section>
                    <div className="max-w-3xl mx-auto px-6 py-12">
                        {formState === "success" ? (
                            <div className="flex flex-col items-center text-center py-16 gap-4">
                                <div className="h-12 w-12 flex items-center justify-center rounded-full border border-green-500/30 bg-green-500/10">
                                    <CheckCircle2 className="h-6 w-6 text-green-500" />
                                </div>
                                <h2 className="text-lg font-semibold">Email client opened</h2>
                                <p className="text-sm text-muted-foreground max-w-sm leading-relaxed">
                                    Your default mail client should have opened with the notice pre-filled. Send the email to complete the
                                    submission. If nothing opened, email{" "}
                                    <a href="mailto:gitarena_legal@mari.zip" className="underline underline-offset-2 hover:text-foreground">
                                        gitarena_legal@mari.zip
                                    </a>{" "}
                                    directly.
                                </p>
                                <button
                                    onClick={() => {
                                        setFormState("idle");
                                        setForm({
                                            yourName: "",
                                            companyName: "",
                                            email: "",
                                            workDescription: "",
                                            infringingUrls: "",
                                            originalUrls: "",
                                            goodFaith: false,
                                            accuracy: false,
                                            signature: "",
                                        });
                                    }}
                                    className="mt-2 px-4 py-2 text-sm border border-border rounded-md hover:bg-accent/50 transition-colors"
                                >
                                    Submit another notice
                                </button>
                            </div>
                        ) : (
                            <form onSubmit={handleSubmit} className="space-y-8">
                                {/* Claimant info */}
                                <fieldset className="space-y-4">
                                    <legend className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-4">
                                        Claimant information
                                    </legend>
                                    <div className="grid sm:grid-cols-2 gap-4">
                                        <div className="space-y-1.5">
                                            <label htmlFor="yourName" className="text-sm font-medium">
                                                Full name <span className="text-red-500">*</span>
                                            </label>
                                            <input
                                                id="yourName"
                                                name="yourName"
                                                type="text"
                                                required
                                                value={form.yourName}
                                                onChange={handleChange}
                                                placeholder="Jane Smith"
                                                className="w-full h-9 px-3 bg-card border border-border rounded-md text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
                                            />
                                        </div>
                                        <div className="space-y-1.5">
                                            <label htmlFor="companyName" className="text-sm font-medium">
                                                Company / organisation <span className="text-muted-foreground font-normal">(optional)</span>
                                            </label>
                                            <input
                                                id="companyName"
                                                name="companyName"
                                                type="text"
                                                value={form.companyName}
                                                onChange={handleChange}
                                                placeholder="Acme Corp"
                                                className="w-full h-9 px-3 bg-card border border-border rounded-md text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
                                            />
                                        </div>
                                    </div>
                                    <div className="space-y-1.5">
                                        <label htmlFor="email" className="text-sm font-medium">
                                            Email address <span className="text-red-500">*</span>
                                        </label>
                                        <input
                                            id="email"
                                            name="email"
                                            type="email"
                                            required
                                            value={form.email}
                                            onChange={handleChange}
                                            placeholder="jane@example.com"
                                            className="w-full h-9 px-3 bg-card border border-border rounded-md text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
                                        />
                                        <p className="text-xs text-muted-foreground">
                                            We will send acknowledgement and status updates to this address.
                                        </p>
                                    </div>
                                </fieldset>

                                <div className="h-px bg-border" />

                                {/* Content identification */}
                                <fieldset className="space-y-4">
                                    <legend className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-4">
                                        Content identification
                                    </legend>
                                    <div className="space-y-1.5">
                                        <label htmlFor="workDescription" className="text-sm font-medium">
                                            Description of the copyrighted work <span className="text-red-500">*</span>
                                        </label>
                                        <textarea
                                            id="workDescription"
                                            name="workDescription"
                                            required
                                            rows={3}
                                            value={form.workDescription}
                                            onChange={handleChange}
                                            placeholder="Describe the original work that has been infringed — e.g. 'The proprietary source code for Acme's authentication library, version 2.1, registered copyright 2024.'"
                                            className="w-full px-3 py-2 bg-card border border-border rounded-md text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring resize-none leading-relaxed"
                                        />
                                    </div>
                                    <div className="space-y-1.5">
                                        <label htmlFor="infringingUrls" className="text-sm font-medium">
                                            URLs of the infringing content <span className="text-red-500">*</span>
                                        </label>
                                        <textarea
                                            id="infringingUrls"
                                            name="infringingUrls"
                                            required
                                            rows={4}
                                            value={form.infringingUrls}
                                            onChange={handleChange}
                                            placeholder={
                                                "https://git.mari.zip/user/repo/blob/main/src/auth.rs\nhttps://git.mari.zip/user/repo/blob/main/src/lib.rs"
                                            }
                                            className="w-full px-3 py-2 bg-card border border-border rounded-md text-sm font-mono placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring resize-none leading-relaxed"
                                        />
                                        <p className="text-xs text-muted-foreground">
                                            One URL per line. Use the full URL to the specific file(s) or commit(s).
                                        </p>
                                    </div>
                                    <div className="space-y-1.5">
                                        <label htmlFor="originalUrls" className="text-sm font-medium">
                                            Location of the original work{" "}
                                            <span className="text-muted-foreground font-normal">(optional)</span>
                                        </label>
                                        <textarea
                                            id="originalUrls"
                                            name="originalUrls"
                                            rows={2}
                                            value={form.originalUrls}
                                            onChange={handleChange}
                                            placeholder="https://example.com/original-work"
                                            className="w-full px-3 py-2 bg-card border border-border rounded-md text-sm font-mono placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring resize-none leading-relaxed"
                                        />
                                        <p className="text-xs text-muted-foreground">
                                            Link to where the original, authorised work can be found, if publicly accessible.
                                        </p>
                                    </div>
                                </fieldset>

                                <div className="h-px bg-border" />

                                {/* Declarations */}
                                <fieldset className="space-y-4">
                                    <legend className="text-xs font-medium uppercase tracking-widest text-muted-foreground mb-4">
                                        Required declarations
                                    </legend>
                                    <label className="flex items-start gap-3 cursor-pointer">
                                        <input
                                            type="checkbox"
                                            name="goodFaith"
                                            checked={form.goodFaith}
                                            onChange={handleChange}
                                            className="mt-0.5 shrink-0"
                                        />
                                        <span className="text-sm leading-relaxed text-foreground/80">
                                            I have a good-faith belief that the use of the material described above is not authorised by the
                                            copyright owner, its agent, or the law. <span className="text-red-500">*</span>
                                        </span>
                                    </label>
                                    <label className="flex items-start gap-3 cursor-pointer">
                                        <input
                                            type="checkbox"
                                            name="accuracy"
                                            checked={form.accuracy}
                                            onChange={handleChange}
                                            className="mt-0.5 shrink-0"
                                        />
                                        <span className="text-sm leading-relaxed text-foreground/80">
                                            I declare that the information in this notice is accurate and that I am the copyright owner or
                                            am authorised to act on the copyright owner&apos;s behalf.{" "}
                                            <span className="text-red-500">*</span>
                                        </span>
                                    </label>
                                </fieldset>

                                {/* Signature */}
                                <div className="space-y-1.5">
                                    <label htmlFor="signature" className="text-sm font-medium">
                                        Electronic signature <span className="text-red-500">*</span>
                                    </label>
                                    <input
                                        id="signature"
                                        name="signature"
                                        type="text"
                                        required
                                        value={form.signature}
                                        onChange={handleChange}
                                        placeholder="Type your full legal name"
                                        className="w-full h-9 px-3 bg-card border border-border rounded-md text-sm font-mono placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
                                    />
                                    <p className="text-xs text-muted-foreground">Typing your name constitutes an electronic signature.</p>
                                </div>

                                {/* Submit */}
                                <div className="flex items-center gap-4 pt-2">
                                    <button
                                        type="submit"
                                        disabled={!allRequired || formState === "submitting"}
                                        className="flex items-center gap-2 px-5 py-2 text-sm bg-foreground text-background rounded-md hover:opacity-90 transition-opacity font-medium disabled:opacity-40 disabled:cursor-not-allowed"
                                    >
                                        {formState === "submitting" ? (
                                            <>
                                                <svg
                                                    className="h-4 w-4 animate-spin"
                                                    viewBox="0 0 24 24"
                                                    fill="none"
                                                    stroke="currentColor"
                                                    strokeWidth="2"
                                                >
                                                    <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" />
                                                </svg>
                                                Opening mail client…
                                            </>
                                        ) : (
                                            <>
                                                <Send className="h-4 w-4" />
                                                Submit notice
                                            </>
                                        )}
                                    </button>
                                    <p className="text-xs text-muted-foreground">
                                        All fields marked <span className="text-red-500">*</span> are required.
                                    </p>
                                </div>
                            </form>
                        )}
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
                    <p className="text-xs">&copy; 2026 GitArena. AGPL-3.0.</p>
                </div>
            </footer>
        </div>
    );
}

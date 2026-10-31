---
description: Read-only audit of the Next.js frontend for cleanup opportunities (repeated JSX, token drift, prop naming, SWR handling, typography, imports)
argument-hint: [optional: subdirectory or category to focus on]
---

Focus (optional): `$ARGUMENTS`

You are auditing the Next.js frontend codebase (`../..`) for cleanup opportunities. Do NOT make any changes. Only produce a structured report.

For each category below, list specific findings with file paths and line numbers where relevant:

1. **Repeated JSX patterns** — UI patterns that appear 3+ times and should become shared components (cards, badges, empty states, page headers, loading skeletons, etc.)
2. **Inconsistent design tokens** — hardcoded Tailwind values that vary for the same conceptual element (e.g. mixed padding on page containers, mixed text colors for secondary labels)
3. **Duplicated layout logic** — page-level max-width, padding, responsive breakpoints copy-pasted across pages
4. **Inconsistent prop naming** — same concept named differently across components (e.g. `isLoading` vs `loading`, `onClick` vs `onPress`, `label` vs `title`)
5. **Duplicated loading/error handling** — SWR loading and error states handled inline repeatedly rather than in a shared wrapper
6. **Typography inconsistency** — heading levels or font size/weight combos applied ad-hoc rather than from a consistent scale
7. **Redundant or messy imports** — unused imports, missing barrel exports, inconsistent import ordering

Judge findings against the design-guidelines skill (the canonical token/typography reference) — a value that matches the design system is not a finding.

For each finding, note:
- Severity (high / medium / low)
- How many places are affected
- What the fix would look like (one sentence)

End with a top-5 list ordered by (severity × places affected) that could be handed to another agent as implementation tasks.

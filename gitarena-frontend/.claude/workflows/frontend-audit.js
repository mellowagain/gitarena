export const meta = {
  name: 'frontend-audit',
  description: 'Fan out one auditor per cleanup category over gitarena-frontend, verify high-severity findings, return a merged prioritized report',
  whenToUse: 'When asked for a thorough/comprehensive frontend cleanup audit. For a quick single-agent pass use the /frontend-audit command instead.',
  phases: [
    { title: 'Audit', detail: 'one auditor per category' },
    { title: 'Verify', detail: 'adversarially check high-severity findings' },
  ],
}

const CATEGORIES = [
  { key: 'repeated-jsx', prompt: 'UI patterns repeated 3+ times that should become shared components (cards, badges, empty states, page headers, loading skeletons)' },
  { key: 'design-tokens', prompt: 'hardcoded Tailwind values that vary for the same conceptual element, or raw color utilities (bg-gray-800) instead of semantic tokens (bg-card, text-muted-foreground)' },
  { key: 'layout-duplication', prompt: 'page-level max-width, padding, and responsive breakpoints copy-pasted across pages instead of shared' },
  { key: 'prop-naming', prompt: 'the same concept named differently across component props (isLoading vs loading, label vs title, onClick vs onPress)' },
  { key: 'swr-handling', prompt: 'SWR loading/error states handled inline repeatedly rather than via a shared wrapper; useState mirrors of SWR data; direct fetch calls bypassing useSWR' },
  { key: 'typography', prompt: 'heading levels and font size/weight combos applied ad-hoc instead of following the text hierarchy in .claude/skills/design-guidelines/SKILL.md' },
  { key: 'imports', prompt: 'unused imports, inconsistent import ordering, missing barrel exports' },
]

const FINDINGS_SCHEMA = {
  type: 'object',
  required: ['findings'],
  properties: {
    findings: {
      type: 'array',
      items: {
        type: 'object',
        required: ['title', 'severity', 'files', 'occurrences', 'fix'],
        properties: {
          title: { type: 'string' },
          severity: { type: 'string', enum: ['high', 'medium', 'low'] },
          files: { type: 'array', items: { type: 'string' }, description: 'repo-relative paths, with :line where relevant' },
          occurrences: { type: 'number' },
          fix: { type: 'string', description: 'one-sentence fix description' },
        },
      },
    },
  },
}

const VERDICT_SCHEMA = {
  type: 'object',
  required: ['isReal', 'reason'],
  properties: { isReal: { type: 'boolean' }, reason: { type: 'string' } },
}

const audited = await pipeline(
  CATEGORIES,
  c => agent(
    `You are auditing the Next.js frontend at gitarena-frontend/ for ONE category of cleanup opportunity: ${c.prompt}.
Read .claude/skills/design-guidelines/SKILL.md first — anything that matches the design system is NOT a finding.
Do NOT make changes; this is read-only. Report every distinct finding with file paths (and line numbers where relevant), how many places are affected, a severity (high/medium/low), and a one-sentence fix.`,
    { label: `audit:${c.key}`, phase: 'Audit', schema: FINDINGS_SCHEMA },
  ),
  (result, c) => parallel((result?.findings ?? []).map(f => () =>
    f.severity !== 'high'
      ? Promise.resolve({ ...f, category: c.key, verified: true })
      : agent(
          `Adversarially verify this frontend-audit finding in gitarena-frontend/. Try to REFUTE it — e.g. the "duplication" is actually intentional variation, the files were misread, or the design-guidelines skill sanctions the value. Finding: ${f.title}. Files: ${f.files.join(', ')}. Claimed fix: ${f.fix}`,
          { label: `verify:${c.key}`, phase: 'Verify', schema: VERDICT_SCHEMA },
        ).then(v => ({ ...f, category: c.key, verified: v?.isReal ?? true, verifyReason: v?.reason })),
  )),
)

const findings = audited.filter(Boolean).flat().filter(Boolean).filter(f => f.verified)
const order = { high: 0, medium: 1, low: 2 }
findings.sort((a, b) => (order[a.severity] - order[b.severity]) || (b.occurrences - a.occurrences))

log(`${findings.length} verified findings across ${CATEGORIES.length} categories`)
return {
  findings,
  top5: findings.slice(0, 5).map(f => `[${f.category}] ${f.title} — ${f.fix}`),
}

---
description: Remove AI code slop from a diff (default: uncommitted vs HEAD; pass "main", "branch", or "task" to change scope)
argument-hint: [HEAD|main|branch|task]
---

Scope argument: `$ARGUMENTS`

Determine the diff to check based on the scope argument:
- empty or `HEAD` → the uncommitted diff against HEAD
- `main` → the full diff of this branch (including uncommitted changes) against main's remote HEAD (`origin/main`)
- `branch` → all commits on this branch since it diverged from main
- `task` → only the files you edited in your latest task this session

Check that diff and remove all AI generated slop introduced in it.

This includes:

- Extra comments that a human wouldn't add or that are inconsistent with the rest of the file
- Extra defensive checks or try/catch blocks that are abnormal for that area of the codebase (especially if called by trusted / validated codepaths)
- Casts to `any` to get around type issues
- Fallbacks (`?? defaultValue`, `unwrap_or_default()`) that hide errors instead of surfacing them
- Helper functions that duplicate what an existing util, crate, or std API already does
- Any other style that is inconsistent with the file
- Unnecessary emoji usage

Only touch lines that are part of the checked diff — do not "improve" pre-existing code.

Report at the end with only a 1-3 sentence summary of what you changed.

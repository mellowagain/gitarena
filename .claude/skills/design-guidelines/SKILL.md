---
name: design-guidelines
description: Applies GitArena's official design system, colors, and typography to frontend components and pages. Use it when building new UI, styling components, or ensuring consistency with the established dark monochromatic visual identity.
---

# GitArena Design System

## Overview

GitArena follows a near-black monochromatic design language inspired by Linear and modern developer tools. The UI is dark-only, achromatic, and relies on luminance contrast rather than color to establish hierarchy.

**Keywords**: design system, dark theme, monochromatic, styling, brand colors, typography, UI components, visual identity, shadcn/ui, tailwind, Linear-style

## Technology Stack

| Layer         | Technology                                              |
|---------------|---------------------------------------------------------|
| Framework     | Next.js 16 (App Router, React Server Components)        |
| Styling       | Tailwind CSS v4 (CSS-only config via `@theme inline`)   |
| Components    | shadcn/ui (`new-york` style, `neutral` base)            |
| Icons         | Lucide React                                            |
| Fonts         | Geist (sans) + Geist Mono (loaded via Next.js built-in) |
| Color space   | oklch throughout                                        |
| Class merging | `cn()` utility from `clsx` + `tailwind-merge`           |

## Brand Guidelines

### Color System

The entire palette is achromatic (chroma `0` in oklch) except for the destructive red. Both `:root` and `.dark` define identical tokens -- the app is dark-only by design.

**Surface Colors (background layers, darkest to lightest):**

- Background: `oklch(0.09 0 0)` -- Page-level background, near-black
- Card: `oklch(0.12 0 0)` -- Elevated surface (cards, panels)
- Input/Secondary/Muted: `oklch(0.14 0 0)` -- Input backgrounds, secondary surfaces
- Popover: `oklch(0.14 0 0)` -- Dropdowns, popovers, overlays
- Sidebar Accent: `oklch(0.15 0 0)` -- Active sidebar items
- Accent: `oklch(0.18 0 0)` -- Hover states, highlighted rows
- Sidebar Border: `oklch(0.2 0 0)` -- Sidebar dividers

**Text Colors (lightest to dimmest):**

- Sidebar Primary/Foreground: `oklch(0.95 0 0)` -- Brightest text, sidebar emphasis
- Foreground/Primary: `oklch(0.93 0 0)` -- Primary body text
- Secondary Foreground: `oklch(0.8 0 0)` -- Secondary text
- Muted Foreground: `oklch(0.5 0 0)` -- Tertiary text, placeholders, timestamps

**Border and Focus:**

- Border: `oklch(1 0 0 / 0.08)` -- 8% white opacity, subtle dividers
- Ring: `oklch(1 0 0 / 0.2)` -- 20% white opacity, focus rings

**Semantic Colors:**

- Destructive: `oklch(0.55 0.18 25)` -- The only chromatic color; red for errors, delete actions, sign-out
- Notification dot: `blue-500` (Tailwind utility) -- Used sparingly for unread indicators

**Chart Colors (achromatic gradient):**

- Chart 1: `oklch(0.7 0 0)`
- Chart 2: `oklch(0.6 0 0)`
- Chart 3: `oklch(0.5 0 0)`
- Chart 4: `oklch(0.4 0 0)`
- Chart 5: `oklch(0.3 0 0)`

### Typography

- **Sans-serif (body/headings):** `Geist` with `Geist Fallback`
- **Monospace (code, usernames):** `Geist Mono` with `Geist Mono Fallback`
- **Base font size:** `18px` (112.5% scale, optimized for 4K readability)
- **Antialiasing:** `-webkit-font-smoothing: antialiased` on body

**Text Hierarchy Patterns:**

| Role                  | Classes                                                                            |
|-----------------------|------------------------------------------------------------------------------------|
| Page heading          | `text-4xl font-bold tracking-tight` (up to `text-6xl` on lg)                       |
| Section heading       | `text-3xl font-bold tracking-tight sm:text-4xl`                                    |
| Card title            | `font-semibold leading-none`                                                       |
| Section label         | `text-xs font-medium uppercase tracking-widest text-muted-foreground`              |
| Body text             | `text-sm` or `text-base` depending on context                                      |
| Description/secondary | `text-sm text-muted-foreground` or `text-sm leading-relaxed text-muted-foreground` |
| Stat/metric value     | `text-3xl font-bold tracking-tight text-foreground`                                |
| Monospace/username    | `font-mono text-xs text-muted-foreground`                                          |
| Keyboard shortcut     | `text-[11px] text-muted-foreground bg-secondary rounded border border-border`      |

### Spacing and Layout

- **Border radius:** `0.5rem` base (`--radius`), computed variants: `sm` (-4px), `md` (-2px), `lg` (base), `xl` (+4px)
- **Page max-width:** `max-w-6xl` (72rem) for landing pages
- **Content padding:** `px-6` horizontal, `py-24` or `py-16` vertical for sections
- **Card internal padding:** `px-6 py-6` with `gap-6` between sections
- **Header height:** `h-14` (app) or `h-16` (landing)
- **Interactive element height:** `h-9` (default), `h-8` (sm), `h-10` (lg)
- **Icon button size:** `size-9` (default), `size-8` (sm), `size-10` (lg)

### Borders and Dividers

- All borders use the `border-border` token (`oklch(1 0 0 / 0.08)`)
- Section dividers: `border-t border-border` or `border-b border-border`
- Grid dividers via gap technique: `gap-px bg-border` on parent with `bg-card` on children
- Vertical separators: `w-px h-7 bg-border`
- Dashed borders for empty states: `border-dashed`

### Transitions and Motion

- **Global interactive transition:** `all 180ms ease` on `button`, `a`, `input`, `[role="button"]`
- **Color-only transitions:** `transition-colors` for links and non-elevated elements
- **Box-shadow transitions:** `transition-[color,box-shadow]` for inputs and badges
- **Full transitions:** `transition-all` for buttons

### Shadows

- Cards: `shadow-sm`
- Inputs: `shadow-xs`
- Decorative product previews: `shadow-2xl shadow-black/20`
- Decorative background glow: gradient with `blur-xl opacity-50`

## Component Patterns

### shadcn/ui Convention

All UI primitives use the `data-slot` attribute for identification and follow the function-component pattern (no `forwardRef`, using `React.ComponentProps`):

```tsx
function ComponentName({ className, ...props }: React.ComponentProps<"div">) {
    return (
        <div
            data-slot="component-name"
            className={cn("base-classes", className)}
            {...props}
        />
    );
}
```

### Button Variants

| Variant       | Usage                                          |
|---------------|------------------------------------------------|
| `default`     | Primary actions (white bg, dark text)          |
| `destructive` | Dangerous actions (red bg)                     |
| `outline`     | Secondary actions with border                  |
| `secondary`   | Less prominent actions (dark bg, gray text)    |
| `ghost`       | Minimal, hover-only actions (nav links, icons) |
| `link`        | Inline text links with underline on hover      |

### Card Pattern

Cards use `bg-card` with `border` and `rounded-xl`. Internal spacing uses `px-6` and `gap-6`. Sub-components: `CardHeader`, `CardTitle`, `CardDescription`, `CardAction`, `CardContent`, `CardFooter`.

### Empty States

Centered layout with `border-dashed`, vertically stacked icon + title + description + action. Use the `Empty`, `EmptyHeader`, `EmptyMedia`, `EmptyTitle`, `EmptyDescription`, `EmptyContent` composable.

### Navigation

- **App header:** Sticky, `bg-background`, `h-14`, breadcrumb with `/` separators
- **Landing header:** Sticky, `bg-background/80 backdrop-blur-xl`, `h-16`
- **Logo:** Text-based "GITARENA" (`text-base font-semibold tracking-tight`) in app; `G` icon block + "GitArena" on landing
- **Nav links:** `text-muted-foreground hover:text-foreground transition-colors`
- **Active nav links:** `text-foreground`

### Hover States

- Buttons: opacity reduction (`hover:bg-primary/90`) or background shift (`hover:bg-accent`)
- Links: `hover:text-foreground` from `text-muted-foreground`
- Rows/items: `hover:bg-accent/50` or `hover:bg-accent/30`
- Icon buttons: `hover:text-foreground hover:bg-accent/50`
- Avatar: `hover:ring-2 hover:ring-ring`

### Focus States

- Visible focus ring: `focus-visible:ring-[3px] focus-visible:ring-ring/50 focus-visible:border-ring`
- Invalid state: `aria-invalid:ring-destructive/20 aria-invalid:border-destructive`

### Feature Grid Pattern

Use `gap-px bg-border` on the grid parent with `bg-card` cells for a border-separated grid layout. Each cell contains an icon, heading, and description.

## File Structure

```
gitarena-frontend/
  app/
    globals.css          # Theme tokens and base styles (source of truth)
    layout.tsx           # Root layout with font and analytics
  components/
    ui/                  # 48 shadcn/ui primitives (new-york style)
    landing/             # 7 landing page sections
    top-bar.tsx          # App navigation header
    client-layout.tsx    # SWR + config providers
    ...
  lib/
    utils.ts             # cn() class merge utility
  public/                # Favicons and placeholder images
  components.json        # shadcn/ui configuration
```

## Do's and Don'ts

**Do:**
- Use the semantic CSS variable tokens (`bg-card`, `text-muted-foreground`, etc.) instead of raw oklch values
- Use `cn()` for all className merging
- Keep the achromatic palette -- express hierarchy through luminance, not hue
- Use `text-sm` as the default text size (renders at ~14.4px given the 18px base)
- Use Lucide icons consistently at `w-4 h-4` (inline) or `w-5 h-5` (standalone)
- Use `transition-colors` for color-only transitions and `transition-all` only when needed
- Apply `tracking-tight` to headings

**Don't:**
- Introduce new chromatic/colored accents beyond destructive red without deliberate intent
- Use light mode tokens -- the design is dark-only
- Skip the `data-slot` attribute when creating new UI primitives
- Use `forwardRef` -- use `React.ComponentProps<>` instead (React 19+ pattern)
- Override the 18px base font size in components
- Use raw Tailwind color utilities (e.g., `bg-gray-800`) instead of the design tokens

# UI Optimization Plan: "Normal" Aesthetic

The goal is to remove the "AI/Cyber" aesthetic (gradients, glassmorphism, glowing effects, complex animations) and return to a pragmatic, professional, and functional UI (similar to VS Code or GitHub Dark Mode).

## 1. Refactor Global Styles (`src/style.css`)

### Color Palette Overhaul
- **Backgrounds**: Switch from `radial-gradient` and deep blacks to solid, professional dark grays.
  - `--bg-primary`: `#0d1117` (Main background)
  - `--bg-secondary`: `#161b22` (Card/Panel background)
  - `--bg-tertiary`: `#21262d` (Hover states)
- **Accents**: Replace neon/gradient accents with solid, readable colors.
  - `--accent-primary`: `#1f6feb` (Standard Blue)
  - `--accent-hover`: `#388bfd`
- **Borders**: Use solid, subtle borders.
  - `--border-color`: `#30363d`
  - `--border-hover`: `#8b949e`

### Component Resets
- **Buttons**:
  - Remove expanding circle animations (`::before`).
  - Remove gradient backgrounds.
  - Use solid colors with standard hover states.
  - Reduce border radius from `999px` (pill) or `12px` to `6px` (standard rounded).
- **Cards**:
  - Remove `backdrop-filter` (glassmorphism).
  - Remove `float` animation on hover.
  - Use solid background colors (`--bg-secondary`).
  - Use standard borders.
- **Inputs**:
  - Remove `glow` animation on focus.
  - Remove glassmorphism.
  - Use solid backgrounds.

### Animation Removal
- Delete `@keyframes glow`, `float`, `shimmer`, `spin` (keep spin for loading only).
- Simplify transitions to `0.2s ease`.

## 2. Update Dashboard View (`src/views/DashboardView.vue`)

- **Page Background**: Remove the `radial-gradient` in `.page`.
- **Header**: Remove `backdrop-filter` and complex gradients. Use a solid header color with a border.
- **Search Box**:
  - Remove pill shape (`border-radius: 999px` -> `6px`).
  - Remove complex shadows and transparency.
- **Category Cards**:
  - Remove "Cyber" gradients (`radial-gradient`, `linear-gradient`).
  - Remove `transform` scaling effects that are too bouncy.
  - Simplify icon containers (remove glowing borders).
- **AI Toggle Button**:
  - Make it a standard floating action button or integrated toolbar button, removing the "glow/pulse" effects.

## 3. Update AI Assistant Panel (`src/components/AiAssistantPanel.vue`)

- Ensure the panel background matches the new `--bg-secondary`.
- Remove any custom scrollbar styling that clashes with the new system.
- Ensure buttons (Send, New Session) match the new flat/solid style.

## 4. Execution Steps

1.  **Modify `src/style.css`**: Complete rewrite of variables and base component classes.
2.  **Modify `src/views/DashboardView.vue`**: Strip out local "cyber" styles.
3.  **Review**: Check for any remaining "flashy" elements.

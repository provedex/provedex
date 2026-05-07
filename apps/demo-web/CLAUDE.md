# apps/demo-web - design tokens and UI rules

Vanilla HTML, CSS, JS. No build step. Served statically by `crates/provedex-server/` on port 3000. Aesthetic target: Linear, Vercel, Sigstore Rekor explorer. Not a marketing landing page.

## Design tokens (do not extend without approval)

| Token | Value | Use |
|-------|-------|-----|
| `--bg` | `#0d1117` | page background |
| `--bg-2` | `#11161d` | pane headers, hover row |
| `--fg` | `#e6edf3` | primary text |
| `--fg-muted` | `#6e7681` | labels, secondary |
| `--fg-dim` | `#484f58` | timestamps, tertiary |
| `--border` | `#21262d` | thin separators |
| `--border-strong` | `#30363d` | button borders |
| `--accent` | `#58a6ff` | accent / event-type highlight |
| `--pass` | `#3fb950` | verify-pass status |
| `--fail` | `#f85149` | verify-fail status, broken-row highlight |

No other colors. No gradients. No drop shadows. No glassmorphism.

## Typography

- Inter (loaded from rsms.me) for UI chrome.
- JetBrains Mono for hashes, timestamps, event types, session/pubkey badges.
- 13px base. Tight line-height (1.45).

## Layout

- 44px header / 1fr main / 48px footer. No outer scroll.
- Two-column main: 40% conversation pane / 60% event stream.
- Event row grid: `seq | type | self_hash | timestamp` with fixed widths.
- Footer action bar with verify, tamper, export buttons left-aligned, status text right-aligned.

## Animation budget

- Event row 200ms `fade-in` only.
- Verify result: instant color flip in footer. No transition.
- Nothing else animates.

## Buttons

- 1px border, 3px radius max.
- `lowercase` text, no icons.
- Hover inverts: `bg` becomes `fg`, color becomes `bg`.
- Primary button (`btn-primary`): accent border + accent text. Filled on press.
- Recording state: filled red.

## Mic input

- Mouse hold-to-talk on the button.
- Spacebar push-to-talk. Skip auto-repeat. Skip if focus is on `input`/`textarea`.

## Strict no-go

- No emojis anywhere in the DOM, any file.
- No icons (no SVG, no font-icon, no emoji icons). Text labels only.
- No special unicode characters (no en/em dash, no arrow, no middle dot, no curly quotes).
- No Tailwind component patterns. Tailwind via CDN is allowed only if every utility class on the page is project-defined; in practice the entire UI uses custom CSS in `style.css`.
- No third-party JS frameworks (React, Vue, Svelte, jQuery). Vanilla only until an ADR overrides this.

## Accessibility floor

- Every interactive control reachable via keyboard.
- Color contrast: foreground on bg-2 must hit AA. Don't lower contrast for "polish".
- Live region for the verify result so screen readers announce status changes.

## File responsibility

- `index.html` - structure and DOM hooks.
- `style.css` - all visual rules. CSS variables at top, tokens defined once.
- `app.js` - all behavior. Single file. No bundler. Plain ES2020.

## Server contract

- API at `/api/*`. SSE at `/api/events` (event name `signed`).
- POST `/api/chat` with multipart `audio` field. Response JSON has `transcript`, `response_text`, optional `response_audio_b64`, `tts_available`.
- POST `/api/verify` returns `ChainReport` JSON.
- POST `/api/tamper-test` returns `{ tampered_seq, event_count }`.
- POST `/api/export` returns the bundle as a JSON download.

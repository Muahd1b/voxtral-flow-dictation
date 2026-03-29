# Capability Parity Matrix: Wispr Flow -> Local Rust Product

Date: 2026-03-29

## Capability Mapping
| Capability Group | Wispr Flow Reference Behavior | Local Rust Target | Gap Level |
|---|---|---|---|
| Dictation speed/UX | Fast push-to-talk style insertion, app-wide usage | Low-latency always-on + push-to-talk in TUI, then global hook | Medium |
| Quiet speech support | Works in discreet/quiet speaking mode | Depends on model/VAD tuning; support target via calibration | Medium |
| Cross-app insertion | Pasting/insertion in many apps with fallbacks | Phase 1: Codex deterministic routing. Phase 2: app insertion adapter | High |
| IDE awareness | Variable recognition + file tagging in Cursor/Windsurf | Phase 2 parser/context adapter for open files + filename tagging | High |
| Command Mode | Rewrite/edit selected text, question mode | Local command transform pipeline (rewrite/summarize/format) | High |
| Dictionary/snippets | Shared + personal shortcuts | Local dictionary + snippets with profile storage | Medium |
| Privacy mode | Zero retention options and admin controls | Local-only default with retention controls and purge | Low |
| Team/compliance admin | Enterprise controls, policy enforcement | Out of v1 scope; local single-user first | Very High |

## Required Parity Scope (Desktop Functional Parity)
Included for this product:
- Always-on dictation with robust turn segmentation.
- Deterministic delivery into selected Codex session.
- Local transcript cleanup and style presets.
- Local command mode for selected text workflows.
- Local dictionary/snippets.
- Reliability and observability comparable to daily production use.

Not included in v1:
- Mobile apps.
- Organization/team admin/compliance workflows.
- Hosted analytics backends.

## Priority Order
P0:
- Deterministic routing.
- VAD-based always-on turn segmentation.
- Stable TUI control plane and health states.

P1:
- Post-processing pipeline (punctuation, fillers, style).
- Command mode (local transform engine).
- Dictionary/snippets and developer context helpers.

P2:
- Cross-app insertion adapter.
- Packaging + installer + migration tooling.

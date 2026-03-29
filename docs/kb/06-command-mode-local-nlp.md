# Local Command Mode and Text Processing

Date: 2026-03-29

## Reference Behavior to Emulate
From Wispr docs, command mode supports:
- text transforms on highlighted content,
- summarization/translation/rewrite intents,
- query mode behavior when no selection.

## Local-First Command Mode Scope (v1)
Supported intents:
- `rewrite(style=concise|formal|casual)`
- `summarize(length=short|medium)`
- `translate(target_lang)`
- `expand(detail_level)`
- `bulletize`

Out of scope for v1:
- remote search integration in command mode by default.

## Architecture
- `command_parser`: maps spoken command to structured intent.
- `transform_engine`: local text rewrite pipeline.
- `selection_adapter`: gets selected text from active context (TUI/terminal first).

## Minimal NLP Stack
- Rule-based parser for high-confidence command intents.
- deterministic transforms where possible.
- optional local LLM adapter (feature-gated) for open-ended rewrites.

## Developer-aware Enhancements
- preserve code fences and inline backticks.
- avoid changing token case in recognized code identifiers.
- preserve filenames with extension (e.g., `main.rs`, `.env`).

## Safety Controls
- preview-before-apply option for destructive rewrites.
- one-step undo buffer per target context.

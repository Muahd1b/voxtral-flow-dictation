# Delivery & Routing: Deterministic Codex Session Targeting

Date: 2026-03-29

## Problem
Any implicit fallback to "last" can route text into the wrong conversation.

## Delivery Contract
- Delivery target must be explicitly selected session ID.
- If selected session is missing/invalid, block delivery and show actionable error.
- No hidden fallback unless user explicitly enables fallback mode.

## Codex CLI Interface (current)
Command:
- `codex exec resume [SESSION_ID] [PROMPT]`
- `PROMPT` can be `-` to read stdin.

Important options from CLI help:
- `--last` resumes most recent session.
- `--skip-git-repo-check` allows operation outside git repo.

## Session Store Contract
Source path pattern:
- `~/.codex/sessions/YYYY/MM/DD/*.jsonl`

Observed first line format:
- record type `session_meta` containing `id`, `timestamp`, and `cwd`.

## Routing Algorithm (Proposed)
1. Read selected receiver ID from local state file.
2. Validate UUID format.
3. Verify existence in session index.
4. Execute `codex exec resume <id> -` with transcript stdin.
5. Store `DeliveryResult` with return code + stderr excerpt.

## Failure Handling
- Exit non-zero: show `codex` stderr in talk pane.
- Session not found: block and prompt reselection.
- Codex binary missing: health state red + setup hint.

## Idempotency / Safety
- Attach `utterance_id` in internal logs for each delivery attempt.
- Keep delivery retry count configurable (default 0 for Codex mode).

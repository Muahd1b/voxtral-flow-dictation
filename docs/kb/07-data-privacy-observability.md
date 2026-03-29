# Data, Privacy, and Observability Design

Date: 2026-03-29

## Local Data Model
Store locally:
- app config (`config.toml`)
- dictionary (`dictionary.json` or sqlite table)
- snippets (`snippets.json` or sqlite table)
- session selection state
- structured logs and metrics

## Privacy Defaults
- Local-only ASR path enabled by default.
- No cloud API calls unless user explicitly enables external providers.
- Config flag to disable transcript persistence completely.

## Retention Policy
- Default log retention: 7 days.
- Optional transcript retention: off by default for sensitive mode.
- One-command purge for all local transcript artifacts.

## Observability Stack
- `tracing` + `tracing-subscriber` with JSON output option.
- Metrics per stage:
  - capture latency,
  - vad decision latency,
  - asr latency,
  - delivery latency,
  - error counts by class.

## Event Schema (Proposed)
- `event_type`
- `utterance_id`
- `session_id`
- `stage`
- `status`
- `latency_ms`
- `error_code`
- `ts`

## SQLite Note
If sqlite is used for state/metrics:
- use transactions for atomic writes.
- rollback-on-failure behavior should remain default.

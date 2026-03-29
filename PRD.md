# PRD: Local WhisperFlow for Codex

Status: Draft v1
Date: 2026-03-29
Owner: Jonas
Workspace: /Users/jonasknppel/dev/codex-asr-bridge

## 1. Product Summary
Build a fully local, privacy-first voice dictation product with a Whisper-class ASR core and a reliable "speak into current Codex session" workflow. The product should combine always-on dictation, low-latency transcript insertion, developer-aware formatting, and robust session routing in one interface.

## 2. Problem Statement
Current ASR bridge can transcribe locally and route text to Codex sessions, but it does not yet provide the reliability, UX polish, and feature depth of Wispr Flow.

Current user pain:
- Session routing can feel non-deterministic when delivery path changes.
- Live insertion path depends on terminal automation that can fail on app detection/focus.
- Always-on mode exists but uses fixed chunking (not true VAD turn-taking).
- No full command-mode rewriting, deep personalization, or app-level quality UX.

## 3. Goals
Primary goals:
- Deterministic "current session" delivery for Codex (no random target).
- Fully local speech-to-text path by default.
- Smooth always-on dictation with voice activity detection (VAD).
- Single control surface to start/stop ASR engine, select target session, view logs, and speak.
- Near-Wispr developer workflow quality for coding terminals/IDEs.

Secondary goals:
- Local command-mode rewriting for selected text.
- Personal dictionary, snippets, and style preferences.
- High reliability fallbacks for text insertion.

## 4. Non-Goals (v1)
- Mobile apps (iOS/Android).
- Team billing/admin dashboards.
- Cloud-dependent AI formatting as default path.
- Enterprise governance controls (SSO/SCIM/HIPAA workflows) in initial release.

## 5. Target Users
- Primary: Solo developer using Codex CLI daily on macOS.
- Secondary: Technical users dictating into IDEs, terminals, docs, and chats.

Jobs-to-be-done:
- "I want to talk naturally and have it land in the exact Codex session I selected."
- "I want fast local dictation without sending audio to cloud providers."
- "I want minimal friction: one interface, one toggle, clear logs, predictable behavior."

## 6. Current State (As-Is)
Implemented in this workspace:
- Local Whisper (MLX) transcription backend via FastAPI + WebSocket.
- Receiver session selection persisted in `.codex_receiver_session`.
- Hub TUI with three panes (sessions, talk, server) and start/stop server controls.
- Single-shot and always-on loop recording modes.
- Codex forwarding via `codex exec resume` for selected session.

Known limitations:
- Always-on uses fixed 5s chunks rather than VAD turn segmentation.
- Injection/forwarding modes have changed over time and created confusion.
- No true app-agnostic insertion manager with robust fallback order.
- No command mode, personalized style transforms, or quality post-processing stack.
- No structured observability for latency/error budgets.

## 7. Gap Analysis vs Wispr Flow

### 7.1 Capability Matrix
| Capability | Wispr Flow (reference) | Current ASR Bridge | Gap | Priority |
|---|---|---|---|---|
| Whisper/quiet speech dictation UX | Strong productized UX | Raw model capability only | UX + turn handling | P0 |
| Always-on dictation | Mature | Fixed chunk loop | VAD, endpointing, interruption | P0 |
| Insertion reliability across apps | Mature + fallback UX | Codex-focused; terminal-dependent paths | Insertion engine + fallback hierarchy | P0 |
| Deterministic target routing | Mature active app flow | Session-file + explicit forwarding | Needs strict routing guarantees + tests | P0 |
| Real-time cleanup/punctuation polish | Product feature | Minimal | Post-processing pipeline | P1 |
| Developer context (variables/files) | IDE-aware features | None | IDE context adapter | P1 |
| Command mode rewrite | Built-in | None | Local rewrite engine | P1 |
| Personal dictionary/snippets | Built-in | None | User profile layer | P1 |
| Team/compliance/admin | Strong enterprise features | None | Out of scope v1 | P3 |

### 7.2 Most Critical Gaps (P0)
- Turn detection: no VAD-based utterance boundaries.
- Delivery guarantees: not enough contract tests for "selected session only".
- Insertion robustness: no unified insertion strategy with fallback and clear state.
- Operability: no SLO-driven telemetry (latency, drop rate, routing errors).

## 8. Product Requirements

### 8.1 Functional Requirements
FR-1 Session Router:
- User can pick one receiver session.
- Every delivered transcript must target that exact session unless user explicitly changes it.
- If no receiver session selected, delivery is blocked with explicit error.

FR-2 Always-On Dictation:
- Toggle on/off from main interface.
- Use VAD to detect speech start/end and emit utterance-level transcripts.
- Support interruption and immediate stop.

FR-3 Delivery Modes:
- Mode A (default for Codex): direct session forwarding via `codex exec resume <session-id> -`.
- Mode B: active-app insertion engine for non-Codex text fields.
- User can switch mode explicitly.

FR-4 Single Unified Interface:
- Pane 1: receiver/session control.
- Pane 2: live speech/transcript stream.
- Pane 3: server/process/log/health.
- Clear state badges: listening, transcribing, forwarding, success, failed.

FR-5 Post-Processing (Local):
- Optional punctuation and filler cleanup.
- Optional style presets (concise, formal, coding).
- Toggle per user.

FR-6 Personalization:
- Local custom dictionary.
- Snippets/expansions.
- Optional term boosting for technical vocabulary.

FR-7 Observability:
- Structured local logs with event IDs.
- Metrics for ASR latency, delivery latency, failure reason, retry count.

### 8.2 Non-Functional Requirements
NFR-1 Privacy:
- Default audio/text path remains local on device.
- No cloud egress unless explicitly enabled by user.

NFR-2 Performance:
- End-of-speech to transcript display p95 <= 1200 ms on Apple Silicon M-series.
- End-of-speech to session delivery p95 <= 1800 ms.

NFR-3 Reliability:
- Session routing correctness >= 99.9% in automated tests.
- Delivery success rate >= 99% for valid selected session and healthy backend.

NFR-4 Usability:
- All core actions accessible from keyboard.
- User can recover from any error without restarting app.

## 9. Proposed Architecture (Local-First)

### 9.1 Components
- Audio Capture Service:
  - Persistent microphone stream.
  - VAD endpointing (speech start/stop) and buffering.

- ASR Engine:
  - Whisper Large v3 MLX (default), optional model presets.
  - Batch/stream inference abstraction.

- Transcript Processor:
  - Optional cleanup/punctuation/filler pass.
  - Dictionary/snippet/style transforms.

- Delivery Engine:
  - Codex Session Adapter (direct `codex exec resume` path).
  - App Insertion Adapter (clipboard/accessibility fallback path).
  - Retry + idempotency guard.

- Session Manager:
  - Read session catalog.
  - Persist selected receiver.
  - Validate session existence before delivery.

- Unified TUI (Control Plane):
  - Session pane, live transcript pane, server/log pane.
  - Explicit mode indicator and health indicator.

### 9.2 Delivery Strategy (Deterministic)
- Resolve target session from selected receiver file only.
- Never auto-fallback to random/last session unless user opts in.
- Validate that session ID format is correct before forwarding.
- Log every delivery with `{utterance_id, target_session_id, result}`.

## 10. UX Requirements
- One hotkey to toggle listening in app-insertion mode (future desktop global hook).
- In TUI mode:
  - `Enter`: set receiver session.
  - `a`: always-on toggle.
  - `t`: single utterance.
  - `s`: server start/stop.
  - `h`: health.
- Error messages must include actionable fix, not only exception text.
- Show latest transcript and latest delivery status separately.

## 11. Security & Data Handling
- Store local artifacts under workspace-local config/log directories.
- Redact sensitive transcript segments in debug logs when privacy mode is enabled.
- Keep rolling logs with retention policy (e.g., 7 days default).
- Provide one command to purge local transcripts/log history.

## 12. Milestones

M1 (P0 Stability, 1-2 weeks):
- Add VAD-based segmentation.
- Hard-lock deterministic session routing.
- Standardize delivery mode defaults and remove ambiguous fallbacks.
- Add structured logs and health states in UI.

M2 (P1 Product Quality, 1-2 weeks):
- Local transcript cleanup pipeline.
- Dictionary/snippets.
- Better live transcript panel (partial + final result events).

M3 (P1 Developer Experience, 1-2 weeks):
- IDE-aware vocabulary/context adapter.
- Command mode (local rewrite model/plugin).
- Robust insertion adapter with fallback hierarchy.

M4 (P2 Polish, ongoing):
- Profile presets, advanced shortcuts, packaging, onboarding.

## 13. Acceptance Criteria
- AC-1: With selected receiver session, 50/50 utterances land in that exact session, 0 in any other session.
- AC-2: Always-on mode can run for 30 minutes with no process crash.
- AC-3: p95 end-of-speech to transcript <= 1200 ms in test environment.
- AC-4: If server is down, UI shows clear unhealthy state and recover path.
- AC-5: If session not selected, no forwarding occurs and user sees a blocking warning.
- AC-6: User can stop listening and stop server from UI without terminal kill commands.

## 14. Risks and Mitigations
- Risk: VAD tuning causes missed speech or over-segmentation.
  - Mitigation: configurable aggressiveness + debug waveform/VAD view.

- Risk: Codex CLI behavior changes and breaks adapter.
  - Mitigation: contract tests around `codex exec resume` invocation and output parsing.

- Risk: App insertion varies by terminal/editor.
  - Mitigation: explicit mode separation and fallback pathways.

- Risk: Latency spikes on large models.
  - Mitigation: model tiering (large-v3 / turbo-like options) + chunk scheduling.

## 15. Open Questions
- Should command mode be strict-local only in v1, or allow optional cloud plugin path?
- Do we want partial streaming transcripts (token-by-token) or only utterance-final for v1?
- Should global hotkeys be in-scope for v1, or only in-app controls?

## 16. Success Metrics
- Daily active dictation sessions.
- Mean words per minute achieved.
- Forwarding success rate to selected session.
- p95 transcription and delivery latency.
- User-reported correction rate (manual edits per 100 words).

## 17. Implementation Hand-off Notes
- Preserve current FastAPI API shape where possible for compatibility.
- Add versioned config file for future migration safety.
- Introduce a dedicated `delivery_mode` setting (`codex_session`, `app_insert`).
- Add integration tests covering:
  - session selection,
  - routing correctness,
  - server lifecycle,
  - always-on stop/resume behavior.

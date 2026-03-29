# Test and Benchmark Plan

Date: 2026-03-29

## Test Layers

### Unit Tests
- command parsing and transforms.
- session ID parsing/validation.
- VAD endpointing logic.

### Integration Tests
- end-to-end utterance -> transcript -> delivery.
- selected-session-only routing correctness.
- server lifecycle start/stop and health transitions.

### Soak Tests
- always-on mode for 30-60 minutes.
- repeated start/stop cycles for capture and server.

## Routing Correctness Test
- prepare N known session IDs.
- set explicit receiver ID.
- send 100 synthetic utterances.
- assert 100% deliveries target selected ID; 0 misroutes.

## Latency Benchmarks
Metrics:
- speech-end -> transcript-ready (p50/p95)
- transcript-ready -> delivery-complete (p50/p95)

Target SLOs:
- p95 speech-end -> transcript <= 1200 ms
- p95 transcript -> delivery <= 1800 ms

## Failure Injection
- missing model file.
- unavailable microphone.
- Codex binary unavailable.
- invalid selected session id.

Expected outcome:
- no panic,
- clear user-facing error,
- recover without full process restart.

## Golden Audio Dataset
Build local fixtures:
- short clean speech
- noisy speech
- whispered speech
- code-heavy dictation samples

Use fixtures in regression runs to track WER proxy and correction rate.

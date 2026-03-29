# Audio Capture and VAD Implementation Notes

Date: 2026-03-29

## Capture Layer
Recommended crate:
- `cpal` for cross-platform input streams and default device/config selection.

Capture requirements:
- mono pipeline target.
- resample/convert to 16kHz f32 for ASR.
- non-blocking callback path with ring buffer or channel handoff.

## VAD Strategy

### Why VAD is required
Current fixed 5-second chunking causes:
- unnecessary latency,
- bad utterance boundaries,
- empty/partial transcripts.

### VAD candidates
1. `webrtc-vad` crate:
- lightweight and well-known behavior.
- easy voice/non-voice frame classification.

2. Silero VAD (model-based):
- higher quality in noisy conditions.
- more integration complexity.

## Recommended v1 Path
- Start with WebRTC VAD for robust baseline and fast integration.
- Build abstraction trait so Silero can be plugged later.

## Turn Endpointing Rules (Proposed)
- Frame size: 20 ms.
- Speech start threshold: 3 consecutive voiced frames.
- Speech end threshold: 20-30 consecutive unvoiced frames.
- Max utterance cap: 15 s to prevent runaway buffers.
- Min utterance duration: 250 ms (otherwise discard as noise).

## Data Flow
`cpal input callback -> frame queue -> vad classifier -> utterance builder -> asr queue`

## Debug Requirements
- VAD debug mode should show:
  - frame energy,
  - voiced/unvoiced decisions,
  - utterance boundaries,
  - dropped frame count.

# ASR Engine Knowledge: Whisper in a Rust-First Product

Date: 2026-03-29

## Known Facts from Source Material
- OpenAI Whisper supports language identification, multilingual transcription, translation to English, and phrase-level timestamps.
- whisper-rs is Rust bindings over whisper.cpp and expects whisper.cpp model formats.
- whisper-rs full pipeline expects audio as `f32`, 16kHz, mono.

## Current Model Constraint in This Workspace
Current downloaded model:
- `/Users/jonasknppel/dev/models/whisper-large-v3/mlx-community__whisper-large-v3-mlx/weights.npz`

Implication:
- This MLX-format model is ideal for current Python/MLX path.
- For fully Rust-native whisper-rs/whisper.cpp path, a compatible model artifact is required.

## Engine Options

### Option A (Recommended for Rust-first execution)
Use `whisper-rs` + whisper.cpp-compatible model.
Pros:
- Fully Rust application runtime.
- Good Mac performance with Metal-backed whisper.cpp path.
- Mature ecosystem and straightforward C-ABI binding via whisper-rs.
Cons:
- Requires model format compatibility migration.

### Option B
Keep MLX model and call sidecar inference.
Pros:
- Reuses current downloaded model directly.
Cons:
- Violates strict "fully Rust-based" runtime goal.

## ASR Module API (Proposed)
- `load_model(config) -> AsrHandle`
- `transcribe_utterance(pcm_f32_16k_mono) -> TranscriptResult`
- optional `transcribe_with_timestamps(...)`

## Latency Optimization Notes
- Warm model and keep state/context resident.
- Avoid repeated allocations for utterance buffers.
- Keep utterance chunk sizes bounded by VAD endpointing.
- Use bounded queue between VAD and ASR to avoid memory spikes.

## Decision
- Product target: Option A for strict Rust runtime.
- Keep temporary compatibility adapter only during migration.

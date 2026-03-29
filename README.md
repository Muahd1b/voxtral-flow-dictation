# Codex ASR Switch

Rust-first, local ASR dictation for macOS.

`ASR_Switch` is the primary runtime and handles:
- push-to-talk microphone capture
- local Voxtral transcription
- focused-app text injection via macOS accessibility (`System Events`)

A FastAPI bridge still exists for compatibility (`app/main.py`), but it is not required for normal usage.

## Project Layout

- `tools/session-switcher-tui/` - Rust app + global hotkey daemon
- `config/profile.json` - runtime profile (created on first run)
- `app/main.py` - optional legacy compatibility bridge
- `scripts/` - helper scripts

## Run

```bash
ASR_Switch
```

Or from source:

```bash
cd tools/session-switcher-tui
cargo run --release
```

Global daemon mode:

```bash
ASR_Switch daemon
```

## Path Resolution Defaults

You can override all important paths with env vars.

- Profile path:
  - `ASR_PROFILE_PATH` (exact file)
  - or `ASR_PROJECT_DIR` + `/config/profile.json`
- Voxtral:
  - `ASR_VOXTRAL_BIN`
  - `ASR_VOXTRAL_MODEL_DIR`
  - default root: `~/DEV/voxtral.c`
- Lock files:
  - `ASR_VOXTRAL_LOCK_FILE` (default `/tmp/codex-asr-voxtral.lock`)
  - `ASR_GLOBAL_PTT_LOCK_FILE` (default `/tmp/codex-asr-global-ptt.lock`)

## Main Runtime Env Vars

- `ASR_VOXTRAL_BIN`
- `ASR_VOXTRAL_MODEL_DIR`
- `ASR_VOXTRAL_TIMEOUT_SEC`
- `ASR_VOXTRAL_EMPTY_RETRIES`
- `ASR_VOXTRAL_LOCK_TIMEOUT_MS`
- `ASR_VOXTRAL_LOCK_STALE_SEC`
- `ASR_VOXTRAL_LOCK_FILE`
- `ASR_GLOBAL_PTT_LOCK_FILE`
- `ASR_FFMPEG_BIN`
- `ASR_PTT_KEY`
- `ASR_LANGUAGE`

## Keybindings (TUI)

- `Space`: start/stop recording and process transcript
- `t`: single-shot record/transcribe/inject
- `c`: rewrite selected text in focused app
- `p`: cycle rewrite mode
- `i`: cycle inject mode
- `g`: toggle global PTT daemon
- `k`: cycle daemon hotkey
- `r`: reload profile
- `v`: validate Voxtral setup
- `Tab`: switch pane
- `q`: quit

## Python Scripts

- `scripts/download_model.py` - download Whisper model assets (optional)
- `scripts/ptt_client.py` - optional bridge WebSocket push-to-talk client
- `scripts/ws_client_example.py` - optional bridge test client

Removed legacy script:
- `scripts/asr_hub.py`

## macOS Permissions

Grant to your terminal app (or host app):
- Accessibility
- Input Monitoring
- Microphone

Without Accessibility, injection will fail.

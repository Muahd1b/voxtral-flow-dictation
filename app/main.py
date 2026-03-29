import asyncio
import base64
import json
import os
import subprocess
import tempfile
from pathlib import Path
from typing import Any, Optional

from fastapi import FastAPI, HTTPException, WebSocket, WebSocketDisconnect
from pydantic import BaseModel, Field

MODEL_PATH = Path(
    os.getenv(
        "ASR_MODEL_PATH",
        "/Users/jonasknppel/dev/models/whisper-large-v3/mlx-community__whisper-large-v3-mlx",
    )
).expanduser()
ASR_BACKEND = os.getenv("ASR_BACKEND", "auto").strip().lower()
ASR_LANGUAGE = os.getenv("ASR_LANGUAGE", "en")
FFMPEG_BIN = os.getenv("ASR_FFMPEG_BIN", "ffmpeg")
VOXTRAL_BIN = Path(
    os.getenv("ASR_VOXTRAL_BIN", "/Users/jonasknppel/DEV/voxtral.c/voxtral")
).expanduser()
VOXTRAL_MODEL_DIR = Path(
    os.getenv("ASR_VOXTRAL_MODEL_DIR", "/Users/jonasknppel/DEV/voxtral.c/voxtral-model")
).expanduser()
CODEX_BIN = os.getenv("CODEX_BIN", "codex")
CODEX_RECEIVER_SESSION_FILE = Path(
    os.getenv(
        "CODEX_RECEIVER_SESSION_FILE",
        "/Users/jonasknppel/dev/codex-asr-bridge/.codex_receiver_session",
    )
).expanduser()


app = FastAPI(title="Codex ASR Bridge", version="0.1.0")
TRANSCRIBE_LOCK = asyncio.Lock()
VOXTRAL_TIMEOUT_SEC = int(os.getenv("ASR_VOXTRAL_TIMEOUT_SEC", "120"))


class TranscribeRequest(BaseModel):
    audio_path: str = Field(..., description="Absolute path to an audio file")
    language: Optional[str] = Field(default=None, description="Language code, e.g. en")
    send_to_codex: bool = Field(default=True)
    codex_session: Optional[str] = Field(default="selected")


def _active_backend() -> str:
    if ASR_BACKEND == "auto":
        return "voxtral" if _voxtral_artifacts_ready() else "whisper"
    if ASR_BACKEND in {"whisper", "mlx", "mlx_whisper"}:
        return "whisper"
    if ASR_BACKEND in {"voxtral", "voxtral.c", "voxtral_cpp", "voxtral-cpp"}:
        return "voxtral"
    return ASR_BACKEND


def _assert_whisper_ready() -> None:
    required = MODEL_PATH / "weights.npz"
    if not required.exists():
        raise RuntimeError(
            f"Model not ready at {MODEL_PATH}. Missing {required.name}. "
            "Run scripts/download_model.py first."
        )


def _assert_voxtral_ready() -> None:
    if not _voxtral_artifacts_ready():
        raise RuntimeError(
            "Voxtral backend is not ready. "
            f"Expected binary: {VOXTRAL_BIN}, model dir: {VOXTRAL_MODEL_DIR}. "
            "Build voxtral.c first (make mps) or set ASR_VOXTRAL_BIN / ASR_VOXTRAL_MODEL_DIR."
        )


def _voxtral_artifacts_ready() -> bool:
    if not VOXTRAL_BIN.exists():
        return False
    required = [
        VOXTRAL_MODEL_DIR / "consolidated.safetensors",
        VOXTRAL_MODEL_DIR / "tekken.json",
        VOXTRAL_MODEL_DIR / "params.json",
    ]
    return all(p.exists() for p in required)


def _get_backend_status() -> dict[str, Any]:
    backend = _active_backend()
    if backend == "whisper":
        return {
            "backend": backend,
            "resource_path": str(MODEL_PATH),
            "ready": (MODEL_PATH / "weights.npz").exists(),
        }
    if backend == "voxtral":
        required = [
            VOXTRAL_MODEL_DIR / "consolidated.safetensors",
            VOXTRAL_MODEL_DIR / "tekken.json",
            VOXTRAL_MODEL_DIR / "params.json",
        ]
        return {
            "backend": backend,
            "resource_path": str(VOXTRAL_MODEL_DIR),
            "binary_path": str(VOXTRAL_BIN),
            "ready": VOXTRAL_BIN.exists() and all(p.exists() for p in required),
        }
    return {"backend": backend, "ready": False}


def _transcribe_file_whisper(path: Path, language: Optional[str]) -> dict[str, Any]:
    _assert_whisper_ready()
    import mlx_whisper

    result = mlx_whisper.transcribe(
        str(path),
        path_or_hf_repo=str(MODEL_PATH),
        language=language or ASR_LANGUAGE,
        word_timestamps=False,
    )
    return result


def _convert_audio_for_voxtral(path: Path) -> tuple[Path, Optional[Path]]:
    if path.suffix.lower() == ".wav":
        return path, None

    with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as tmp:
        wav_path = Path(tmp.name)

    cmd = [
        FFMPEG_BIN,
        "-y",
        "-i",
        str(path),
        "-ac",
        "1",
        "-ar",
        "16000",
        "-sample_fmt",
        "s16",
        str(wav_path),
    ]
    proc = subprocess.run(cmd, capture_output=True, text=True, check=False)
    if proc.returncode != 0:
        wav_path.unlink(missing_ok=True)
        raise RuntimeError(
            "Failed converting input audio to WAV for Voxtral. "
            f"ffmpeg returned {proc.returncode}: {(proc.stderr or proc.stdout).strip()[:240]}"
        )
    return wav_path, wav_path


def _transcribe_file_voxtral(path: Path, _language: Optional[str]) -> dict[str, Any]:
    _assert_voxtral_ready()
    wav_path, cleanup_path = _convert_audio_for_voxtral(path)
    try:
        cmd = [
            str(VOXTRAL_BIN),
            "-d",
            str(VOXTRAL_MODEL_DIR),
            "-i",
            str(wav_path),
            "--silent",
        ]
        try:
            proc = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                check=False,
                timeout=VOXTRAL_TIMEOUT_SEC,
            )
        except subprocess.TimeoutExpired as exc:
            raise RuntimeError(
                f"voxtral timed out after {VOXTRAL_TIMEOUT_SEC}s "
                f"for input {wav_path.name}"
            ) from exc
        if proc.returncode != 0:
            raise RuntimeError(
                f"voxtral failed with exit code {proc.returncode}: "
                f"{(proc.stderr or proc.stdout).strip()[:240]}"
            )
        text = (proc.stdout or "").strip()
        if not text:
            raise RuntimeError("voxtral returned empty transcript")
        return {"text": text, "segments": []}
    finally:
        if cleanup_path is not None:
            cleanup_path.unlink(missing_ok=True)


def _transcribe_file(path: Path, language: Optional[str]) -> dict[str, Any]:
    backend = _active_backend()
    if backend == "whisper":
        return _transcribe_file_whisper(path, language)
    if backend == "voxtral":
        return _transcribe_file_voxtral(path, language)
    raise RuntimeError(
        f"Unsupported ASR_BACKEND={ASR_BACKEND!r}. "
        "Use one of: whisper, voxtral."
    )


async def _transcribe_serialized(path: Path, language: Optional[str]) -> dict[str, Any]:
    async with TRANSCRIBE_LOCK:
        return await asyncio.to_thread(_transcribe_file, path, language)


def _codex_resume(prompt: str, session: Optional[str] = "last") -> dict[str, Any]:
    if not prompt.strip():
        return {"ok": False, "error": "Empty transcript, refusing to forward to codex."}

    cmd = [CODEX_BIN, "exec", "resume", "--skip-git-repo-check"]

    resolved_session = _resolve_codex_session(session)
    if resolved_session != "last":
        cmd.append(resolved_session)
    else:
        cmd.append("--last")
    cmd.append("-")

    proc = subprocess.run(
        cmd,
        input=prompt,
        text=True,
        capture_output=True,
        check=False,
    )
    return {
        "ok": proc.returncode == 0,
        "returncode": proc.returncode,
        "stdout": proc.stdout,
        "stderr": proc.stderr,
        "cmd": cmd,
        "resolved_session": resolved_session,
    }


def _resolve_codex_session(session: Optional[str]) -> str:
    raw = (session or "").strip()
    if raw and raw not in {"last", "selected"}:
        return raw
    if raw == "last":
        return "last"

    # default mode: selected receiver from file, fallback to --last
    if CODEX_RECEIVER_SESSION_FILE.exists():
        selected = CODEX_RECEIVER_SESSION_FILE.read_text(encoding="utf-8").strip()
        if selected:
            return selected
    return "last"


@app.get("/healthz")
async def healthz() -> dict[str, Any]:
    backend_status = _get_backend_status()
    return {
        "ok": True,
        "backend": backend_status.get("backend"),
        "model_path": backend_status.get("resource_path"),
        "model_ready": backend_status.get("ready"),
        "asr_binary_path": backend_status.get("binary_path"),
    }


@app.post("/transcribe")
async def transcribe_api(payload: TranscribeRequest) -> dict[str, Any]:
    audio = Path(payload.audio_path).expanduser()
    if not audio.exists():
        raise HTTPException(status_code=404, detail=f"Audio file not found: {audio}")

    result = await _transcribe_serialized(audio, payload.language)
    transcript = (result.get("text") or "").strip()

    out: dict[str, Any] = {
        "ok": True,
        "text": transcript,
        "segments": result.get("segments", []),
    }
    if payload.send_to_codex:
        out["codex"] = await asyncio.to_thread(
            _codex_resume, transcript, payload.codex_session
        )
    return out


@app.websocket("/ws")
async def ws_transcribe(websocket: WebSocket) -> None:
    await websocket.accept()
    stream_tmp_path: Optional[Path] = None
    stream_tmp_file = None
    stream_language: Optional[str] = None
    stream_send_to_codex: bool = False
    stream_session: Optional[str] = "last"

    def _clear_stream_state() -> None:
        nonlocal stream_tmp_path, stream_tmp_file
        if stream_tmp_file is not None:
            try:
                stream_tmp_file.close()
            except Exception:
                pass
        if stream_tmp_path is not None:
            stream_tmp_path.unlink(missing_ok=True)
        stream_tmp_path = None
        stream_tmp_file = None

    backend_status = _get_backend_status()
    await websocket.send_json(
        {
            "type": "ready",
            "backend": backend_status.get("backend"),
            "model_path": backend_status.get("resource_path"),
            "model_ready": backend_status.get("ready"),
            "asr_binary_path": backend_status.get("binary_path"),
            "protocol": {
                "transcribe_file": {
                    "type": "transcribe_file",
                    "audio_path": "/abs/path.wav",
                    "language": "en",
                    "send_to_codex": True,
                    "codex_session": "selected",
                },
                "transcribe_base64": {
                    "type": "transcribe_base64",
                    "audio_b64": "<base64 wav/mp3/m4a bytes>",
                    "audio_ext": "wav",
                    "language": "en",
                    "send_to_codex": True,
                    "codex_session": "selected",
                },
                "stream_start": {
                    "type": "stream_start",
                    "audio_ext": "wav",
                    "language": "en",
                    "send_to_codex": True,
                    "codex_session": "selected",
                },
                "stream_chunk": {
                    "type": "stream_chunk",
                    "audio_b64": "<base64 chunk bytes>",
                },
                "stream_end": {"type": "stream_end"},
                "stream_cancel": {"type": "stream_cancel"},
                "ping": {"type": "ping"},
            },
        }
    )

    try:
        while True:
            raw = await websocket.receive_text()
            try:
                msg = json.loads(raw)
            except json.JSONDecodeError:
                await websocket.send_json({"type": "error", "error": "Invalid JSON payload"})
                continue

            mtype = msg.get("type")
            if mtype == "ping":
                await websocket.send_json({"type": "pong"})
                continue

            if mtype == "stream_start":
                _clear_stream_state()
                ext = str(msg.get("audio_ext", "wav")).strip(".")
                stream_language = msg.get("language")
                stream_send_to_codex = bool(msg.get("send_to_codex", True))
                stream_session = msg.get("codex_session", "selected")
                tmp = tempfile.NamedTemporaryFile(suffix=f".{ext}", delete=False)
                stream_tmp_path = Path(tmp.name)
                stream_tmp_file = tmp
                await websocket.send_json(
                    {"type": "status", "status": "stream_started", "tmp_path": str(stream_tmp_path)}
                )
                continue

            if mtype == "stream_chunk":
                if stream_tmp_file is None:
                    await websocket.send_json(
                        {"type": "error", "error": "stream_chunk received before stream_start"}
                    )
                    continue
                b64 = msg.get("audio_b64")
                if not b64:
                    await websocket.send_json(
                        {"type": "error", "error": "Missing field: audio_b64"}
                    )
                    continue
                try:
                    chunk = base64.b64decode(b64)
                except Exception:
                    await websocket.send_json(
                        {"type": "error", "error": "Invalid base64 payload in stream_chunk"}
                    )
                    continue
                stream_tmp_file.write(chunk)
                stream_tmp_file.flush()
                continue

            if mtype == "stream_cancel":
                _clear_stream_state()
                await websocket.send_json({"type": "status", "status": "stream_canceled"})
                continue

            if mtype == "stream_end":
                if stream_tmp_file is None or stream_tmp_path is None:
                    await websocket.send_json(
                        {"type": "error", "error": "stream_end received before stream_start"}
                    )
                    continue
                stream_tmp_file.close()
                stream_tmp_file = None

                try:
                    await websocket.send_json({"type": "status", "status": "transcribing"})
                    result = await _transcribe_serialized(
                        stream_tmp_path, stream_language
                    )
                    text = (result.get("text") or "").strip()
                    await websocket.send_json({"type": "transcript", "text": text})
                    if stream_send_to_codex:
                        await websocket.send_json(
                            {"type": "status", "status": "forwarding_to_codex"}
                        )
                        codex_res = await asyncio.to_thread(
                            _codex_resume, text, stream_session
                        )
                        await websocket.send_json({"type": "codex_result", "data": codex_res})
                finally:
                    _clear_stream_state()
                continue

            if mtype == "transcribe_file":
                audio = Path(str(msg.get("audio_path", ""))).expanduser()
                if not audio.exists():
                    await websocket.send_json(
                        {"type": "error", "error": f"Audio file not found: {audio}"}
                    )
                    continue
                language = msg.get("language")
                send_to_codex = bool(msg.get("send_to_codex", True))
                session = msg.get("codex_session", "selected")

                await websocket.send_json({"type": "status", "status": "transcribing"})
                result = await _transcribe_serialized(audio, language)
                text = (result.get("text") or "").strip()
                await websocket.send_json({"type": "transcript", "text": text})

                if send_to_codex:
                    await websocket.send_json({"type": "status", "status": "forwarding_to_codex"})
                    codex_res = await asyncio.to_thread(_codex_resume, text, session)
                    await websocket.send_json({"type": "codex_result", "data": codex_res})
                continue

            if mtype == "transcribe_base64":
                b64 = msg.get("audio_b64")
                if not b64:
                    await websocket.send_json(
                        {"type": "error", "error": "Missing field: audio_b64"}
                    )
                    continue
                language = msg.get("language")
                send_to_codex = bool(msg.get("send_to_codex", True))
                session = msg.get("codex_session", "selected")
                ext = str(msg.get("audio_ext", "wav")).strip(".")

                try:
                    decoded = base64.b64decode(b64)
                except Exception:
                    await websocket.send_json(
                        {"type": "error", "error": "Invalid base64 payload in transcribe_base64"}
                    )
                    continue

                with tempfile.NamedTemporaryFile(suffix=f".{ext}", delete=False) as tmp:
                    tmp_path = Path(tmp.name)
                    tmp.write(decoded)

                try:
                    await websocket.send_json({"type": "status", "status": "transcribing"})
                    result = await _transcribe_serialized(tmp_path, language)
                    text = (result.get("text") or "").strip()
                    await websocket.send_json({"type": "transcript", "text": text})
                    if send_to_codex:
                        await websocket.send_json(
                            {"type": "status", "status": "forwarding_to_codex"}
                        )
                        codex_res = await asyncio.to_thread(_codex_resume, text, session)
                        await websocket.send_json({"type": "codex_result", "data": codex_res})
                finally:
                    tmp_path.unlink(missing_ok=True)
                continue

            await websocket.send_json({"type": "error", "error": f"Unsupported type: {mtype}"})
    except WebSocketDisconnect:
        _clear_stream_state()
        return

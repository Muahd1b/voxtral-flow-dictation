import argparse
import asyncio
import base64
import json
import subprocess
from contextlib import suppress
from typing import Any

import websockets


def _build_ffmpeg_cmd(device_index: str) -> list[str]:
    return [
        "ffmpeg",
        "-loglevel",
        "error",
        "-f",
        "avfoundation",
        "-i",
        f":{device_index}",
        "-ac",
        "1",
        "-ar",
        "16000",
        "-f",
        "wav",
        "-",
    ]


async def _read_server_until_done(ws: Any, *, expect_codex: bool) -> None:
    transcript_seen = False
    while True:
        raw = await ws.recv()
        data = json.loads(raw)
        mtype = data.get("type")
        if mtype == "status":
            print(f"[server] status: {data.get('status')}")
            continue
        if mtype == "transcript":
            print(f"\n[transcript]\n{data.get('text', '').strip()}\n")
            transcript_seen = True
            if not expect_codex:
                break
            continue
        if mtype == "codex_result":
            c = data.get("data", {})
            print("[codex] returncode:", c.get("returncode"))
            stdout = (c.get("stdout") or "").strip()
            stderr = (c.get("stderr") or "").strip()
            if stdout:
                print("[codex stdout]\n", stdout)
            if stderr:
                print("[codex stderr]\n", stderr)
            break
        if mtype == "error":
            print("[server error]", data.get("error"))
            break
        if transcript_seen and not expect_codex:
            break


async def _record_and_stream(
    ws: Any,
    *,
    device_index: str,
    language: str,
    send_to_codex: bool,
    codex_session: str,
    chunk_size: int,
) -> None:
    await ws.send(
        json.dumps(
            {
                "type": "stream_start",
                "audio_ext": "wav",
                "language": language,
                "send_to_codex": send_to_codex,
                "codex_session": codex_session,
            }
        )
    )

    proc = subprocess.Popen(
        _build_ffmpeg_cmd(device_index),
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        stdin=subprocess.PIPE,
    )

    async def _pump_audio() -> None:
        assert proc.stdout is not None
        while True:
            chunk = await asyncio.to_thread(proc.stdout.read, chunk_size)
            if not chunk:
                break
            payload = {
                "type": "stream_chunk",
                "audio_b64": base64.b64encode(chunk).decode("ascii"),
            }
            await ws.send(json.dumps(payload))

    pump_task = asyncio.create_task(_pump_audio())
    try:
        await asyncio.to_thread(input, "Recording... press Enter to stop > ")
    finally:
        if proc.stdin is not None:
            with suppress(Exception):
                proc.stdin.write(b"q\n")
                proc.stdin.flush()
        with suppress(Exception):
            await asyncio.wait_for(asyncio.to_thread(proc.wait), timeout=3)
        if proc.poll() is None:
            with suppress(Exception):
                proc.terminate()
        with suppress(Exception):
            await pump_task

    await ws.send(json.dumps({"type": "stream_end"}))
    await _read_server_until_done(ws, expect_codex=send_to_codex)


async def run(args: argparse.Namespace) -> None:
    async with websockets.connect(args.url, max_size=None) as ws:
        first = json.loads(await ws.recv())
        print("[connected]", first.get("type"), "model_ready:", first.get("model_ready"))
        while True:
            cmd = await asyncio.to_thread(
                input, "Press Enter to talk, or type q then Enter to quit > "
            )
            if cmd.strip().lower() == "q":
                break
            await _record_and_stream(
                ws,
                device_index=args.device_index,
                language=args.language,
                send_to_codex=args.send_to_codex,
                codex_session=args.codex_session,
                chunk_size=args.chunk_size,
            )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Push-to-talk client: stream mic audio to codex-asr-bridge over WebSocket."
    )
    parser.add_argument(
        "--url",
        default="ws://127.0.0.1:8000/ws",
        help="Bridge WebSocket URL",
    )
    parser.add_argument(
        "--device-index",
        default="0",
        help="macOS avfoundation audio device index (default: 0)",
    )
    parser.add_argument("--language", default="en", help="ASR language code")
    parser.add_argument(
        "--send-to-codex",
        action=argparse.BooleanOptionalAction,
        default=False,
        help="Forward transcript to codex exec resume (default: disabled). Use --send-to-codex to enable.",
    )
    parser.add_argument(
        "--codex-session",
        default="selected",
        help="Codex session id, 'selected' (from receiver file), or 'last'",
    )
    parser.add_argument(
        "--chunk-size",
        type=int,
        default=8192,
        help="Bytes per streamed chunk",
    )
    return parser.parse_args()


if __name__ == "__main__":
    asyncio.run(run(parse_args()))

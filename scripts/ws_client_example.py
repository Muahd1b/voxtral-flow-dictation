import asyncio
import json
from pathlib import Path

import websockets


async def main() -> None:
    audio_file = Path("/tmp/sample.wav")
    if not audio_file.exists():
        raise SystemExit(f"Put a test audio file here first: {audio_file}")

    uri = "ws://127.0.0.1:8000/ws"
    async with websockets.connect(uri, max_size=None) as ws:
        print("CONNECTED")
        print("SERVER:", await ws.recv())

        payload = {
            "type": "transcribe_file",
            "audio_path": str(audio_file),
            "language": "en",
            "send_to_codex": False,
            "codex_session": "last",
        }
        await ws.send(json.dumps(payload))
        while True:
            msg = await ws.recv()
            print("SERVER:", msg)
            data = json.loads(msg)
            if data.get("type") in {"transcript", "error"}:
                break


if __name__ == "__main__":
    asyncio.run(main())

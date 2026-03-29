import argparse
import os
from pathlib import Path

from huggingface_hub import snapshot_download


DEFAULT_REPO_ID = "mlx-community/whisper-large-v3-mlx"
DEFAULT_TARGET_DIR = Path(
    os.getenv(
        "ASR_MODEL_PATH",
        str(Path.home() / "dev/models/whisper-large-v3/mlx-community__whisper-large-v3-mlx"),
    )
).expanduser()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Download Whisper model weights.")
    parser.add_argument("--repo-id", default=DEFAULT_REPO_ID, help="HF model repo id")
    parser.add_argument(
        "--target-dir",
        default=str(DEFAULT_TARGET_DIR),
        help="Local output directory",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    target_dir = Path(args.target_dir).expanduser()

    target_dir.mkdir(parents=True, exist_ok=True)
    print(f"Downloading {args.repo_id} into {target_dir}")
    snapshot_download(repo_id=args.repo_id, local_dir=str(target_dir))
    required = target_dir / "weights.npz"
    if not required.exists():
        raise SystemExit(
            f"Download did not produce {required}. Re-run this script and check network/auth."
        )
    print("Done. Model is ready.")


if __name__ == "__main__":
    main()

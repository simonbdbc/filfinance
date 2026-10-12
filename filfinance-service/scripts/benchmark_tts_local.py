#!/usr/bin/env python3
"""Benchmark local du provider TTS Piper pour filfinance-service."""

from __future__ import annotations

import argparse
import base64
import json
import math
import platform
import subprocess
import time
from pathlib import Path


def parse_args() -> argparse.Namespace:
    root_dir = Path(__file__).resolve().parent.parent
    fixtures_path = root_dir / "scripts" / "fixtures" / "benchmark_tts_narrations_fr.txt"
    helper_path = root_dir / "scripts" / "piper_local_session.py"
    data_dir = root_dir / "data" / "models" / "piper"
    python_path = root_dir / ".venv-piper" / "bin" / "python"

    parser = argparse.ArgumentParser(
        description="Mesure la latence et le volume audio du provider Piper local."
    )
    parser.add_argument(
        "--python",
        default=str(python_path if python_path.exists() else Path("python3")),
        help="Interpreteur Python a utiliser pour le helper Piper.",
    )
    parser.add_argument(
        "--helper",
        default=str(helper_path),
        help="Chemin du helper Python Piper.",
    )
    parser.add_argument(
        "--data-dir",
        default=str(data_dir),
        help="Repertoire racine des voix Piper installees localement.",
    )
    parser.add_argument(
        "--voice",
        default="fr_FR-upmc-medium-jessica",
        help="Voix Piper a mesurer.",
    )
    parser.add_argument(
        "--rate-words-per-minute",
        type=int,
        default=185,
        help="Vitesse de diction approximatee envoyee au helper.",
    )
    parser.add_argument(
        "--fixtures",
        default=str(fixtures_path),
        help="Fichier de narrations FR de reference.",
    )
    parser.add_argument(
        "--warmup-text",
        default="Test audio. Les clips HLS audio sont operationnels.",
        help="Texte de warmup hors benchmark.",
    )
    parser.add_argument("--json", action="store_true", help="Retourne uniquement le JSON.")
    return parser.parse_args()


def load_fixtures(path: Path) -> list[str]:
    fixtures: list[str] = []
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        fixtures.append(line)

    if not fixtures:
        raise SystemExit(f"No fixtures found in {path}")

    return fixtures


def percentile(values: list[float], percent: int) -> float:
    if not values:
        return 0.0
    sorted_values = sorted(values)
    index = max(0, min(len(sorted_values) - 1, math.ceil(len(sorted_values) * percent / 100) - 1))
    return sorted_values[index]


class PiperBenchmarkSession:
    """Petit client JSONL du helper Piper."""

    def __init__(self, python_bin: str, helper_path: Path, data_dir: Path) -> None:
        self._process = subprocess.Popen(
            [python_bin, str(helper_path), "--session"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            bufsize=1,
        )
        self._data_dir = data_dir

    def synthesize(self, voice: str, rate_words_per_minute: int, text: str) -> tuple[float, int]:
        payload = {
            "command": "synthesize",
            "data_dir": str(self._data_dir),
            "voice": voice,
            "text": text,
            "rate_words_per_minute": rate_words_per_minute,
        }
        started_at = time.perf_counter()
        assert self._process.stdin is not None
        assert self._process.stdout is not None
        self._process.stdin.write(json.dumps(payload, ensure_ascii=False) + "\n")
        self._process.stdin.flush()
        line = self._process.stdout.readline()
        duration_ms = (time.perf_counter() - started_at) * 1000

        if not line:
            stderr = ""
            if self._process.stderr is not None:
                stderr = self._process.stderr.read().strip()
            raise RuntimeError(f"piper helper closed stdout unexpectedly: {stderr or '<empty>'}")

        response = json.loads(line)
        if response.get("error"):
            raise RuntimeError(response["error"])

        audio_bytes = base64.b64decode(response["audio_base64"])
        return duration_ms, len(audio_bytes)

    def close(self) -> None:
        if self._process.stdin is not None:
            self._process.stdin.close()
        self._process.terminate()
        self._process.wait(timeout=5)


def main() -> int:
    args = parse_args()
    helper_path = Path(args.helper)
    fixtures_path = Path(args.fixtures)
    data_dir = Path(args.data_dir)

    if not helper_path.exists():
        raise SystemExit(f"Missing helper: {helper_path}")
    if not data_dir.exists():
        raise SystemExit(f"Missing Piper data dir: {data_dir}")

    fixtures = load_fixtures(fixtures_path)
    session = PiperBenchmarkSession(args.python, helper_path, data_dir)

    try:
        warmup_duration_ms, warmup_bytes = session.synthesize(
            args.voice,
            args.rate_words_per_minute,
            args.warmup_text,
        )

        latencies_ms: list[float] = []
        audio_sizes_bytes: list[int] = []

        started_at = time.perf_counter()
        for text in fixtures:
            duration_ms, total_bytes = session.synthesize(
                args.voice,
                args.rate_words_per_minute,
                text,
            )
            latencies_ms.append(duration_ms)
            audio_sizes_bytes.append(total_bytes)
        total_duration_ms = (time.perf_counter() - started_at) * 1000
    finally:
        session.close()

    summary = {
        "provider": "piper_local_session",
        "voice": args.voice,
        "rate_words_per_minute": args.rate_words_per_minute,
        "fixtures_path": str(fixtures_path),
        "data_dir": str(data_dir),
        "narration_count": len(fixtures),
        "warmup_duration_ms": round(warmup_duration_ms, 2),
        "warmup_audio_bytes": warmup_bytes,
        "total_duration_ms": round(total_duration_ms, 2),
        "mean_latency_ms": round(sum(latencies_ms) / len(latencies_ms), 2),
        "p50_latency_ms": round(percentile(latencies_ms, 50), 2),
        "p95_latency_ms": round(percentile(latencies_ms, 95), 2),
        "max_latency_ms": round(max(latencies_ms), 2),
        "throughput_items_per_second": round(
            len(latencies_ms) / (total_duration_ms / 1000), 2
        ),
        "mean_audio_bytes": round(sum(audio_sizes_bytes) / len(audio_sizes_bytes), 2),
        "max_audio_bytes": max(audio_sizes_bytes),
        "environment": {
            "platform": platform.platform(),
            "python_version": platform.python_version(),
        },
    }

    if args.json:
        print(json.dumps(summary, ensure_ascii=False, indent=2))
    else:
        print("Benchmark TTS local Piper")
        print(json.dumps(summary, ensure_ascii=False, indent=2))

    return 0


if __name__ == "__main__":
    raise SystemExit(main())

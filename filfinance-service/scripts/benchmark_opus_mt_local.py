#!/usr/bin/env python3
"""Benchmark local du helper OPUS-MT tc-big en mode session."""

import argparse
import json
import math
import platform
import subprocess
import sys
import time
from pathlib import Path

PROFILE_BEAM_SIZES = {
    "fast": 1,
    "balanced": 2,
    "quality": 4,
}


def parse_args() -> argparse.Namespace:
    root_dir = Path(__file__).resolve().parent.parent
    model_dir = root_dir / "data" / "models" / "opus-mt-tc-big-en-fr-ct2-int8"
    fixtures_path = (
        root_dir / "scripts" / "fixtures" / "benchmark_translation_headlines_en.txt"
    )

    parser = argparse.ArgumentParser(
        description="Mesure la latence et le throughput du helper OPUS-MT local."
    )
    parser.add_argument(
        "--python-bin",
        default=str(root_dir / ".venv-opus-mt" / "bin" / "python"),
        help="Interpreteur Python a utiliser pour lancer le helper.",
    )
    parser.add_argument(
        "--helper",
        default=str(root_dir / "scripts" / "opus_mt_local_command.py"),
        help="Chemin du helper OPUS-MT local.",
    )
    parser.add_argument(
        "--model-dir",
        default=str(model_dir),
        help="Chemin du checkpoint CTranslate2 tc-big.",
    )
    parser.add_argument(
        "--fixtures",
        default=str(fixtures_path),
        help="Fichier de fixtures TSV: title\\tbody optionnel.",
    )
    parser.add_argument("--device", default="cpu", help="Device CTranslate2.")
    parser.add_argument(
        "--compute-type", default="int8", help="Compute type CTranslate2."
    )
    parser.add_argument(
        "--profile",
        choices=sorted(PROFILE_BEAM_SIZES),
        default="quality",
        help="Profil OPUS-MT a mesurer quand --all-profiles est absent.",
    )
    parser.add_argument(
        "--all-profiles",
        action="store_true",
        help="Mesure successivement fast, balanced et quality.",
    )
    parser.add_argument(
        "--beam-size",
        type=int,
        default=None,
        help="Override ponctuel du beam-size pour le profil unique.",
    )
    parser.add_argument(
        "--warmup-text",
        default="Federal Reserve signals rates stay higher for longer",
        help="Texte de warmup hors benchmark.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Retourne uniquement le resume JSON.",
    )
    return parser.parse_args()


def load_fixtures(path: Path) -> list[dict[str, str | None]]:
    fixtures: list[dict[str, str | None]] = []

    for raw_line in path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split("\t", 1)
        title = parts[0].strip()
        body = parts[1].strip() if len(parts) > 1 and parts[1].strip() else None
        fixtures.append({"title": title, "body": body})

    if not fixtures:
        raise SystemExit(f"No fixtures found in {path}")

    return fixtures


def percentile(values: list[float], percent: int) -> float:
    if not values:
        return 0.0
    sorted_values = sorted(values)
    index = max(0, min(len(sorted_values) - 1, math.ceil(len(sorted_values) * percent / 100) - 1))
    return sorted_values[index]


def exchange_payload(
    process: subprocess.Popen[str], payload: dict[str, str | None]
) -> tuple[dict[str, object], float]:
    started_at = time.perf_counter()
    process.stdin.write(json.dumps(payload, ensure_ascii=False) + "\n")
    process.stdin.flush()
    line = process.stdout.readline()
    duration_ms = (time.perf_counter() - started_at) * 1000

    if not line:
        stderr = process.stderr.read().strip() if process.stderr else ""
        raise RuntimeError(
            f"Helper session closed unexpectedly. stderr={stderr or '<empty>'}"
        )

    response = json.loads(line)
    return response, duration_ms


def start_helper_session(
    args: argparse.Namespace, python_bin: Path, helper: Path, model_dir: Path, beam_size: int
) -> subprocess.Popen[str]:
    command = [
        str(python_bin),
        str(helper),
        "--session",
        "--ct2-model-dir",
        str(model_dir),
        "--sentencepiece-model",
        str(model_dir / "source.tokenizer.model"),
        "--target-sentencepiece-model",
        str(model_dir / "target.tokenizer.model"),
        "--device",
        args.device,
        "--compute-type",
        args.compute_type,
        "--beam-size",
        str(beam_size),
        "--translation-mode",
        "opus_mt_tc_big_local_session",
    ]

    return subprocess.Popen(
        command,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        bufsize=1,
    )


def run_benchmark(
    args: argparse.Namespace,
    python_bin: Path,
    helper: Path,
    model_dir: Path,
    fixtures_path: Path,
    fixtures: list[dict[str, str | None]],
    profile: str,
    beam_size: int,
) -> dict[str, object]:
    process = start_helper_session(args, python_bin, helper, model_dir, beam_size)

    if process.stdin is None or process.stdout is None:
        raise SystemExit("Failed to start helper session with piped stdin/stdout")

    try:
        _, warmup_duration_ms = exchange_payload(
            process,
            {
                "source_language": "en",
                "target_language": "fr",
                "title": args.warmup_text,
                "body": None,
            },
        )

        latencies_ms: list[float] = []
        examples: list[dict[str, object]] = []
        started_at = time.perf_counter()
        for fixture in fixtures:
            response, duration_ms = exchange_payload(
                process,
                {
                    "source_language": "en",
                    "target_language": "fr",
                    "title": fixture["title"],
                    "body": fixture["body"],
                },
            )
            if not response.get("translated_title"):
                raise RuntimeError(f"Invalid translation response: {response}")
            if len(examples) < 3:
                examples.append(
                    {
                        "source_title": fixture["title"],
                        "translated_title": response.get("translated_title"),
                        "latency_ms": round(duration_ms, 2),
                    }
                )
            latencies_ms.append(duration_ms)

        total_duration_ms = (time.perf_counter() - started_at) * 1000
    finally:
        if process.stdin:
            process.stdin.close()
        process.terminate()
        try:
            process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=3)

    return {
        "provider": "opus_mt_tc_big_local_session",
        "profile": profile,
        "device": args.device,
        "compute_type": args.compute_type,
        "beam_size": beam_size,
        "model_dir": str(model_dir),
        "fixtures_path": str(fixtures_path),
        "headline_count": len(fixtures),
        "warmup_duration_ms": round(warmup_duration_ms, 2),
        "total_duration_ms": round(total_duration_ms, 2),
        "mean_latency_ms": round(sum(latencies_ms) / len(latencies_ms), 2),
        "p50_latency_ms": round(percentile(latencies_ms, 50), 2),
        "p95_latency_ms": round(percentile(latencies_ms, 95), 2),
        "max_latency_ms": round(max(latencies_ms), 2),
        "throughput_headlines_per_second": round(
            len(latencies_ms) / (total_duration_ms / 1000), 2
        ),
        "examples": examples,
        "environment": {
            "platform": platform.platform(),
            "python_version": platform.python_version(),
        },
    }


def main() -> int:
    args = parse_args()
    python_bin = Path(args.python_bin)
    helper = Path(args.helper)
    model_dir = Path(args.model_dir)
    fixtures_path = Path(args.fixtures)

    for required_path in [
        python_bin,
        helper,
        model_dir,
        model_dir / "source.tokenizer.model",
        model_dir / "target.tokenizer.model",
        fixtures_path,
    ]:
        if not required_path.exists():
            raise SystemExit(f"Missing required path: {required_path}")

    fixtures = load_fixtures(fixtures_path)
    if args.all_profiles:
        profile_plan = [(profile, PROFILE_BEAM_SIZES[profile]) for profile in PROFILE_BEAM_SIZES]
    else:
        beam_size = args.beam_size or PROFILE_BEAM_SIZES[args.profile]
        profile_plan = [(args.profile, beam_size)]

    summaries = [
        run_benchmark(
            args,
            python_bin,
            helper,
            model_dir,
            fixtures_path,
            fixtures,
            profile,
            beam_size,
        )
        for profile, beam_size in profile_plan
    ]

    output: dict[str, object]
    if args.all_profiles:
        output = {
            "provider": "opus_mt_tc_big_local_session",
            "profiles": summaries,
        }
    else:
        output = summaries[0]

    if args.json:
        print(json.dumps(output, ensure_ascii=False, indent=2))
    else:
        print("Benchmark OPUS-MT tc-big local")
        print(json.dumps(output, ensure_ascii=False, indent=2))

    return 0


if __name__ == "__main__":
    raise SystemExit(main())

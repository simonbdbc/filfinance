#!/usr/bin/env python3
"""Helper local Piper pour `filfinance-service`.

Ce helper implemente un protocole JSONL simple sur stdin/stdout pour garder
`Piper` charge en memoire entre plusieurs syntheses. Il expose aussi un mode
hors session utile pour lister et valider les voix installees localement.
"""

from __future__ import annotations

import argparse
import base64
import io
import json
import sys
import wave
from dataclasses import dataclass
from pathlib import Path
from typing import Any


@dataclass
class VoiceAssets:
    """Assets minimums attendus pour une voix Piper installee localement."""

    voice_id: str
    selection_id: str
    model_path: Path
    config_path: Path
    speaker_name: str | None = None
    speaker_id: int | None = None


@dataclass
class SynthesisPayload:
    """Payload JSON normalise envoye par le backend Rust."""

    data_dir: Path
    voice: str
    text: str
    rate_words_per_minute: int


def parse_args() -> argparse.Namespace:
    """Construit l'interface CLI du helper."""
    parser = argparse.ArgumentParser(
        description="Helper local Piper pour le worker TTS de filfinance-service."
    )
    parser.add_argument(
        "--data-dir",
        help="Repertoire racine des voix Piper installees localement.",
    )
    parser.add_argument(
        "--session",
        action="store_true",
        help="Active le protocole JSONL long-vivant sur stdin/stdout.",
    )
    parser.add_argument(
        "--list-voices",
        action="store_true",
        help="Liste les voix francaises valides detectees dans le data dir.",
    )
    parser.add_argument(
        "--validate-voice",
        help="Valide qu'une voix precise existe et expose les assets minimums.",
    )
    return parser.parse_args()


def fail(message: str, *, exit_code: int = 1) -> None:
    """Affiche une erreur exploitable puis quitte."""
    print(message, file=sys.stderr)
    raise SystemExit(exit_code)


def normalize_text(value: Any) -> str:
    """Normalise une chaine libre vers une seule ligne compacte."""
    if value is None:
        return ""
    return " ".join(str(value).split())


def parse_payload(data: dict[str, Any]) -> SynthesisPayload:
    """Valide un payload JSON de synthese."""
    voice = normalize_text(data.get("voice"))
    if not voice:
        raise ValueError("payload.voice is required and cannot be empty")

    text = normalize_text(data.get("text"))
    if not text:
        raise ValueError("payload.text is required and cannot be empty")

    data_dir_raw = normalize_text(data.get("data_dir"))
    if not data_dir_raw:
        raise ValueError("payload.data_dir is required and cannot be empty")

    rate = int(data.get("rate_words_per_minute") or 185)
    if rate <= 0:
        raise ValueError("payload.rate_words_per_minute must be greater than 0")

    return SynthesisPayload(
        data_dir=Path(data_dir_raw).expanduser().resolve(),
        voice=voice,
        text=text,
        rate_words_per_minute=rate,
    )


def discover_french_voices(data_dir: Path) -> list[VoiceAssets]:
    """Scanne le repertoire d'assets et retourne les voix FR valides."""
    if not data_dir.exists():
        raise FileNotFoundError(f"piper data dir does not exist: {data_dir}")

    voices: list[VoiceAssets] = []
    for child in sorted(data_dir.iterdir()):
        if not child.is_dir():
            continue

        voice_id = child.name
        if not voice_id.startswith("fr_"):
            continue

        model_path = child / "model.onnx"
        config_path = child / "model.onnx.json"
        if not model_path.exists() or not config_path.exists():
            continue

        try:
            config_data = json.loads(config_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            continue

        speaker_id_map = config_data.get("speaker_id_map")
        if isinstance(speaker_id_map, dict) and speaker_id_map:
            ordered_speakers = sorted(
                (
                    (normalize_text(speaker_name), int(speaker_id))
                    for speaker_name, speaker_id in speaker_id_map.items()
                    if normalize_text(speaker_name)
                ),
                key=lambda item: (item[1], item[0]),
            )

            for speaker_name, speaker_id in ordered_speakers:
                voices.append(
                    VoiceAssets(
                        voice_id=voice_id,
                        selection_id=f"{voice_id}-{speaker_name}",
                        model_path=model_path,
                        config_path=config_path,
                        speaker_name=speaker_name,
                        speaker_id=speaker_id,
                    )
                )
            continue

        voices.append(
            VoiceAssets(
                voice_id=voice_id,
                selection_id=voice_id,
                model_path=model_path,
                config_path=config_path,
            )
        )

    return voices


def load_runtime():
    """Charge Piper paresseusement pour garder un message d'erreur clair."""
    try:
        from piper import PiperVoice  # type: ignore
        from piper.voice import SynthesisConfig  # type: ignore
    except ImportError as error:  # pragma: no cover - dependance externe
        fail(f"missing dependency piper-tts: {error}")

    return PiperVoice, SynthesisConfig


def rate_words_per_minute_to_length_scale(rate_words_per_minute: int) -> float:
    """Approxime l'ancienne notion WPM en `length_scale` Piper."""
    baseline = 185.0
    length_scale = baseline / float(rate_words_per_minute)
    return min(2.0, max(0.5, length_scale))


class PiperSession:
    """Session long-vivante avec cache de voix Piper chargees."""

    def __init__(self) -> None:
        PiperVoice, SynthesisConfig = load_runtime()
        self._piper_voice_cls = PiperVoice
        self._synthesis_config_cls = SynthesisConfig
        self._voice_cache: dict[str, Any] = {}

    def _get_assets(self, payload: SynthesisPayload) -> VoiceAssets:
        discovered_voices = discover_french_voices(payload.data_dir)

        # Le chemin nominal passe par l'identifiant plat expose au client, potentiellement enrichi du speaker.
        for assets in discovered_voices:
            if assets.selection_id == payload.voice:
                return assets

        # Compatibilite descendante: une ancienne config `fr_FR-upmc-medium` est redirigee
        # vers le premier speaker detecte pour ce modele.
        for assets in discovered_voices:
            if assets.voice_id == payload.voice:
                return assets

        raise FileNotFoundError(
            f"piper voice '{payload.voice}' was not found in {payload.data_dir}"
        )

    def _load_voice(self, assets: VoiceAssets):
        cached = self._voice_cache.get(assets.voice_id)
        if cached is not None:
            return cached

        voice = self._piper_voice_cls.load(str(assets.model_path))
        self._voice_cache[assets.voice_id] = voice
        return voice

    def synthesize(self, payload: SynthesisPayload) -> dict[str, Any]:
        """Genere un WAV complet et le retourne en base64."""
        assets = self._get_assets(payload)
        voice = self._load_voice(assets)
        synthesis_config = self._synthesis_config_cls(
            speaker_id=assets.speaker_id,
            length_scale=rate_words_per_minute_to_length_scale(
                payload.rate_words_per_minute
            )
        )

        wav_buffer = io.BytesIO()
        with wave.open(wav_buffer, "wb") as wav_file:
            voice.synthesize_wav(payload.text, wav_file, syn_config=synthesis_config)

        wav_bytes = wav_buffer.getvalue()
        if not wav_bytes:
            raise RuntimeError("piper returned an empty WAV payload")

        with wave.open(io.BytesIO(wav_bytes), "rb") as wav_reader:
            sample_rate_hz = wav_reader.getframerate()

        return {
            "voice": assets.selection_id,
            "audio_format": "wav",
            "mime_type": "audio/wav",
            "audio_base64": base64.b64encode(wav_bytes).decode("ascii"),
            "sample_rate_hz": sample_rate_hz,
        }


def run_session() -> int:
    """Boucle JSONL long-vivante du helper."""
    session = PiperSession()

    try:
        for line in sys.stdin:
            raw_line = line.strip()
            if not raw_line:
                continue

            try:
                data = json.loads(raw_line)
                payload = parse_payload(data)
                response = session.synthesize(payload)
            except Exception as error:  # pragma: no cover - defensive runtime path
                response = {"error": str(error)}

            sys.stdout.write(json.dumps(response, ensure_ascii=False) + "\n")
            sys.stdout.flush()
    except KeyboardInterrupt:
        return 0

    return 0


def main() -> int:
    """Point d'entree principal du helper."""
    args = parse_args()

    if args.session:
        return run_session()

    data_dir_value = args.data_dir
    if not data_dir_value:
        fail("--data-dir is required outside session mode")

    data_dir = Path(data_dir_value).expanduser().resolve()

    if args.list_voices:
        voices = [assets.selection_id for assets in discover_french_voices(data_dir)]
        print(json.dumps({"voices": voices}, ensure_ascii=False, indent=2))
        return 0

    if args.validate_voice:
        voice_id = normalize_text(args.validate_voice)
        for assets in discover_french_voices(data_dir):
            if assets.selection_id == voice_id or assets.voice_id == voice_id:
                print(
                    json.dumps(
                        {
                            "voice": assets.selection_id,
                            "model_voice": assets.voice_id,
                            "speaker": assets.speaker_name,
                            "speaker_id": assets.speaker_id,
                            "model_path": str(assets.model_path),
                            "config_path": str(assets.config_path),
                        },
                        ensure_ascii=False,
                        indent=2,
                    )
                )
                return 0
        fail(f"voice not found: {voice_id}")

    fail("choose one of --session, --list-voices or --validate-voice")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

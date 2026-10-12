#!/usr/bin/env python3
"""Helper local de traduction OPUS-MT pour `filfinance-service`.

Ce script implemente le contrat `stdin JSON -> stdout JSON` attendu par
`NEWS_TRANSLATION_COMMAND`. Il utilise un modele OPUS-MT converti au format
`CTranslate2` et un modele `SentencePiece` pour tokenizer et detokenizer.

Le script supporte aussi un mode `--session` en JSON lines pour garder le modele
charge en memoire entre plusieurs traductions.
"""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, NoReturn


@dataclass
class TranslationPayload:
    """Payload normalise envoye par le service Rust."""

    source_language: str
    target_language: str
    title: str
    body: str | None


def parse_args() -> argparse.Namespace:
    """Construit l'interface CLI du helper local."""
    parser = argparse.ArgumentParser(
        description="Traduit un payload JSON via un modele OPUS-MT converti pour CTranslate2."
    )
    parser.add_argument(
        "--ct2-model-dir",
        required=True,
        help="Chemin du modele converti au format CTranslate2.",
    )
    parser.add_argument(
        "--sentencepiece-model",
        required=True,
        help="Chemin du fichier SentencePiece source.spm du modele OPUS-MT.",
    )
    parser.add_argument(
        "--target-sentencepiece-model",
        help="Chemin du fichier SentencePiece cible. Par defaut, tente `target.spm` a cote du modele source.",
    )
    parser.add_argument(
        "--device",
        default="cpu",
        choices=("cpu", "cuda", "auto"),
        help="Device CTranslate2 a utiliser.",
    )
    parser.add_argument(
        "--compute-type",
        default="default",
        help="Precision de calcul CTranslate2, par exemple default, int8 ou float16.",
    )
    parser.add_argument(
        "--beam-size",
        type=int,
        default=1,
        help="Beam size de traduction. 1 minimise la latence.",
    )
    parser.add_argument(
        "--max-decoding-length",
        type=int,
        default=128,
        help="Longueur maximale de decodage pour chaque texte.",
    )
    parser.add_argument(
        "--inter-threads",
        type=int,
        default=1,
        help="Nombre maximal de traductions paralleles cote CTranslate2.",
    )
    parser.add_argument(
        "--intra-threads",
        type=int,
        default=0,
        help="Nombre de threads OpenMP par traduction. 0 laisse CTranslate2 decider.",
    )
    parser.add_argument(
        "--translation-mode",
        default="opus_mt_local_command",
        help="Valeur retournee dans `translation_mode` vers le service Rust.",
    )
    parser.add_argument(
        "--session",
        action="store_true",
        help="Active le protocole JSONL long-vivant sur stdin/stdout.",
    )
    return parser.parse_args()


def fail(message: str, *, exit_code: int = 1) -> NoReturn:
    """Ecrit une erreur exploitable sur stderr puis quitte."""
    print(message, file=sys.stderr)
    raise SystemExit(exit_code)


def parse_payload(data: dict[str, Any]) -> TranslationPayload:
    """Valide un payload JSON deja parse."""
    title = normalize_text(data.get("title"))
    if not title:
        raise ValueError("payload.title is required and cannot be empty")

    return TranslationPayload(
        source_language=normalize_text(data.get("source_language")) or "en",
        target_language=normalize_text(data.get("target_language")) or "fr",
        title=title,
        body=normalize_nullable_text(data.get("body")),
    )


def load_payload() -> TranslationPayload:
    """Lit et valide le payload JSON envoye sur stdin."""
    raw = sys.stdin.read()
    if not raw.strip():
        fail("stdin payload is empty")

    try:
        data = json.loads(raw)
        return parse_payload(data)
    except json.JSONDecodeError as error:
        fail(f"invalid JSON payload: {error}")
    except ValueError as error:
        fail(str(error))


def normalize_text(value: Any) -> str:
    """Uniformise un texte pour reduire les ecarts d'entree/sortie."""
    if value is None:
        return ""
    return " ".join(str(value).split())


def normalize_nullable_text(value: Any) -> str | None:
    """Uniformise un texte optionnel."""
    normalized = normalize_text(value)
    return normalized or None


def ensure_file_exists(path: Path, label: str) -> None:
    """Valide la presence des assets critiques au demarrage."""
    if not path.exists():
        fail(f"{label} does not exist: {path}")


def load_runtime(args: argparse.Namespace):
    """Charge paresseusement les dependances et artefacts CTranslate2."""
    try:
        import ctranslate2  # type: ignore
    except ImportError as error:
        fail(f"missing dependency ctranslate2: {error}")

    try:
        import sentencepiece as spm  # type: ignore
    except ImportError as error:
        fail(f"missing dependency sentencepiece: {error}")

    model_dir = Path(args.ct2_model_dir).expanduser().resolve()
    source_sp_model = Path(args.sentencepiece_model).expanduser().resolve()
    target_sp_model = resolve_target_sentencepiece_model(
        source_sp_model=source_sp_model,
        explicit_target=args.target_sentencepiece_model,
    )

    ensure_file_exists(model_dir, "ct2 model dir")
    ensure_file_exists(source_sp_model, "source sentencepiece model")
    ensure_file_exists(target_sp_model, "target sentencepiece model")

    translator = ctranslate2.Translator(
        str(model_dir),
        device=args.device,
        compute_type=args.compute_type,
        inter_threads=args.inter_threads,
        intra_threads=args.intra_threads,
    )
    source_tokenizer = spm.SentencePieceProcessor()
    source_tokenizer.load(str(source_sp_model))
    target_tokenizer = spm.SentencePieceProcessor()
    target_tokenizer.load(str(target_sp_model))

    return translator, source_tokenizer, target_tokenizer


def resolve_target_sentencepiece_model(
    *,
    source_sp_model: Path,
    explicit_target: str | None,
) -> Path:
    """Determine le modele SentencePiece cible utilise pour le decodage."""
    if explicit_target:
        return Path(explicit_target).expanduser().resolve()

    candidate = source_sp_model.with_name("target.spm")
    if candidate.exists():
        return candidate

    return source_sp_model


def translate_text(
    text: str | None,
    *,
    translator,
    source_tokenizer,
    target_tokenizer,
    beam_size: int,
    max_decoding_length: int,
) -> str | None:
    """Traduit un texte unique via OPUS-MT et restaure une chaine propre."""
    if not text:
        return None

    source_tokens = source_tokenizer.encode(text, out_type=str)
    if not source_tokens or source_tokens[-1] != "</s>":
        source_tokens.append("</s>")

    results = translator.translate_batch(
        [source_tokens],
        beam_size=beam_size,
        max_decoding_length=max_decoding_length,
    )
    target_tokens = results[0].hypotheses[0]
    translated = target_tokenizer.decode(
        [token for token in target_tokens if token not in {"<pad>", "</s>", "<unk>"}]
    )
    return normalize_text(translated)


def translate_payload(
    payload: TranslationPayload,
    *,
    args,
    translator,
    source_tokenizer,
    target_tokenizer,
) -> dict[str, Any]:
    """Traduit un payload complet et renvoie la reponse JSON standardisee."""
    translated_title = translate_text(
        payload.title,
        translator=translator,
        source_tokenizer=source_tokenizer,
        target_tokenizer=target_tokenizer,
        beam_size=args.beam_size,
        max_decoding_length=args.max_decoding_length,
    )
    translated_body = translate_text(
        payload.body,
        translator=translator,
        source_tokenizer=source_tokenizer,
        target_tokenizer=target_tokenizer,
        beam_size=args.beam_size,
        max_decoding_length=args.max_decoding_length,
    )

    if not translated_title:
        raise ValueError("translation produced an empty title")

    return {
        "translated_title": translated_title,
        "translated_body": translated_body,
        "translation_mode": args.translation_mode,
    }


def run_session(*, args, translator, source_tokenizer, target_tokenizer) -> int:
    """Execute le protocole long-vivant JSONL sur stdin/stdout."""
    for line in sys.stdin:
        if not line.strip():
            continue

        try:
            payload = parse_payload(json.loads(line))
            response = translate_payload(
                payload,
                args=args,
                translator=translator,
                source_tokenizer=source_tokenizer,
                target_tokenizer=target_tokenizer,
            )
        except json.JSONDecodeError as error:
            response = {"error": f"invalid JSON payload: {error}"}
        except Exception as error:  # noqa: BLE001
            response = {"error": str(error)}

        sys.stdout.write(json.dumps(response, ensure_ascii=False) + "\n")
        sys.stdout.flush()

    return 0


def main() -> int:
    """Point d'entree principal du helper."""
    args = parse_args()
    translator, source_tokenizer, target_tokenizer = load_runtime(args)
    if args.session:
        return run_session(
            args=args,
            translator=translator,
            source_tokenizer=source_tokenizer,
            target_tokenizer=target_tokenizer,
        )

    payload = load_payload()

    try:
        response = translate_payload(
            payload,
            args=args,
            translator=translator,
            source_tokenizer=source_tokenizer,
            target_tokenizer=target_tokenizer,
        )
    except ValueError as error:
        fail(str(error))

    sys.stdout.write(json.dumps(response, ensure_ascii=False))
    sys.stdout.flush()
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except KeyboardInterrupt:
        # Le helper long-vivant peut recevoir le meme SIGINT que `cargo run`.
        # On quitte proprement sans traceback pour garder un arret lisible du backend.
        raise SystemExit(0)

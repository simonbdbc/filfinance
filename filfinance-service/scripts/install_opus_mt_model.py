#!/usr/bin/env python3
"""Installe le modele OPUS-MT CTranslate2 utilise par le service Docker."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import NoReturn


DEFAULT_REPO_ID = "keisuke-miyako/opus-mt-tc-big-en-fr-ct2-int8"
REQUIRED_FILES = (
    "config.json",
    "model.bin",
    "shared_vocabulary.json",
    "source.tokenizer.model",
    "target.tokenizer.model",
)


def parse_args() -> argparse.Namespace:
    """Construit l'interface CLI d'installation du modele."""
    parser = argparse.ArgumentParser(
        description="Telecharge et verifie le modele OPUS-MT tc-big en-fr CTranslate2."
    )
    parser.add_argument(
        "--repo-id",
        default=DEFAULT_REPO_ID,
        help="Depot Hugging Face contenant le modele CTranslate2 preconverti.",
    )
    parser.add_argument(
        "--revision",
        default="main",
        help="Revision Hugging Face a telecharger.",
    )
    parser.add_argument(
        "--output-dir",
        required=True,
        help="Dossier cible du modele dans l'image Docker.",
    )
    parser.add_argument(
        "--verify-only",
        action="store_true",
        help="Verifie uniquement les fichiers attendus sans telecharger.",
    )
    return parser.parse_args()


def fail(message: str) -> NoReturn:
    """Affiche une erreur de build exploitable puis quitte."""
    raise SystemExit(message)


def missing_required_files(output_dir: Path) -> list[Path]:
    """Retourne les fichiers critiques absents du dossier modele."""
    return [output_dir / filename for filename in REQUIRED_FILES if not (output_dir / filename).is_file()]


def verify_model(output_dir: Path) -> None:
    """Valide que le modele CTranslate2 minimal est present."""
    missing_files = missing_required_files(output_dir)
    if missing_files:
        formatted = ", ".join(str(path) for path in missing_files)
        fail(f"OPUS-MT model is incomplete, missing: {formatted}")


def download_model(*, repo_id: str, revision: str, output_dir: Path) -> None:
    """Telecharge les artefacts CTranslate2 depuis Hugging Face Hub."""
    try:
        from huggingface_hub import snapshot_download
    except ImportError as error:
        fail(f"missing dependency huggingface_hub: {error}")

    output_dir.mkdir(parents=True, exist_ok=True)
    snapshot_download(
        repo_id=repo_id,
        revision=revision,
        local_dir=str(output_dir),
        allow_patterns=[
            "README.md",
            "config.json",
            "model.bin",
            "shared_vocabulary.json",
            "source.tokenizer.model",
            "target.tokenizer.model",
        ],
    )


def main() -> int:
    """Point d'entree appele par le Dockerfile."""
    args = parse_args()
    output_dir = Path(args.output_dir).expanduser().resolve()

    if not args.verify_only and missing_required_files(output_dir):
        download_model(repo_id=args.repo_id, revision=args.revision, output_dir=output_dir)

    verify_model(output_dir)
    print(f"OPUS-MT CTranslate2 model ready: {output_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

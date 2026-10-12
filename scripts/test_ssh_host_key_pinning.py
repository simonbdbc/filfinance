#!/usr/bin/env python3
"""Teste le bloc SSH reel de la CI, sans reseau ni secret de production."""

from pathlib import Path
import os
import subprocess
import tempfile
import textwrap


ROOT = Path(__file__).resolve().parent.parent


def main() -> None:
    pipeline = (ROOT / ".gitlab-ci.yml").read_text(encoding="utf-8")
    start = pipeline.index("      # ssh-keyscan essaie")
    end_marker = "      chmod 644 ~/.ssh/known_hosts\n"
    end = pipeline.index(end_marker, start) + len(end_marker)
    block = textwrap.dedent(pipeline[start:end])

    with tempfile.TemporaryDirectory(prefix="filfinance-ssh-pinning-") as directory:
        root = Path(directory)
        private_key = root / "disposable-key"
        subprocess.run(
            ["ssh-keygen", "-q", "-t", "ed25519", "-N", "", "-f", str(private_key)],
            check=True,
        )
        public_key = Path(f"{private_key}.pub")
        fingerprint = subprocess.check_output(
            ["ssh-keygen", "-lf", str(public_key), "-E", "sha256"], text=True
        ).split()[1]
        key_type, key_data, *_ = public_key.read_text().split()
        candidate = f"production.invalid {key_type} {key_data}"
        fixture = root / "scan-output"
        mock = root / "ssh-keyscan"
        mock.write_text(
            '#!/bin/sh\ncat "$SCAN_FIXTURE"\nexit "$SCAN_EXIT"\n', encoding="utf-8"
        )
        mock.chmod(0o700)
        known_hosts = root / "known_hosts"
        # Ne jamais remplacer HOME : seules les destinations du bloc teste
        # sont dirigees vers le repertoire temporaire propre au scenario.
        script = block.replace("~/.ssh/known_hosts", str(known_hosts))
        cases = [
            ("commentaires-et-algorithmes-optionnels", f"# SSH-2.0-OpenSSH\n\n{candidate}\n# optional algorithm refused\n", 255, fingerprint, True),
            ("cle-simple", f"{candidate}\n", 0, fingerprint, True),
            ("empreinte-differente", f"{candidate}\n", 0, "SHA256:wrong", False),
            ("commentaires-seuls", "# SSH-2.0-OpenSSH\n\n", 255, fingerprint, False),
            ("aucune-cle", "", 255, fingerprint, False),
            ("cle-malformee", "production.invalid ssh-ed25519 invalid\n", 0, fingerprint, False),
            ("cle-malformee-apres-cle-valide", f"{candidate}\ninvalid-key-record\n", 0, fingerprint, False),
        ]
        for name, output, scan_exit, expected_pin, expected_success in cases:
            fixture.write_text(output, encoding="utf-8")
            env = dict(os.environ)
            env.update(
                PATH=f"{root}{os.pathsep}{env['PATH']}",
                SCAN_FIXTURE=str(fixture),
                SCAN_EXIT=str(scan_exit),
                SSH_PORT="22",
                HOST="production.invalid",
                SSH_HOST_KEY_SHA256=expected_pin,
            )
            # Bash et BusyBox ash (le shell du runner Alpine) ont tous deux
            # pipefail ; le cas invalide doit stopper la boucle, pas etre ignore.
            shell = env.get("FILFINANCE_SSH_PIN_TEST_SHELL", "bash")
            result = subprocess.run(
                [shell, "-eu", "-o", "pipefail", "-c", script],
                env=env, text=True, capture_output=True, timeout=10,
            )
            assert (result.returncode == 0) == expected_success, (
                f"{name}: unexpected exit {result.returncode}: {result.stderr}"
            )
            if expected_success:
                assert known_hosts.read_text() == f"{candidate}\n", name
            print(f"SSH host pinning: {name} passed")


if __name__ == "__main__":
    main()

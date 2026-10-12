#!/usr/bin/env python3
"""Valide les deux shells du deploy CI sans connexion ni commande Docker/Git."""

import json
import os
from pathlib import Path
import subprocess
import tempfile
import textwrap


ROOT = Path(__file__).resolve().parent.parent


def main() -> None:
    pipeline = (ROOT / ".gitlab-ci.yml").read_text(encoding="utf-8")
    template = pipeline.index(".deploy_backend_template: &deploy_backend_template\n")
    marker = "  script:\n    - |\n"
    start = pipeline.index(marker, template) + len(marker)
    end = pipeline.index("\ndeploy:main:backend:\n", start)
    script = textwrap.dedent(pipeline[start:end])
    shell = os.environ.get("FILFINANCE_DEPLOY_TEST_SHELL", "bash")
    syntax = subprocess.run([shell, "-n"], input=script, text=True, capture_output=True)
    assert syntax.returncode == 0, f"CI shell syntax: {syntax.stderr}"

    with tempfile.TemporaryDirectory(prefix="filfinance-deploy-shell-") as directory:
        root = Path(directory)
        capture = root / "ssh-arguments.json"
        mock = root / "ssh"
        mock.write_text(
            '#!/usr/bin/env python3\nimport json, os, pathlib, sys\n'
            'pathlib.Path(os.environ["FILFINANCE_DEPLOY_CAPTURE"]).write_text(json.dumps(sys.argv[1:]))\n',
            encoding="utf-8",
        )
        mock.chmod(0o700)
        # Toute substitution Git/Docker ou requete curl executee par erreur
        # cote runner est detectee sans toucher au depot, au reseau ou a Docker.
        for command in ("git", "docker", "curl"):
            deny = root / command
            deny.write_text('#!/bin/sh\necho "Unexpected local command" >&2\nexit 88\n')
            deny.chmod(0o700)

        env = dict(os.environ)
        env.update(
            PATH=f"{root}{os.pathsep}{env['PATH']}",
            FILFINANCE_DEPLOY_CAPTURE=str(capture),
            DEPLOY_ENV="prod", DEPLOY_BRANCH="main",
            DEPLOY_ENV_FILE=".env.server.prod",
            DEPLOY_COMPOSE_PROJECT_NAME="filfinance_prod",
            DEPLOY_BACKEND_PORT="8088", DEPLOY_BROWSER_COLLECTOR_SCALE="1",
            FILFINANCE_PROD_DEPLOY_PATH="/srv/filfinance",
            FILFINANCE_PROD_PWA_URL="https://app.filfinance.fr",
            CI_COMMIT_SHA="1234567890abcdef1234567890abcdef12345678",
            HOST="production.invalid", SSH_PORT="22", USER="ci-deploy",
            DEPLOY_PUBLIC_API_URL="", FILFINANCE_DEV_BOOTSTRAP_RELEASE="false",
        )
        result = subprocess.run(
            [shell, "-eu", "-o", "pipefail", "-c", script],
            cwd=root, env=env, capture_output=True, text=True, timeout=10,
        )
        assert result.returncode == 0, f"CI shell expansion: {result.stderr}"
        arguments = json.loads(capture.read_text())
        assert len(arguments) == 10, "remote script must remain one SSH argument"
        assert "StrictHostKeyChecking=yes" in arguments
        # Bash developpe aussi le tilde apres '=' ; ash peut le conserver.
        known_hosts_options = {
            "UserKnownHostsFile=~/.ssh/known_hosts",
            f"UserKnownHostsFile={Path.home()}/.ssh/known_hosts",
        }
        assert any(argument in known_hosts_options for argument in arguments)
        assert arguments[-3:-1] == ["-p", "22"]
        remote_script = arguments[-1]
        assert 'if [ -n "$(git status --porcelain)" ]; then' in remote_script
        assert 'test "$(git rev-parse HEAD)"' in remote_script
        assert remote_script.count('test -z "$(docker ps -aq') == 2
        assert "git checkout --detach '1234567890abcdef1234567890abcdef12345678'" in remote_script
        assert "--force-recreate" in remote_script
        assert "docker rm -f $legacy_n8n_id" in remote_script
        assert "docker rm -fv" not in remote_script and "docker volume rm" not in remote_script
        remote_syntax = subprocess.run(
            [shell, "-n"], input=remote_script, text=True, capture_output=True
        )
        assert remote_syntax.returncode == 0, f"remote shell syntax: {remote_syntax.stderr}"
        print("Backend deploy: CI parsing, expansion, SSH boundaries and remote syntax passed")


if __name__ == "__main__":
    main()

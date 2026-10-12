#!/usr/bin/env python3
"""Verifie que les mutations PROD ne peuvent pas partir automatiquement."""

from __future__ import annotations

import re
from pathlib import Path


ROOT_DIR = Path(__file__).resolve().parent.parent
PIPELINE = ROOT_DIR / ".gitlab-ci.yml"
PRODUCTION_DEPLOY_JOBS = (
    "deploy:main:backend",
    "deploy:main:bunny-landing",
    "deploy:main:bunny-pwa",
    "deploy:main:edge",
)
CALENDAR_RUN_JOB = "calendar:main:run-once"


def fail(message: str) -> None:
    raise SystemExit(f"production deploy manual guard failed: {message}")


def job_section(pipeline: str, job_name: str) -> str:
    marker = f"{job_name}:\n"
    start = pipeline.find(marker)
    if start < 0:
        fail(f"missing job {job_name}")
    next_job = re.search(
        r"^[-.A-Za-z0-9_:]+:\n",
        pipeline[start + len(marker) :],
        re.MULTILINE,
    )
    end = start + len(marker) + next_job.start() if next_job else len(pipeline)
    return pipeline[start:end]


def backend_template_section(pipeline: str) -> str:
    marker = ".deploy_backend_template: &deploy_backend_template\n"
    start = pipeline.find(marker)
    if start < 0:
        fail("missing shared backend deploy template")
    end = pipeline.find("deploy:main:backend:\n", start)
    if end < 0:
        fail("missing deploy:main:backend after the shared template")
    return pipeline[start:end]


def main() -> int:
    try:
        pipeline = PIPELINE.read_text(encoding="utf-8")
    except OSError:
        fail("pipeline configuration is unreadable")

    for job_name in PRODUCTION_DEPLOY_JOBS:
        section = job_section(pipeline, job_name)
        if "FILFINANCE_PROD_DEPLOY_ENABLED == \"true\"" not in section:
            fail(f"missing production deploy guard in {job_name}")
        if "when: manual" not in section or "allow_failure: false" not in section:
            fail(f"job {job_name} must be manual and blocking")
        if job_name.startswith("deploy:main:bunny-") and (
            'FILFINANCE_PROD_BUNNY_DEPLOY_ENABLED == "true"' not in section
        ):
            fail(f"missing Bunny production guard in {job_name}")

    calendar_run = job_section(pipeline, CALENDAR_RUN_JOB)
    if 'FILFINANCE_CALENDAR_RUN_AUTHORIZED == "true"' not in calendar_run:
        fail("calendar run must require explicit authorization")
    for variable in (
        "FILFINANCE_CALENDAR_RUN_DAY",
        "FILFINANCE_CALENDAR_RUN_EXPECTED_ID",
    ):
        if f"${variable}" not in calendar_run:
            fail(f"calendar run must require {variable} before becoming manual")
    if "when: manual" not in calendar_run or "allow_failure: false" not in calendar_run:
        fail("calendar run must remain manual and blocking when all locks are provided")

    backend_template = backend_template_section(pipeline)
    if '"${SSH_HOST_KEY_SHA256:?Missing SSH_HOST_KEY_SHA256}"' not in backend_template:
        fail("backend deploy must require the pinned SSH host fingerprint")
    if "StrictHostKeyChecking=yes" not in backend_template:
        fail("backend deploy must enforce strict SSH host key checking")
    if "SSH host key does not match SSH_HOST_KEY_SHA256" not in backend_template:
        fail("backend deploy must reject an SSH host key mismatch")
    if 'ssh-keyscan -p "$SSH_PORT" "$HOST" 2>/dev/null > ~/.ssh/known_hosts.candidate || true' not in backend_template:
        fail("backend deploy must retain usable host keys when optional algorithms are refused")
    if "test -s ~/.ssh/known_hosts.candidate" not in backend_template:
        fail("backend deploy must fail when no SSH host key candidate is received")
    if "label=com.docker.compose.project=$DEPLOY_COMPOSE_PROJECT_NAME" not in backend_template:
        fail("backend deploy must discover legacy n8n containers by Compose project")
    if "label=com.docker.compose.service=n8n" not in backend_template:
        fail("backend deploy must inventory the legacy n8n service")
    if "label=com.docker.compose.service=n8n-postgres" not in backend_template:
        fail("backend deploy must inventory the legacy n8n PostgreSQL service")
    if "docker rm -f 'filfinance_n8n'" in backend_template:
        fail("backend deploy must not silently remove hard-coded legacy n8n names")

    print("production deploy manual guard tests passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

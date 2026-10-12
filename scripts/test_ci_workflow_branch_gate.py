#!/usr/bin/env python3
"""Verifie que seuls main et dev peuvent creer une pipeline GitLab."""

from __future__ import annotations

from pathlib import Path


PIPELINE = Path(__file__).resolve().parent.parent / ".gitlab-ci.yml"
ALLOWED_BRANCHES_RULE = '- if: \'$CI_COMMIT_BRANCH == "main" || $CI_COMMIT_BRANCH == "dev"\''


def fail(message: str) -> None:
    raise SystemExit(f"CI workflow branch gate failed: {message}")


def workflow_section(pipeline: str) -> str:
    start = pipeline.find("workflow:\n")
    end = pipeline.find("variables:\n", start)
    if start < 0 or end < 0:
        fail("the top-level workflow section is missing")
    return pipeline[start:end]


def main() -> int:
    try:
        workflow = workflow_section(PIPELINE.read_text(encoding="utf-8"))
    except OSError:
        fail("pipeline configuration is unreadable")

    if ALLOWED_BRANCHES_RULE not in workflow:
        fail("workflow must allow only main and dev branches")
    if "- when: never" not in workflow:
        fail("workflow must explicitly reject every other pipeline source")
    if "CI_COMMIT_TAG" in workflow or "- if: '$CI_COMMIT_BRANCH'" in workflow:
        fail("workflow must not allow tags or arbitrary branches")

    print("CI workflow branch gate tests passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

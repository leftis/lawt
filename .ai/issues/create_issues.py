#!/usr/bin/env python3
"""Files the issue drafts in this folder on GitHub with the `gh` CLI.

Usage:
    python3 .ai/issues/create_issues.py --repo OWNER/NAME --dry-run   # show what would happen
    python3 .ai/issues/create_issues.py --repo OWNER/NAME             # create labels and issues

Needs `gh` with access to the repo (a fine-grained token with "Issues: read and write").
Safe to run again: issues whose title already exists (open or closed) are skipped.
"""

import argparse
import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent

LABEL_COLORS = {
    "security": "b60205", "feature": "1d76db", "tech-debt": "fbca04", "infra": "5319e7",
    "backend": "0e8a16", "frontend": "006b75", "pipeline": "c5def5", "decision": "d4c5f9",
    "epic": "3e4b9e", "docs": "0075ca", "product": "bfdadc", "ci": "ededed", "test": "c2e0c6",
    "ops": "f9d0c4", "performance": "fef2c0", "a11y": "bfd4f2",
    "P0": "b60205", "P1": "d93f0b", "P2": "fbca04", "P3": "cccccc",
}


def gh(*args: str, check: bool = True) -> str:
    result = subprocess.run(["gh", *args], capture_output=True, text=True)
    if check and result.returncode != 0:
        sys.exit(f"gh {' '.join(args[:3])} failed: {result.stderr.strip()}")
    return result.stdout


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", required=True, help="OWNER/NAME")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    issues = json.loads((HERE / "index.json").read_text())

    if args.dry_run:
        for i in issues:
            print(f"[dry run] {', '.join(i['labels']):45} {i['title']}")
        print(f"{len(issues)} issues would be created")
        return

    # Fails early with a clear message when the token cannot see the repo.
    gh("repo", "view", args.repo, "--json", "name")

    for label in sorted({l for i in issues for l in i["labels"]}):
        gh("label", "create", label, "--repo", args.repo, "--color", LABEL_COLORS.get(label, "ededed"),
           "--force", check=False)

    existing = {
        item["title"]
        for item in json.loads(gh("issue", "list", "--repo", args.repo, "--state", "all",
                                  "--limit", "500", "--json", "title"))
    }

    created = skipped = 0
    for i in issues:
        if i["title"] in existing:
            skipped += 1
            print(f"skip    {i['title']}")
            continue
        url = gh("issue", "create", "--repo", args.repo, "--title", i["title"],
                 "--body-file", str(HERE / i["file"]), "--label", ",".join(i["labels"])).strip()
        created += 1
        print(f"created {url}")
    print(f"done: {created} created, {skipped} skipped")


if __name__ == "__main__":
    main()

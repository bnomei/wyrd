---
emoji: "🏹"
name: Devana Bug Hunt
description: Hunt for source-visible runtime bugs and create one labeled issue for each new Devana report.
on:
  workflow_dispatch:
  schedule: daily

permissions:
  contents: read
  issues: read
  copilot-requests: write

runtimes:
  node:
    version: "24"

network:
  allowed:
    - defaults
    - github
    - node

pre-agent-steps:
  - name: Install Devana for GitHub Copilot
    working-directory: ${{ github.workspace }}
    run: |
      set -euo pipefail
      npx --yes skills add bnomei/devana \
        --skill devana-bug-hunt \
        --agent github-copilot \
        --copy \
        --yes
      test -f .agents/skills/devana-bug-hunt/SKILL.md

tools:
  edit:
  bash:
    - "cat"
    - "date"
    - "find"
    - "git:*"
    - "head"
    - "ls"
    - "pwd"
    - "rg"
    - "sed"
    - "sort"
    - "tail"
    - "wc"
  github:
    mode: gh-proxy
    min-integrity: approved
    toolsets: [issues]

safe-outputs:
  create-issue:
    title-prefix: "[devana] "
    labels: [bug]
    max: 20

timeout-minutes: 60
tracker-id: devana-bug-hunt
---

# Devana Bug Hunt

Hunt this repository for source-visible semantic runtime bugs and turn every
new report into a GitHub issue.

## Hunt

1. Read the repository guidance before inspecting source.
2. Invoke the installed `/devana-bug-hunt` skill with no arguments. Follow the
   skill exactly, including its static-only rule: do not run tests, builds,
   package installs, migrations, services, or network calls during the hunt.
3. Let the skill write any accepted findings under `.devana/` and finish its
   validation and duplicate checks before publishing anything.

The installation step before the agent starts is the only package installation
allowed in this workflow. It is not part of the hunt.

## Publish reports

After the hunt finishes:

1. Read every Markdown report written under `.devana/` during this run.
2. Search existing open and closed issues for its stable `DEVANA-KEY`, affected
   location, and finding title. Do not create an issue when the same finding is
   already tracked, even if its issue is closed.
3. For each new report, call `create_issue` once:
   - use the finding title without adding `[devana]` yourself; the safe output
     adds that prefix
   - copy the complete report into the issue body, preserving all deterministic
     `DEVANA-*` header and trailer lines
   - add a short final note that the issue was produced by an automated Devana
     hunt and must be validated against the current code before it is fixed
4. Create one issue per report. Never combine unrelated reports.

If Devana writes no reports, or every report already has an issue, call `noop`
with a concise explanation. Do not create a summary issue.

---
emoji: "🛠️"
name: Devana Work Queue
description: Validate and fix open Devana issues, producing one focused pull request per confirmed report.
on:
  workflow_dispatch:
  schedule: every 30m
  skip-if-no-match: "is:issue is:open label:devana"
  permissions:
    issues: read
    pull-requests: read

permissions:
  actions: read
  contents: read
  issues: read
  pull-requests: read
  copilot-requests: write

network:
  allowed:
    - defaults
    - dotnet
    - github
    - go
    - java
    - node
    - python
    - rust

tools:
  edit:
  bash: [":*"]
  github:
    mode: gh-proxy
    min-integrity: approved
    toolsets: [issues, pull_requests]

safe-outputs:
  create-pull-request:
    title-prefix: "[devana] "
    labels: [devana]
    draft: true
    max: 3
    protected-files: fallback-to-issue
  add-comment:
    target: "*"
    max: 3
  close-issue:
    target: "*"
    required-labels: [devana]
    max: 3

timeout-minutes: 60
concurrency:
  group: devana-work-queue
  cancel-in-progress: false
tracker-id: devana-work-queue
---

# Devana Work Queue

Work through the open issues labeled `devana` in `${{ github.repository }}`.
Each issue contains a report produced by the Devana bug-hunt skill. Reports are
candidates, not proof that the current code is still defective.

## Select work

1. Search for all open issues carrying the exact `devana` label, oldest first.
2. Exclude an issue when an open pull request already references or closes it.
3. Select at most three issues whose fixes are independent and are not expected
   to modify overlapping files. Select fewer when independence is uncertain.
4. Never work on an issue merely because its title contains `[devana]`; the
   label is the authority for this queue.

## Work each selected report

For each selected issue, separately:

1. Read the complete issue body and relevant comments.
2. Validate the report against the current code, repository guidance, callers,
   contracts, guards, and framework behavior. Reproduce its counterexample by
   reasoning before editing.
3. If the report is valid, make the smallest correct fix. Add or update focused
   tests when appropriate, then run the narrowest relevant formatter, test,
   typecheck, or build required by the repository.
4. Keep each issue's changes separate. Create one draft pull request for that
   issue, with a focused title and body containing:
   - `Fixes #<issue-number>`
   - the validated root cause and why the change blocks the counterexample
   - validation commands and their outcomes
   - an explicit note for anything that could not be verified
5. Do not combine multiple Devana issues into one pull request.

If a report is invalid, stale, or a duplicate, add a concise comment with the
evidence and close that issue. Do not change code and do not create a pull
request for it.

If no issue is currently actionable, call `noop` with the reason. Stop after
the selected issues are handled; later runs will continue through the queue.

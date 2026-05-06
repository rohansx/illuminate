# Illuminate — CI integration

This page describes the GitHub Action that runs `illuminate audit` against pull requests, and how to wire it into your repo.

## What it does

For every PR, the action:

1. Installs the `illuminate` CLI.
2. Rebuilds the wiki + graph (`illuminate wiki rebuild`).
3. Runs `illuminate audit "<plan>"` against the PR title (configurable).
4. Comments the audit output on the PR.
5. Fails the check (exit 2) on policy violations. Optionally fails on warnings.

## Setup

Copy `.github/workflows/example-audit-pr.yml.example` from the illuminate repo to your repo's `.github/workflows/audit-pr.yml`. Commit. The action runs on every PR open/edit/push.

The action expects a `.illuminate/illuminate.toml` and a `.illuminate/wiki/` in your repo. If they're not present, the audit still runs against any policies you pass inline, but graph queries return nothing.

## Inputs

| Input | Default | Meaning |
|-------|---------|---------|
| `plan` | `${{ github.event.pull_request.title }}` | Plan text to audit. Often you want to read the PR body — see "Custom plan" below. |
| `fail-on-warning` | `false` | If true, exit 1 (warning) also fails the check. |
| `illuminate-version` | `latest` (HEAD of master) | A semver if you want to pin. |

## Custom plan

To audit the PR body or a specific section, pre-process before calling the action:

```yaml
- name: Extract plan from PR body
  id: plan
  run: |
    PLAN=$(echo '${{ github.event.pull_request.body }}' | awk '/^## Plan/{flag=1;next}/^## /{flag=0}flag')
    echo "plan<<EOF" >> "$GITHUB_OUTPUT"
    echo "$PLAN" >> "$GITHUB_OUTPUT"
    echo "EOF" >> "$GITHUB_OUTPUT"

- uses: rohansx/illuminate/.github/actions/audit-pr@master
  with:
    plan: ${{ steps.plan.outputs.plan }}
```

## Local testing

To smoke-test without GitHub:

```bash
illuminate audit "add Redis caching to billing service"
echo "Exit: $?"
```

Exit codes match what the Action checks: `0` pass, `1` warning, `2` violation.

## Notes

- The action's `Comment on PR` step requires the default `GITHUB_TOKEN` write permission on `pull-requests` and `issues`. If your repo restricts this, set `permissions:` in the workflow.
- The action installs from source via `cargo install --git ...` which is slow (~2-3 min on a fresh runner). Cache `~/.cargo/registry` and `~/.cargo/bin` if you run frequently — or use a release prebuilt binary once those land.

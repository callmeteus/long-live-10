# Contributing

## Getting started

1. Read [README.md](README.md) and [ROADMAP.md](ROADMAP.md).
2. Follow the safety rules: never test destructive changes on the host.
3. Work in small increments with a passing build.

## Development process

- Every change needs tests appropriate to its risk, and the tests that were not run must be named.
- Architecture decisions go to `docs/decisions/` as short ADR files.
- Update the roadmap through the Stillmark CLI instead of editing `ROADMAP.md` by hand.
- Reversible changes need a documented rollback path.

## Planning with Stillmark

Planning files are split by audience:

| File | Audience | Versioned |
|------|----------|-----------|
| `ROADMAP.md` | Public strategy | Yes |
| `.stillmark/plans/TASK_PLAN.md` | Private execution plan | No |
| `.stillmark/plans/PROGRESS.md` | Private status | No |
| `.stillmark/plans/FINDINGS.md` | Private technical notes | No |

The CLI is installed from the [latest Stillmark release](https://github.com/callmeteus/stillmark/releases/latest):

```bash
npm install -g https://github.com/callmeteus/stillmark/releases/latest/download/stillmark.tgz
```

Stillmark does not ship an OpenCode adapter yet. Install the agent skill manually:

```powershell
New-Item -ItemType Directory -Force "$env:USERPROFILE\.config\opencode\skills\stillmark"
Copy-Item -Recurse -Force "<stillmark-repo>\templates\cursor\.cursor\skills\stillmark\*" `
    "$env:USERPROFILE\.config\opencode\skills\stillmark\"
```

Copy `templates/cursor/.cursor/commands/stillmark.md` to `$env:USERPROFILE\.config\opencode\commands\stillmark.md` and append `$ARGUMENTS` at the end.

Then use `/stillmark roadmap add "..."`, `/stillmark plan add "..."`, and the other commands. Never edit `ROADMAP.md` or the plan by hand.

## Commits

Commit messages are written in Brazilian Portuguese. Keep the subject short and describe what changed for the project.

## Safety rules

- Experimental patches, drivers, and system modifications run only in disposable VMs.
- Host credentials, tokens, and personal directories are never exposed to a guest.
- Network access in test VMs is disabled unless the test requires it.
- A passing installer does not prove a fix works; evidence must show the mitigated behavior.

# Tools

Dev utilities for the SkyOS workspace. All are stdlib-only Python 3 (or PowerShell),
each prints `--help`, and exits 1 on findings so they can gate CI.

## duplicate-finder

| Tool | What it does |
|------|--------------|
| `find_duplicates.py [roots...] [--min-size N]` | Exact duplicate files by SHA-256 content hash. `--selfcheck` to verify. |
| `find_similar_files.py [roots...] [--threshold 0.6]` | Near-duplicate files (copy-paste with light edits) via normalized line-shingle similarity. `--selfcheck` to verify. |

## audit

| Tool | What it does |
|------|--------------|
| `markers.py [--kind todo|fixme|...] [--top N]` | TODO/FIXME/HACK/XXX inventory. |
| `unwraps.py [--top N]` | `unwrap()`/`expect()` counts in Rust — AGENTS.md bans them. |
| `trailing_ws.py [roots...]` | Trailing whitespace + missing final newline check. |

## stats

| Tool | What it does |
|------|--------------|
| `lines.py [--top N]` | LOC by extension + biggest files. |
| `size_report.py [--repo-root .]` | Kernel ELF / bootimage / disk image sizes. |
| `git_churn.py [--since "90 days ago"]` | Per-file commit counts — bug hotspots. |

## cleanup

| Tool | What it does |
|------|--------------|
| `junk.py [roots...] [--delete]` | `__pycache__`, `.pyc`, `.tmp`, `.bak`, etc. — preview, then delete. |
| `bigfiles.py [--top N]` | Largest files in the tree. |

## gate

| Tool | What it does |
|------|--------------|
| `run_gate.ps1` | One-shot: kernel build (self_test) → bootimage → `boot_stress.py --tries 10`. Run this after any kernel change. |

## Running everything

```powershell
# sanity-check the non-trivial tools
py tools\duplicate-finder\find_duplicates.py --selfcheck
py tools\duplicate-finder\find_similar_files.py --selfcheck

# full audit sweep of the workspace
py tools\audit\markers.py .
py tools\audit\unwraps.py .
py tools\audit\trailing_ws.py .
```

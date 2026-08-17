# formsy-cli (`fsy`)

Rust CLI client for the `formsy.server` HTTP API (`packages/server/src/formsy/server/routes/code.py`).
Covers the two endpoints requested today — `/api/v1/compile` and `/api/v1/query` — plus a
combined `search` subcommand that replays the compile → query flow from
`scripts/e2e_server_compile_query.py`.

## Install / build

```bash
cd formsy-cli
cargo build --release
# binary: ./target/release/fsy
```

Every build exposes its source identity and protocol capabilities. Automation must consume the
JSON document instead of inferring compatibility from the package version alone:

```bash
fsy --version
fsy capabilities --json
```

`--version` includes the Git commit, dirty state, and build target. `capabilities --json` is the
machine contract used by runners before repository collection or any server/model request.

## Global options (apply to every subcommand)

| flag | default | notes |
|------|---------|-------|
| `--base-url <URL>` | `http://127.0.0.1:8080` | Running `formsy.server` base URL |
| `--api-key <KEY>` | `fsy_test_key_dev_only_12345678` | Sent as `Authorization: Bearer <KEY>` (matches the e2e script) |
| `--timeout <SECS>` | `1200` | Per-request HTTP timeout (matches the e2e script) |

## Subcommands

### `fsy compile` — ingest source files

Collects source files under `--repo-root` and posts them to `/api/v1/compile`. By default, file
collection automatically includes the languages supported by CodeGraph. In Git worktrees it uses
tracked files plus non-ignored working-tree files; ignored dependencies and build outputs are not
uploaded. Use `--extensions` only to restrict or override that automatic selection. Payload paths
are repo-relative POSIX paths and each file carries its detected source language.

```bash
fsy compile --repo-id my-repo --repo-root ./packages/server/src
fsy compile --repo-id my-repo --repo-root ./src --mode replace --extensions py,ts --json
```

Flags: `--repo-id`, `--repo-root`, `--mode merge|replace`, `--revision`, `--extensions py,ts`,
`--task-id`, `--task-revision`, `--task-file`, `--test-file-mutation-policy`, `--json`.

### `fsy query` — natural-language repo query

```bash
fsy query --repo-id my-repo --query "Where is compile_repo implemented?"
fsy query --repo-id my-repo --revision <REV> --query "..." --intent symbol_definition --budget 6000
```

Flags: `--repo-id`, `--query`, `--revision`, `--intent`, `--budget`,
`--response-format bundle|legacy`, `--task-id`, `--task-revision`, `--json`.

### `fsy search` — compile then query in one shot

Runs `compile` (collecting `--repo-root`), then `query` against the returned revision, then
prints a summary. Equivalent to the e2e script's compile → query leg.

```bash
fsy search --repo-id my-repo --repo-root ./packages/server/src \
  --query "Where are the compile and query endpoints implemented?"
```

## Output

By default each subcommand prints a short human-readable summary (`[ok] ...` lines). Pass
`--json` to print the raw server JSON response instead (pipe-friendly).

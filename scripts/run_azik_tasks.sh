#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TASK_DOC="$ROOT_DIR/docs/azik_implementation_tasks.md"
SPEC_DOC="$ROOT_DIR/docs/azik_extension_spec.md"

TASK_IDS=(T01 T02 T03 T04 T05 T08 T06 T07 T09 T10 T11)
MAX_FIX_ATTEMPTS=3
FROM_TASK="${FROM_TASK:-}"
TO_TASK="${TO_TASK:-}"

usage() {
  cat <<'EOF'
Usage:
  scripts/run_azik_tasks.sh [--from T04] [--to T09] [--max-fixes 5]

Environment variables:
  FROM_TASK         Start from this task ID.
  TO_TASK           Stop after this task ID.
  MAX_FIX_ATTEMPTS  Maximum validator-driven repair attempts per task.

Notes:
  - The script expects a clean git worktree before it starts.
  - Each implementation step runs in a fresh Codex session.
  - Each validation step runs in a separate fresh Codex session.
  - On validator `OK`, the script commits the task.
  - On validator `NG`, the script asks a fresh Codex session to repair and re-validates.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --from)
      FROM_TASK="$2"
      shift 2
      ;;
    --to)
      TO_TASK="$2"
      shift 2
      ;;
    --max-fixes)
      MAX_FIX_ATTEMPTS="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [[ ! -f "$TASK_DOC" || ! -f "$SPEC_DOC" ]]; then
  echo "Required docs are missing." >&2
  exit 1
fi

tmpdir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmpdir"
}
trap cleanup EXIT

schema_file="$tmpdir/validator_schema.json"
cat > "$schema_file" <<'EOF'
{
  "type": "object",
  "properties": {
    "result": {
      "type": "string",
      "enum": ["OK", "NG"]
    },
    "summary": {
      "type": "string"
    },
    "issues": {
      "type": "array",
      "items": {
        "type": "string"
      }
    }
  },
  "required": ["result", "summary", "issues"],
  "additionalProperties": false
}
EOF

ensure_clean_worktree() {
  local status
  status="$(git -C "$ROOT_DIR" status --short --untracked-files=all | grep -vE '^[?][?] \.codex$' || true)"
  if [[ -n "$status" ]]; then
    echo "Worktree must be clean before running automation." >&2
    echo "$status" >&2
    exit 1
  fi
}

update_task_status() {
  local task_id="$1"
  local status="$2"
  python3 - "$TASK_DOC" "$task_id" "$status" <<'PY'
import pathlib
import re
import sys

path = pathlib.Path(sys.argv[1])
task_id = sys.argv[2]
status = sys.argv[3]
text = path.read_text()

row_pattern = re.compile(rf'^(\| {task_id} \| )([^|]+)( \| .*)$', re.M)
text, row_count = row_pattern.subn(lambda m: f"{m.group(1)}{status}{m.group(3)}", text, count=1)
if row_count != 1:
    raise SystemExit(f"failed to update table row for {task_id}")

section_pattern = re.compile(rf'^(### {task_id}\b[^\n]*\n\n- Status: )`[^`]+`', re.M)
text, section_count = section_pattern.subn(lambda m: f"{m.group(1)}`{status}`", text, count=1)
if section_count != 1:
    raise SystemExit(f"failed to update section status for {task_id}")

path.write_text(text)
PY
}

task_title() {
  case "$1" in
    T01) echo "拡張トークン定義と共通ユーティリティ追加" ;;
    T02) echo "日本語前処理器の実装" ;;
    T03) echo "n-gram DB 生成時に source ごとの前処理を適用" ;;
    T04) echo "拡張付きキー表現の導入" ;;
    T05) echo "拡張付きキーの検索・評価 API 実装" ;;
    T06) echo "遺伝的アルゴリズムに拡張割当遺伝子を追加" ;;
    T07) echo "交叉・突然変異戦略の分離" ;;
    T08) echo "レイアウト出力の可読化" ;;
    T09) echo "日本語/英語の別評価と重み合成の整理" ;;
    T10) echo "単体テスト追加" ;;
    T11) echo "統合確認とリファクタ" ;;
    *) echo "unknown task" ;;
  esac
}

task_commit_message() {
  case "$1" in
    T01) echo "feat(azik): add extension token utilities" ;;
    T02) echo "feat(azik): add Japanese token preprocessor" ;;
    T03) echo "feat(azik): preprocess Japanese corpus for n-grams" ;;
    T04) echo "feat(azik): add extension-aware key model" ;;
    T05) echo "feat(azik): evaluate extension tokens on parent keys" ;;
    T06) echo "feat(azik): add extension genes to GA" ;;
    T07) echo "feat(azik): split base and extension mutations" ;;
    T08) echo "feat(azik): print extension assignments in layouts" ;;
    T09) echo "feat(azik): separate JA and EN layout evaluation" ;;
    T10) echo "test(azik): cover preprocessing and extension rules" ;;
    T11) echo "chore(azik): verify and polish integration" ;;
    *) echo "chore: update $1" ;;
  esac
}

task_prompt() {
  local task_id="$1"
  local title
  title="$(task_title "$task_id")"
  cat <<EOF
Implement ${task_id}: ${title}.

Constraints:
- Read AGENTS.md first.
- Follow docs/azik_extension_spec.md and docs/azik_implementation_tasks.md.
- Work only on ${task_id}, except for minimal prerequisite refactors strictly needed to complete it.
- Update tests when the task requires it.
- Run relevant commands from AGENTS.md for validation.
- Do not create a git commit.
- Leave the repository ready for an external validator.

When done, summarize:
1. files changed
2. checks run
3. any residual risk
EOF
}

validator_prompt() {
  local task_id="$1"
  local title
  title="$(task_title "$task_id")"
  cat <<EOF
You are validating ${task_id}: ${title}.

Repository context:
- Spec: docs/azik_extension_spec.md
- Task board: docs/azik_implementation_tasks.md
- Focus only on completion criteria for ${task_id} and regressions introduced by the current uncommitted changes.

Rules:
- Do not modify files.
- Do not create commits.
- Run checks if needed.
- Return JSON only, matching the provided schema.
- Set "result" to "OK" only if the task is complete and the current changes are acceptable to commit.
- Otherwise set "result" to "NG" and provide actionable items in "issues".
EOF
}

repair_prompt() {
  local task_id="$1"
  local review_json="$2"
  local title
  title="$(task_title "$task_id")"
  cat <<EOF
Repair ${task_id}: ${title} based on the validator feedback below.

Rules:
- Read AGENTS.md first.
- Follow docs/azik_extension_spec.md and docs/azik_implementation_tasks.md.
- Fix only the issues needed for ${task_id} to pass validation.
- Do not create a git commit.

Validator feedback JSON:
${review_json}
EOF
}

run_codex_exec() {
  local sandbox="$1"
  local output_file="$2"
  local prompt_file="$3"
  codex exec \
    --full-auto \
    --sandbox "$sandbox" \
    --cd "$ROOT_DIR" \
    --ephemeral \
    --color never \
    -o "$output_file" \
    - < "$prompt_file" > /dev/null
}

run_validator() {
  local task_id="$1"
  local prompt_file="$tmpdir/validator_${task_id}.txt"
  local output_file="$tmpdir/validator_${task_id}.json"

  validator_prompt "$task_id" > "$prompt_file"

  codex exec \
    --full-auto \
    --sandbox workspace-write \
    --cd "$ROOT_DIR" \
    --ephemeral \
    --color never \
    --output-schema "$schema_file" \
    -o "$output_file" \
    - < "$prompt_file" > /dev/null

  cat "$output_file"
}

json_result_field() {
  local json_file="$1"
  local field="$2"
  python3 - "$json_file" "$field" <<'PY'
import json
import sys

with open(sys.argv[1]) as f:
    data = json.load(f)

value = data[sys.argv[2]]
if isinstance(value, list):
    for item in value:
        print(item)
else:
    print(value)
PY
}

task_is_selected() {
  local task_id="$1"
  local started=0
  local ended=0

  if [[ -z "$FROM_TASK" ]]; then
    started=1
  fi

  for id in "${TASK_IDS[@]}"; do
    if [[ "$id" == "$FROM_TASK" ]]; then
      started=1
    fi
    if [[ "$id" == "$task_id" && "$started" -eq 1 && "$ended" -eq 0 ]]; then
      return 0
    fi
    if [[ -n "$TO_TASK" && "$id" == "$TO_TASK" ]]; then
      ended=1
    fi
  done

  return 1
}

commit_current_task() {
  local task_id="$1"
  local commit_message
  commit_message="$(task_commit_message "$task_id")"
  git -C "$ROOT_DIR" add -A
  git -C "$ROOT_DIR" restore --staged .codex >/dev/null 2>&1 || true
  git -C "$ROOT_DIR" commit -m "$commit_message"
}

main() {
  ensure_clean_worktree

  for task_id in "${TASK_IDS[@]}"; do
    if ! task_is_selected "$task_id"; then
      continue
    fi

    local title
    title="$(task_title "$task_id")"
    echo "==> ${task_id}: ${title}"

    update_task_status "$task_id" "DOING"

    local impl_prompt="$tmpdir/impl_${task_id}.txt"
    local impl_output="$tmpdir/impl_${task_id}.out"
    task_prompt "$task_id" > "$impl_prompt"
    run_codex_exec "workspace-write" "$impl_output" "$impl_prompt"

    local attempt
    for ((attempt = 1; attempt <= MAX_FIX_ATTEMPTS; attempt++)); do
      echo "   validation attempt ${attempt}/${MAX_FIX_ATTEMPTS}"
      local review_json
      review_json="$(run_validator "$task_id")"

      local review_file="$tmpdir/review_${task_id}.json"
      printf '%s\n' "$review_json" > "$review_file"

      local result
      result="$(json_result_field "$review_file" result)"
      local summary
      summary="$(json_result_field "$review_file" summary)"
      echo "   validator result: ${result}"
      echo "   summary: ${summary}"

      if [[ "$result" == "OK" ]]; then
        update_task_status "$task_id" "DONE"
        commit_current_task "$task_id"
        break
      fi

      if (( attempt == MAX_FIX_ATTEMPTS )); then
        echo "Validator did not approve ${task_id} after ${MAX_FIX_ATTEMPTS} attempts." >&2
        update_task_status "$task_id" "BLOCKED"
        exit 1
      fi

      echo "   issues:"
      json_result_field "$review_file" issues | sed 's/^/   - /'

      local repair_prompt_file="$tmpdir/repair_${task_id}_${attempt}.txt"
      local repair_output="$tmpdir/repair_${task_id}_${attempt}.out"
      repair_prompt "$task_id" "$review_json" > "$repair_prompt_file"
      run_codex_exec "workspace-write" "$repair_output" "$repair_prompt_file"
    done

    if [[ -n "$TO_TASK" && "$task_id" == "$TO_TASK" ]]; then
      break
    fi
  done
}

main "$@"

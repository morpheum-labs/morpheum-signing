#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SKILL_DIR=$(cd "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(cd "$SKILL_DIR/../../.." && pwd)

TOOLCHAIN=1.98.1
export CARGO_TERM_COLOR=never

usage() {
  cat >&2 <<'EOF'
usage: verify-morpheum-signing.sh <command>

  doctor
  launch
  drive-native-sign
  drive-mnemonic-native
  drive-agent-claim
  drive-agent-claim-verify
  cleanup

VERIFY_RUN_ID is required for launch, drive-*, and cleanup.
doctor writes evidence/$VERIFY_RUN_ID/doctor/ when VERIFY_RUN_ID is set.
A run id matches ^[A-Za-z0-9][A-Za-z0-9._-]{0,80}$.
EOF
  exit 2
}

die() {
  printf 'verify-morpheum-signing: %s\n' "$*" >&2
  exit 1
}

require_run_id() {
  local id=${VERIFY_RUN_ID:-}
  if [[ ! $id =~ ^[A-Za-z0-9][A-Za-z0-9._-]{0,80}$ ]]; then
    echo "VERIFY_RUN_ID must match ^[A-Za-z0-9][A-Za-z0-9._-]{0,80}\$ (got: ${id:-empty})" >&2
    exit 2
  fi
}

scratch_dir() {
  printf '/tmp/verify-morpheum-signing-%s\n' "$VERIFY_RUN_ID"
}

evidence_root() {
  printf '%s/evidence/%s\n' "$SKILL_DIR" "$VERIFY_RUN_ID"
}

repo_parent() {
  local parent
  parent=$(dirname "$REPO_ROOT")
  if [[ $parent == / ]]; then
    printf '/\n'
  else
    printf '%s\n' "$parent"
  fi
}

sibling_manifest() {
  local name=$1
  local parent
  parent=$(repo_parent)
  if [[ $parent == / ]]; then
    printf '/%s/Cargo.toml\n' "$name"
  else
    printf '%s/%s/Cargo.toml\n' "$parent" "$name"
  fi
}

reject_rustflags() {
  if [[ -n ${RUSTFLAGS:-} || -n ${CARGO_ENCODED_RUSTFLAGS:-} ]]; then
    die "refusing to build with RUSTFLAGS or CARGO_ENCODED_RUSTFLAGS set"
  fi
}

ensure_toolchain() {
  if ! rustup run "$TOOLCHAIN" rustc --version >/dev/null 2>&1; then
    rustup toolchain install "$TOOLCHAIN" --profile minimal --component rustfmt,clippy || return 1
  fi
  if ! rustup run "$TOOLCHAIN" rustfmt --version >/dev/null 2>&1; then
    rustup component add rustfmt --toolchain "$TOOLCHAIN" || return 1
  fi
  if ! rustup run "$TOOLCHAIN" cargo clippy --version >/dev/null 2>&1; then
    rustup component add clippy --toolchain "$TOOLCHAIN" || return 1
  fi
  # prost-build shells out to protoc. openssl-sys (pulled in by native-tls
  # on the resolved graph) needs pkg-config and libssl.
  if ! command -v protoc >/dev/null 2>&1 \
    || ! command -v pkg-config >/dev/null 2>&1 \
    || ! pkg-config --exists openssl
  then
    sudo apt-get update -qq || return 1
    sudo apt-get install -y -qq protobuf-compiler pkg-config libssl-dev || return 1
    hash -r
  fi
  command -v protoc >/dev/null 2>&1 || return 1
  pkg-config --exists openssl
}

redact_file() {
  local src=$1 dest=$2
  sed -E \
    -e 's#(ghp_|gho_|ghs_|github_pat_)[A-Za-z0-9_]+#\1[REDACTED]#g' \
    -e 's#(x-access-token:)[^@[:space:]]+#\1[REDACTED]#g' \
    -e 's#((priv|private_key|secret_key|priv_hex)[[:space:]]*[:=][[:space:]]*)[0-9A-Fa-f]{64}#\1[REDACTED]#gI' \
    "$src" >"$dest"
}

leaked_secret() {
  grep -E -i -q '(priv|private_key|secret_key|priv_hex)[[:space:]]*[:=][[:space:]]*[0-9A-Fa-f]{64}' "$1"
}

write_result() {
  local dest=$1 status=$2
  local tmp
  mkdir -p "$dest"
  tmp=$(mktemp "$dest/result.txt.XXXXXX")
  printf '%s\n' "$status" >"$tmp"
  mv "$tmp" "$dest/result.txt"
}

load_feature() {
  local id=$1
  FEATURE_ID=$id
  FEATURE_EXACT=''
  FEATURE_PREFIX=''
  FEATURE_ABSENT=''
  case $id in
    native-sign)
      FEATURE_EXAMPLE=native
      FEATURE_BANNER='Morpheum Native Signing Example'
      FEATURE_SUCCESS='Transaction signed successfully!'
      FEATURE_TX_RE='^  TxHash          : [0-9a-f]{64}$'
      FEATURE_RAW_RE='^  Raw bytes len   : [1-9][0-9]* bytes$'
      ;;
    mnemonic-native)
      FEATURE_EXAMPLE=native
      FEATURE_BANNER='Morpheum Native Signing Example'
      FEATURE_SUCCESS='Transaction signed successfully!'
      FEATURE_TX_RE='^  TxHash          : [0-9a-f]{64}$'
      FEATURE_RAW_RE='^  Raw bytes len   : [1-9][0-9]* bytes$'
      FEATURE_PREFIX='  Signer (from mnemonic) account:'
      ;;
    agent-claim)
      FEATURE_EXAMPLE=agent
      FEATURE_BANNER='Morpheum AI Agent Signing Example'
      FEATURE_SUCCESS='Agent transaction signed successfully with TradingKey + VC claim!'
      FEATURE_TX_RE='^  TxHash          : [0-9a-f]{64}$'
      FEATURE_RAW_RE='^  Raw bytes len   : [1-9][0-9]* bytes$'
      FEATURE_EXACT='  Claim sub-range size: 1000'
      ;;
    agent-claim-verify)
      FEATURE_EXAMPLE=agent_with_claim_verification
      FEATURE_BANNER='Morpheum Agent + Claim Verification Example'
      FEATURE_SUCCESS='Transaction signed successfully with verified claim!'
      FEATURE_TX_RE='^  TxHash        : [0-9a-f]{64}$'
      FEATURE_RAW_RE='^  Raw bytes len : [1-9][0-9]* bytes$'
      FEATURE_EXACT=$'  Claim verification: PASSED\n  Claim embedded: YES (in SignerInfo.signing_options)'
      FEATURE_ABSENT='Claim verification: FAILED'
      ;;
    *)
      die "unknown feature: $id"
      ;;
  esac
}

note() {
  printf '%s\n' "$*" >>"$CHECK_FILE"
}

judge_stdout() {
  local file=$1
  local line
  if grep -F -x -q -- "$FEATURE_BANNER" "$file"; then
    note "ok banner"
  else
    note "missing banner: $FEATURE_BANNER"
  fi
  if grep -F -x -q -- "$FEATURE_SUCCESS" "$file"; then
    note "ok success"
  else
    note "missing success: $FEATURE_SUCCESS"
  fi
  if grep -E -q -- "$FEATURE_TX_RE" "$file"; then
    note "ok txhash"
  else
    note "missing txhash"
  fi
  if grep -E -q -- 'TxHash.*: 0{64}$' "$file"; then
    note "txhash is 64 zeros"
  fi
  if grep -E -q -- "$FEATURE_RAW_RE" "$file"; then
    note "ok raw_bytes"
  else
    note "missing raw bytes len"
  fi
  if [[ -n $FEATURE_EXACT ]]; then
    while IFS= read -r line; do
      [[ -z $line ]] && continue
      if grep -F -x -q -- "$line" "$file"; then
        note "ok exact: $line"
      else
        note "missing exact: $line"
      fi
    done <<<"$FEATURE_EXACT"
  fi
  if [[ -n $FEATURE_PREFIX ]]; then
    while IFS= read -r line; do
      [[ -z $line ]] && continue
      if grep -F -q -- "$line" "$file"; then
        note "ok prefix: $line"
      else
        note "missing prefix: $line"
      fi
    done <<<"$FEATURE_PREFIX"
  fi
  if [[ -n $FEATURE_ABSENT ]]; then
    while IFS= read -r line; do
      [[ -z $line ]] && continue
      if grep -F -q -- "$line" "$file"; then
        note "forbidden: $line"
      else
        note "ok absent: $line"
      fi
    done <<<"$FEATURE_ABSENT"
  fi
}

checks_ok() {
  ! grep -q -E '^(missing|forbidden|txhash is|secret leaked|exit |sha256 |binary |launch )' "$CHECK_FILE"
}

record_siblings() {
  local name path
  local fail=0
  for name in cryptogram morpheum-primitives morpheum-proto; do
    path=$(sibling_manifest "$name")
    if [[ -f $path ]]; then
      printf 'sibling_%s=present\n' "$name"
    else
      printf 'sibling_%s=missing path=%s\n' "$name" "$path"
      fail=1
    fi
  done
  return "$fail"
}

doctor() {
  local fail=0
  local ver

  echo "repo_root=$REPO_ROOT"
  if [[ -f $REPO_ROOT/Cargo.toml ]] && grep -q 'crates/native' "$REPO_ROOT/Cargo.toml"; then
    echo "workspace=morpheum-signing"
  else
    echo "workspace=NOT_MORPHEUM_SIGNING"
    fail=1
  fi

  if [[ -f $REPO_ROOT/crates/native/Cargo.toml ]] && grep -q 'name = "morpheum-signing-native"' "$REPO_ROOT/crates/native/Cargo.toml"; then
    echo "package=morpheum-signing-native"
  else
    echo "package=NOT_NATIVE"
    fail=1
  fi

  local src
  for src in \
    crates/native/examples/native.rs \
    crates/native/examples/agent.rs \
    crates/native/examples/agent_with_claim_verification.rs \
    Cargo.lock
  do
    if [[ -f $REPO_ROOT/$src ]]; then
      echo "source_present $src"
    else
      echo "source_missing $src"
      fail=1
    fi
  done

  local rel dest
  for rel in \
    ../morpheum-primitives \
    ../cryptogram/crates/morpheum-crypto \
    ../cryptogram/crates/cryptogram-crypto \
    ../cryptogram/crates/cryptogram-hd
  do
    if grep -q "path = \"$rel\"" "$REPO_ROOT/Cargo.toml"; then
      echo "path_dep $rel"
    else
      echo "path_dep_missing $rel"
      fail=1
    fi
    dest=$(repo_parent)
    if [[ $dest == / ]]; then
      dest="/${rel#../}/Cargo.toml"
    else
      dest="$dest/${rel#../}/Cargo.toml"
    fi
    if [[ -f $dest ]]; then
      echo "path_ok $dest"
    else
      echo "path_missing $dest"
      fail=1
    fi
  done

  if record_siblings; then
    echo "siblings=present"
  else
    echo "siblings=missing"
    fail=1
  fi

  echo "toolchain_pin=$TOOLCHAIN"
  if command -v rustup >/dev/null 2>&1; then
    echo "rustup=present"
  else
    echo "rustup=missing"
    fail=1
  fi
  if command -v rustup >/dev/null 2>&1 && rustup run "$TOOLCHAIN" rustc --version >/dev/null 2>&1; then
    ver=$(rustup run "$TOOLCHAIN" rustc --version)
    echo "toolchain_installed=yes"
    echo "rustc=$ver"
  else
    echo "toolchain_installed=no"
  fi
  if command -v protoc >/dev/null 2>&1; then
    ver=$(protoc --version)
    echo "protoc=present"
    echo "protoc_version=$ver"
  else
    echo "protoc=missing"
  fi
  if command -v pkg-config >/dev/null 2>&1 && pkg-config --exists openssl; then
    echo "openssl=present"
  else
    echo "openssl=missing"
  fi
  if git -C "$REPO_ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    echo "git_head=$(git -C "$REPO_ROOT" rev-parse HEAD)"
  fi

  if [[ $fail -eq 0 ]]; then
    echo "doctor=OK"
    return 0
  fi
  echo "doctor=NOT_READY"
  return 1
}

cmd_doctor() {
  local log code dest
  log=$(mktemp)
  set +e
  doctor >"$log" 2>&1
  code=$?
  set -e
  cat "$log"
  if [[ -n ${VERIFY_RUN_ID:-} ]]; then
    require_run_id
    dest=$(evidence_root)/doctor
    mkdir -p "$dest"
    cp "$log" "$dest/transcript.txt"
    if [[ $code -eq 0 ]]; then
      write_result "$dest" PASS
    else
      write_result "$dest" FAIL
    fi
  fi
  rm -f "$log"
  return "$code"
}

example_bin() {
  local scratch
  scratch=$(scratch_dir)
  printf '%s/target/debug/examples/%s\n' "$scratch" "$1"
}

cmd_launch() {
  require_run_id
  reject_rustflags

  local scratch target ev code out err summary
  scratch=$(scratch_dir)
  target=$scratch/target
  ev=$(evidence_root)/launch
  mkdir -p "$ev" "$scratch"

  summary=$(mktemp)
  {
    echo "repo_root=$REPO_ROOT"
    if ! record_siblings; then
      echo "launch=FAIL"
      echo "siblings=missing"
    fi
  } >"$summary"

  if grep -q 'siblings=missing' "$summary" || grep -q '=missing path=' "$summary"; then
    cp "$summary" "$ev/summary.txt"
    write_result "$ev" FAIL
    cat "$summary"
    rm -f "$summary"
    die "sibling checkout missing. Do not clone from this helper."
  fi

  if ! ensure_toolchain; then
    echo "toolchain_install=FAIL" >>"$summary"
    echo "launch=FAIL" >>"$summary"
    cp "$summary" "$ev/summary.txt"
    write_result "$ev" FAIL
    cat "$summary"
    rm -f "$summary"
    die "toolchain or protoc install failed"
  fi
  LOCK_BACKUP=$(mktemp)
  cp "$REPO_ROOT/Cargo.lock" "$LOCK_BACKUP"
  restore_committed_lockfile() {
    if [[ -n ${LOCK_BACKUP:-} && -f $LOCK_BACKUP ]]; then
      cp "$LOCK_BACKUP" "$REPO_ROOT/Cargo.lock"
      rm -f "$LOCK_BACKUP"
      LOCK_BACKUP=''
    fi
  }
  trap restore_committed_lockfile EXIT

  {
    echo "rustc=$(rustup run "$TOOLCHAIN" rustc --version)"
    echo "protoc=$(protoc --version)"
    echo "command=rustup run $TOOLCHAIN cargo build -p morpheum-signing-native --examples"
    echo "CARGO_TARGET_DIR=$target"
  } >>"$summary"

  out=$(mktemp)
  err=$(mktemp)
  set +e
  (
    cd "$REPO_ROOT"
    export CARGO_TARGET_DIR=$target
    export CARGO_TERM_COLOR=never
    timeout 1800 rustup run "$TOOLCHAIN" cargo build -p morpheum-signing-native --examples
  ) >"$out" 2>"$err"
  code=$?
  set -e
  restore_committed_lockfile
  echo "lockfile_restored=Cargo.lock" >>"$summary"

  printf '%s\n' "$code" >"$ev/exit-code.txt"
  redact_file "$out" "$ev/stdout.txt"
  redact_file "$err" "$ev/stderr.txt"
  printf '%s\n' "rustup run $TOOLCHAIN cargo build -p morpheum-signing-native --examples" >"$ev/command.txt"
  printf 'CARGO_TARGET_DIR=%s\n' "$target" >>"$ev/command.txt"

  : >"$ev/binaries.txt"
  local name bin hash
  for name in native agent agent_with_claim_verification; do
    bin=$(example_bin "$name")
    if [[ -x $bin ]]; then
      hash=$(sha256sum "$bin" | awk '{print $1}')
      printf '%s %s %s\n' "$name" "$hash" "$bin" >>"$ev/binaries.txt"
      echo "binary $name sha256=$hash path=$bin" >>"$summary"
    else
      echo "binary_missing $name path=$bin" >>"$summary"
    fi
  done

  local ready=0
  if [[ $code -ne 0 ]]; then
    echo "cargo_exit=$code" >>"$summary"
    ready=1
  fi
  if ! grep -E -q 'Finished `dev` profile' "$ev/stdout.txt" "$ev/stderr.txt"; then
    echo "missing Finished dev profile line" >>"$summary"
    ready=1
  fi
  if [[ $(wc -l <"$ev/binaries.txt") -ne 3 ]]; then
    echo "expected 3 example binaries" >>"$summary"
    ready=1
  fi
  if leaked_secret "$out" || leaked_secret "$err"; then
    echo "secret leaked in cargo log" >>"$summary"
    ready=1
  fi

  if [[ $ready -eq 0 ]]; then
    echo "launch=OK" >>"$summary"
    write_result "$ev" PASS
  else
    echo "launch=FAIL" >>"$summary"
    write_result "$ev" FAIL
  fi

  cp "$summary" "$ev/summary.txt"
  cat "$summary"
  rm -f "$summary" "$out" "$err"
  [[ $ready -eq 0 ]]
}

cmd_drive() {
  require_run_id
  load_feature "$1"

  local ev bin bins expected actual out err code
  ev=$(evidence_root)/$FEATURE_ID
  mkdir -p "$ev"
  bin=$(example_bin "$FEATURE_EXAMPLE")
  bins=$(evidence_root)/launch/binaries.txt
  CHECK_FILE=$ev/checks.txt
  : >"$CHECK_FILE"

  printf 'feature=%s\nexample=%s\nargv=%s\n' "$FEATURE_ID" "$FEATURE_EXAMPLE" "$bin" >"$ev/command.txt"
  printf 'built_by=rustup run %s cargo build -p morpheum-signing-native --examples\n' "$TOOLCHAIN" >>"$ev/command.txt"

  if [[ ! -x $bin ]]; then
    note "binary missing: $bin"
    write_result "$ev" FAIL
    write_result "$(evidence_root)" FAIL
    cat "$CHECK_FILE"
    die "example binary missing. Run launch for this VERIFY_RUN_ID."
  fi
  if [[ ! -f $bins ]]; then
    note "launch evidence missing: $bins"
    write_result "$ev" FAIL
    write_result "$(evidence_root)" FAIL
    cat "$CHECK_FILE"
    die "launch evidence missing. Run launch for this VERIFY_RUN_ID."
  fi

  expected=$(awk -v name="$FEATURE_EXAMPLE" '$1 == name { print $2; exit }' "$bins")
  actual=$(sha256sum "$bin" | awk '{print $1}')
  printf '%s\n' "$bin" >"$ev/binary-path.txt"
  printf '%s\n' "$actual" >"$ev/binary-sha256.txt"
  if [[ -z $expected || $expected != "$actual" ]]; then
    note "sha256 mismatch expected=${expected:-empty} actual=$actual"
  else
    note "ok sha256"
  fi

  out=$(mktemp)
  err=$(mktemp)
  set +e
  timeout 120 "$bin" >"$out" 2>"$err"
  code=$?
  set -e
  printf '%s\n' "$code" >"$ev/exit-code.txt"
  redact_file "$out" "$ev/stdout.txt"
  redact_file "$err" "$ev/stderr.txt"

  if [[ $code -eq 0 ]]; then
    note "ok exit 0"
  else
    note "exit $code"
  fi
  if leaked_secret "$out" || leaked_secret "$err"; then
    note "secret leaked"
  else
    note "ok no labeled secret"
  fi
  judge_stdout "$ev/stdout.txt"

  if checks_ok; then
    write_result "$ev" PASS
    write_result "$(evidence_root)" PASS
  else
    write_result "$ev" FAIL
    write_result "$(evidence_root)" FAIL
  fi

  echo "feature=$FEATURE_ID"
  echo "exit=$code"
  echo "result=$(cat "$ev/result.txt")"
  echo "evidence=$ev"
  cat "$CHECK_FILE"
  rm -f "$out" "$err"
  checks_ok
}

cmd_cleanup() {
  require_run_id
  local scratch ev
  scratch=$(scratch_dir)
  ev=$(evidence_root)
  case $scratch in
    /tmp/verify-morpheum-signing-*) ;;
    *) die "refusing to delete $scratch" ;;
  esac
  if [[ $scratch == "$REPO_ROOT"* ]]; then
    die "refusing to delete a path inside the repo"
  fi
  rm -rf "$scratch"
  echo "scratch_removed=$scratch"
  if [[ -d $ev ]]; then
    echo "evidence_preserved=$ev"
    if [[ -f $ev/result.txt ]]; then
      echo "result=$(cat "$ev/result.txt")"
    fi
  else
    echo "evidence_preserved=none"
  fi
}

main() {
  local cmd=${1:-}
  [[ -n $cmd ]] || usage
  shift
  case $cmd in
    doctor) cmd_doctor ;;
    launch) cmd_launch ;;
    drive-native-sign) cmd_drive native-sign ;;
    drive-mnemonic-native) cmd_drive mnemonic-native ;;
    drive-agent-claim) cmd_drive agent-claim ;;
    drive-agent-claim-verify) cmd_drive agent-claim-verify ;;
    cleanup) cmd_cleanup ;;
    -h|--help|help) usage ;;
    *) usage ;;
  esac
}

main "$@"

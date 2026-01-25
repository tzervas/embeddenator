#!/usr/bin/env bash
set -euo pipefail

base_dir="/home/kang/Documents/projects/embdntr"
repos=(
  embeddenator
  embeddenator-cli
  embeddenator-contract-bench
  embeddenator-fs
  embeddenator-interop
  embeddenator-io
  embeddenator-obs
  embeddenator-retrieval
  embeddenator-testkit
  embeddenator-vsa
  embeddenator-workspace
)

# time-limited run to avoid hangs on large fetches
fetch_cmd=(git fetch --all --tags --prune --jobs=4)
pull_cmd=(git pull --rebase --autostash)

# Tracking arrays for summary reporting
declare -a repos_with_conflicts=()
declare -a repos_ahead=()
declare -a repos_behind=()
declare -a orphaned_branches=()

# Helper function: detect and report merge conflicts
detect_conflicts() {
  local dir="$1"
  local repo="$2"
  
  # Check for conflict markers in status
  if git -C "${dir}" status --porcelain 2>/dev/null | grep -q '^UU\|^AA\|^DD\|^AU\|^UA\|^DU\|^UD'; then
    echo "⚠️  MERGE CONFLICT DETECTED"
    echo "   Conflicted files:"
    git -C "${dir}" diff --name-only --diff-filter=U 2>/dev/null | sed 's/^/     - /'
    echo "   Resolution strategies:"
    echo "     1. cd ${dir}"
    echo "     2. Resolve conflicts manually in files"
    echo "     3. git add <resolved-files>"
    echo "     4. git rebase --continue (or git merge --continue)"
    echo "     5. Rerun this script"
    repos_with_conflicts+=("${repo}")
    return 1
  fi
  return 0
}

# Helper function: check for orphaned upstream branches
check_orphaned_branches() {
  local dir="$1"
  local repo="$2"
  
  # Get current branch
  local current_branch
  current_branch=$(git -C "${dir}" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")
  
  if [[ -z "${current_branch}" || "${current_branch}" == "HEAD" ]]; then
    return 0
  fi
  
  # Check if upstream exists
  local upstream
  upstream=$(git -C "${dir}" rev-parse --abbrev-ref "${current_branch}@{u}" 2>/dev/null || echo "")
  
  if [[ -n "${upstream}" ]]; then
    # Parse remote and branch
    local remote_name="${upstream%%/*}"
    local remote_branch="${upstream#*/}"
    
    # Check if remote branch still exists
    if ! git -C "${dir}" ls-remote --heads "${remote_name}" "${remote_branch}" 2>/dev/null | grep -q .; then
      echo "🗑️  ORPHANED UPSTREAM: ${upstream} (remote branch deleted)"
      orphaned_branches+=("${repo}:${current_branch}:${upstream}")
      return 1
    fi
  fi
  return 0
}


for repo in "${repos[@]}"; do
  dir="${base_dir}/${repo}"
  echo "===== ${repo} ====="
  if [[ ! -d "${dir}/.git" ]]; then
    echo "(skip) not a git repo"
    continue
  fi

  # show branch and origin
  branch=$(git -C "${dir}" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
  echo "branch: ${branch}"
  echo "origin:"
  git -C "${dir}" remote get-url --all origin | sed 's/^/  - /'

  # fetch with timeout to avoid hanging
  if ! timeout 180 git -C "${dir}" "${fetch_cmd[@]:1}" >/dev/null 2>&1; then
    echo "fetch: TIMEOUT/ERROR (retry manually if needed)"
    continue
  fi

  # Check for orphaned upstream branches before pulling
  check_orphaned_branches "${dir}" "${repo}"

  # pull with timeout; if it fails, check for conflicts
  set +e
  pull_output=$(timeout 180 git -C "${dir}" "${pull_cmd[@]:1}" 2>&1)
  pull_exit=$?
  set -e
  
  if [[ ${pull_exit} -ne 0 ]]; then
    echo "pull: CONFLICT/ERROR"
    # Check if it's a merge conflict
    if ! detect_conflicts "${dir}" "${repo}"; then
      continue
    fi
    # If not a conflict, show the error
    echo "${pull_output}" | tail -5
  fi

  # Enhanced ahead/behind summary with visual indicators
  if git -C "${dir}" rev-parse --abbrev-ref @{u} >/dev/null 2>&1; then
    set +e
    read -r ahead behind <<<"$(git -C "${dir}" rev-list --left-right --count HEAD...@{u} 2>/dev/null)"
    set -e
    ahead=${ahead:-0}
    behind=${behind:-0}
    
    # Visual indicators and tracking
    if [[ ${ahead} -gt 0 ]]; then
      echo "📤 ahead: ${ahead} commits (needs push)"
      repos_ahead+=("${repo}:${ahead}")
    else
      echo "ahead: ${ahead}"
    fi
    
    if [[ ${behind} -gt 0 ]]; then
      echo "📥 behind: ${behind} commits (needs pull)"
      repos_behind+=("${repo}:${behind}")
    else
      echo "behind: ${behind}"
    fi
  else
    echo "no upstream tracking branch"
  fi

  dirty=$(git -C "${dir}" status --porcelain | wc -l | tr -d '\n')
  if [[ ${dirty} -gt 0 ]]; then
    echo "⚠️  dirty files: ${dirty}"
  else
    echo "dirty files: ${dirty}"
  fi
  echo
done

# Summary report
echo "========================================="
echo "           SYNC SUMMARY"
echo "========================================="
echo

if [[ ${#repos_with_conflicts[@]} -gt 0 ]]; then
  echo "⚠️  Repos with conflicts (${#repos_with_conflicts[@]}):"
  for repo in "${repos_with_conflicts[@]}"; do
    echo "  - ${repo}"
  done
  echo
fi

if [[ ${#repos_ahead[@]} -gt 0 ]]; then
  echo "📤 Repos ahead of remote (${#repos_ahead[@]}):"
  for entry in "${repos_ahead[@]}"; do
    repo="${entry%%:*}"
    count="${entry#*:}"
    echo "  - ${repo}: ${count} commits"
  done
  echo
fi

if [[ ${#repos_behind[@]} -gt 0 ]]; then
  echo "📥 Repos behind remote (${#repos_behind[@]}):"
  for entry in "${repos_behind[@]}"; do
    repo="${entry%%:*}"
    count="${entry#*:}"
    echo "  - ${repo}: ${count} commits"
  done
  echo
fi

if [[ ${#orphaned_branches[@]} -gt 0 ]]; then
  echo "🗑️  Orphaned upstream branches detected (${#orphaned_branches[@]}):"
  for entry in "${orphaned_branches[@]}"; do
    IFS=: read -r repo branch upstream <<<"${entry}"
    echo "  - ${repo}: branch '${branch}' tracks deleted '${upstream}'"
  done
  echo
  echo "To clean up orphaned branches, you can:"
  echo "  1. cd into the repo"
  echo "  2. git branch --unset-upstream (to remove tracking)"
  echo "  3. Or delete the local branch if no longer needed: git branch -d <branch>"
  echo
fi

if [[ ${#repos_with_conflicts[@]} -eq 0 ]] && \
   [[ ${#repos_ahead[@]} -eq 0 ]] && \
   [[ ${#repos_behind[@]} -eq 0 ]] && \
   [[ ${#orphaned_branches[@]} -eq 0 ]]; then
  echo "✅ All repos are in sync!"
fi

echo "========================================="


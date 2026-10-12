#!/usr/bin/env bash
set -euo pipefail

fail() {
  printf '::error::%s\n' "$1" >&2
  exit 1
}

validate_pr() {
  jq -e --arg repository "$GITHUB_REPOSITORY" --arg head "$2" '
    .state == "open"
    and .head.repo.full_name == $repository
    and .base.repo.full_name == $repository
    and .head.sha == $head
  ' <<< "$1" > /dev/null || fail 'PR is closed, belongs to another repository, or has a different head SHA.'
}

validate_base() {
  local candidate="$1" expected="$2" branch="$3" current="$4"
  [[ "$candidate" == "$expected" ]] && return 0
  local pulls predecessor number base ref parents fresh candidate_tree expected_tree
  pulls="$(gh api --method GET "repos/$GITHUB_REPOSITORY/pulls" \
    -f state=open -f "head=${GITHUB_REPOSITORY%%/*}:$branch" -f per_page=100)" || return 1
  predecessor="$(jq -ce --arg repository "$GITHUB_REPOSITORY" --arg head "$expected" \
    --arg branch "$branch" --arg merge "$candidate" --argjson current "$current" '
    if type != "array" or length >= 100 then empty else . end
    | [.[] | select(.state == "open" and .number != $current and .mergeable != false
      and .head.repo.full_name == $repository and .base.repo.full_name == $repository
      and .head.sha == $head and .head.ref == $branch and .merge_commit_sha == $merge)]
    | if length == 1 then .[0] else empty end
  ' <<< "$pulls")" || return 1
  number="$(jq -r '.number' <<< "$predecessor")"
  base="$(jq -r '.base.sha' <<< "$predecessor")"
  ref="$(jq -r '.base.ref' <<< "$predecessor")"
  [[ "$number" =~ ^[1-9][0-9]*$ && "$base" =~ ^[0-9a-f]{40}$ ]] || return 1
  parents="$(git show -s --format=%P "$candidate")" || return 1
  [[ "$parents" == "$base $expected" ]] || return 1
  candidate_tree="$(git rev-parse "$candidate^{tree}")" || return 1
  expected_tree="$(git rev-parse "$expected^{tree}")" || return 1
  [[ "$candidate_tree" == "$expected_tree" ]] || return 1
  fresh="$(gh api --method GET "repos/$GITHUB_REPOSITORY/pulls/$number")" || return 1
  jq -e --arg repository "$GITHUB_REPOSITORY" --arg head "$expected" --arg branch "$branch" \
    --arg merge "$candidate" --arg base "$base" --arg ref "$ref" --argjson number "$number" '
    .number == $number and .state == "open" and .mergeable != false
    and .head.repo.full_name == $repository and .base.repo.full_name == $repository
    and .head.sha == $head and .head.ref == $branch
    and .base.sha == $base and .base.ref == $ref and .merge_commit_sha == $merge
  ' <<< "$fresh" > /dev/null
}

main() {
  local pull_request="${PULL_REQUEST:-}"
  local expected_head="${EXPECTED_HEAD_SHA:-}"
  if [[ -z "$pull_request" && -z "$expected_head" ]]; then
    return
  fi
  [[ "$pull_request" =~ ^[1-9][0-9]*$ ]] || fail 'pull_request must be a positive PR number.'
  [[ "$expected_head" =~ ^[0-9a-fA-F]{40}$ ]] || fail 'expected_head_sha must be the full PR head SHA.'
  expected_head="${expected_head,,}"
  [[ "$GITHUB_SHA" == "$expected_head" ]] || fail 'Stale dispatch: this workflow run does not belong to the expected PR head.'
  [[ "$(git rev-parse HEAD)" == "$expected_head" ]] || fail 'Initial checkout does not match the expected PR head.'

  local endpoint="repos/$GITHUB_REPOSITORY/pulls/$pull_request"
  local before base_sha base_ref authorization merge_sha parents after attempt
  local first_parent second_parent extra_parent
  authorization="$(printf 'x-access-token:%s' "$GH_TOKEN" | base64 -w 0)"
  printf '::add-mask::%s\n' "$authorization"
  for attempt in 1 2 3 4 5 6; do
    before="$(gh api --method GET "$endpoint")"
    validate_pr "$before" "$expected_head"
    [[ "$(jq -r '.mergeable' <<< "$before")" != 'false' ]] \
      || fail 'PR has merge conflicts; resolve them before validation.'
    base_sha="$(jq -r '.base.sha' <<< "$before")"
    base_ref="$(jq -r '.base.ref' <<< "$before")"
    [[ "$base_sha" =~ ^[0-9a-f]{40}$ ]] || fail 'PR metadata did not contain a valid base SHA.'

    if GIT_CONFIG_COUNT=1 \
      GIT_CONFIG_KEY_0="http.$GITHUB_SERVER_URL/.extraheader" \
      GIT_CONFIG_VALUE_0="AUTHORIZATION: basic $authorization" \
      git fetch --no-tags --no-recurse-submodules --depth=3 origin \
        "+refs/pull/$pull_request/merge:refs/validation/merge"; then
      merge_sha="$(git rev-parse --verify 'refs/validation/merge^{commit}')"
      parents="$(git show -s --format=%P "$merge_sha")"
      read -r first_parent second_parent extra_parent <<< "$parents"
      if [[ "$second_parent" == "$expected_head" && -z "$extra_parent" ]] \
        && validate_base "$first_parent" "$base_sha" "$base_ref" "$pull_request"; then
        after="$(gh api --method GET "$endpoint")"
        validate_pr "$after" "$expected_head"
        if [[ "$(jq -r '.base.sha' <<< "$after")" == "$base_sha" \
          && "$(jq -r '.base.ref' <<< "$after")" == "$base_ref" \
          && "$(jq -r '.merge_commit_sha' <<< "$after")" == "$merge_sha" ]]; then
          break
        fi
      fi
      printf 'Merge %s has parents [%s]; expected head %s and verified base %s.\n' \
        "$merge_sha" "$parents" "$expected_head" "$base_sha"
    fi
    [[ "$attempt" -lt 6 ]] \
      || fail 'PR merge ref has not caught up with the expected head and base; retry validation.'
    printf 'Waiting for the PR merge ref to match its current head and base.\n'
    sleep 2
  done
  unset authorization

  git checkout --detach "$merge_sha"
  printf 'Validating PR #%s: head %s merged into base %s (%s).\n' \
    "$pull_request" "$expected_head" "$base_sha" "$merge_sha"
}

main

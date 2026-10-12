import argparse
import base64
import difflib
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys
import time
from urllib.error import HTTPError
from urllib.request import Request, urlopen


MANIFEST = "scripts/generated-svgs.txt"
MAX_FILE_BYTES = 10 * 1024 * 1024
MAX_BUNDLE_BYTES = 50 * 1024 * 1024
COMMIT_TITLE = "chore: regenerate SVG examples"
SOURCE_TRAILER = "Planorama-SVG-Source: "
MSRV_PATHS = {
    "Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "Dockerfile",
    ".dockerignore", "compose.yaml", ".github/workflows/cargo_msrv.yml",
    ".github/scripts/checkout-validation.sh",
}


class RefUpdateError(RuntimeError):
    pass


class GitHub:
    def __init__(self, repository, token):
        if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository):
            raise ValueError("Invalid repository")
        self.repository = repository
        self.token = token

    def request(self, method, path, data=None):
        url = ("https://api.github.com/graphql" if path == "graphql" else
               f"https://api.github.com/repos/{self.repository}/{path}")
        request = Request(
            url,
            data=None if data is None else json.dumps(data).encode(),
            method=method,
            headers={
                "Authorization": f"Bearer {self.token}",
                "Accept": "application/vnd.github+json",
                "Content-Type": "application/json",
                "X-GitHub-Api-Version": "2026-03-10",
                "User-Agent": "planorama-svg-examples",
            },
        )
        with urlopen(request, timeout=60) as response:
            body = response.read()
            return json.loads(body) if body else None

    def pull(self, number):
        return self.request("GET", f"pulls/{number}")

    def update_ref(self, repository_id, branch, before_sha, after_sha):
        result = self.request("POST", "graphql", {
            "query": "mutation($input: UpdateRefsInput!) { updateRefs(input: $input) { clientMutationId } }",
            "variables": {"input": {
                "repositoryId": repository_id,
                "refUpdates": [{"name": f"refs/heads/{branch}", "beforeOid": before_sha,
                                "afterOid": after_sha, "force": False}],
            }},
        })
        if result.get("errors") or result.get("data", {}).get("updateRefs") is None:
            raise RefUpdateError("Branch update rejected; its head or write permissions may have changed")


def output(**values):
    with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as stream:
        for key, value in values.items():
            text = str(value).lower() if isinstance(value, bool) else str(value)
            if "\n" in text or "\r" in text:
                raise ValueError("Multiline workflow output")
            stream.write(f"{key}={text}\n")


def summary(text):
    print(text)
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as stream:
            stream.write(text + "\n\n")


def source_sha(value):
    if not isinstance(value, str) or not re.fullmatch(r"[0-9a-f]{40}", value):
        raise ValueError("Expected a complete source commit SHA")
    return value


def pull_number(value):
    if not re.fullmatch(r"[1-9][0-9]*", str(value)):
        raise ValueError("Expected a positive pull request number")
    return int(value)


def manifest_paths(text):
    paths = text.splitlines()
    if not paths or len(paths) > 200 or len(paths) != len(set(paths)):
        raise ValueError("Generated SVG manifest is empty, oversized or duplicated")
    for path in paths:
        if (not re.fullmatch(r"examples/[A-Za-z0-9_./-]+\.svg", path)
                or any(part in ("", ".", "..") for part in path.split("/"))
                or str(PurePosixPath(path)) != path):
            raise ValueError(f"Invalid generated SVG path: {path!r}")
    return paths


def current_pull(pr, repository, sha):
    return (pr["state"] == "open"
            and pr["head"]["repo"] is not None
            and pr["head"]["repo"]["full_name"] == repository
            and pr["head"]["sha"] == sha)


def prepare(api, event, event_name, manual_number):
    number = pull_number(event["number"] if event_name == "pull_request" else manual_number)
    pr = api.pull(number)
    if (pr["state"] != "open" or pr["head"]["repo"] is None
            or (event_name == "pull_request"
                and pr["head"]["sha"] != event["pull_request"]["head"]["sha"])):
        summary("PR is closed or this source revision was superseded; nothing to generate.")
        return {"active": False}
    sha = source_sha(pr["head"]["sha"])
    same_repository = pr["head"]["repo"]["full_name"] == api.repository
    summary(f"## SVG generation for PR #{number}\n\nSource: `{sha}`")
    if not same_repository:
        summary("Fork PR: download the generated SVG artifact and apply `updates.patch` on the source branch.")
    return {
        "active": True, "pull_request": number, "head_sha": sha,
        "head_repo": pr["head"]["repo"]["full_name"],
        "same_repository": same_repository,
    }


def read_output(root, relative):
    path = root / relative
    if any((root / Path(*Path(relative).parts[:index])).is_symlink()
           for index in range(1, len(Path(relative).parts) + 1)):
        raise ValueError(f"Generated output must not be a symlink: {relative}")
    if not path.is_file() or path.stat().st_size > MAX_FILE_BYTES:
        raise ValueError(f"Missing or oversized generated output: {relative}")
    return path.read_bytes().decode("utf-8")


def file_patch(path, before, after):
    header = f"diff --git a/{path} b/{path}\n"
    if before is None:
        header += "new file mode 100644\n"
    diff = difflib.unified_diff(
        [] if before is None else before.splitlines(keepends=True),
        after.splitlines(keepends=True),
        fromfile="/dev/null" if before is None else f"a/{path}", tofile=f"b/{path}",
    )
    return header + "".join(
        line if line.endswith("\n") else line + "\n\\ No newline at end of file\n"
        for line in diff
    )


def package(root, destination, sha, number):
    root = root.resolve()
    actual = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip()
    if actual != sha:
        raise ValueError("Generation checkout no longer matches the captured source SHA")
    paths = manifest_paths((root / MANIFEST).read_text(encoding="utf-8"))
    files, patches, generated = [], [], {}
    for path in paths:
        content = read_output(root, path)
        generated[path] = content
        previous = subprocess.run(
            ["git", "-C", str(root), "show", f"{sha}:{path}"],
            stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, check=False,
        )
        before = previous.stdout.decode("utf-8") if previous.returncode == 0 else None
        if before != content:
            files.append({"path": path, "content": content})
            patches.append(file_patch(path, before, content))
    bundle = {"schema": 1, "source_sha": sha, "pull_request": number, "files": files}
    encoded = json.dumps(bundle, ensure_ascii=False).encode("utf-8")
    if len(encoded) > MAX_BUNDLE_BYTES:
        raise ValueError("Generated SVG bundle is too large")
    destination.mkdir(parents=True, exist_ok=True)
    (destination / "updates.json").write_bytes(encoded)
    (destination / "updates.patch").write_text("".join(patches), encoding="utf-8")
    for path, content in generated.items():
        target = destination / "svgs" / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content, encoding="utf-8")
    summary(f"Generated {len(paths)} SVGs; {len(files)} changed.\n\n"
            + "\n".join(f"- `{entry['path']}`" for entry in files))


def validate_bundle(bundle, allowed, sha, number):
    if (not isinstance(bundle, dict) or bundle.get("schema") != 1 or bundle.get("source_sha") != sha
            or bundle.get("pull_request") != number or not isinstance(bundle.get("files"), list)):
        raise ValueError("Artifact does not match this PR source revision")
    seen = set()
    for entry in bundle["files"]:
        if not isinstance(entry, dict):
            raise ValueError("Invalid artifact entry")
        path, content = entry.get("path"), entry.get("content")
        if not isinstance(path, str) or path not in allowed or path in seen or not isinstance(content, str):
            raise ValueError("Artifact contains an undeclared, duplicated or invalid output")
        if len(content.encode("utf-8")) > MAX_FILE_BYTES:
            raise ValueError("Artifact contains an oversized output")
        seen.add(path)
    return bundle["files"]


def validation_workflows(api, number):
    workflows = ["cargo_fmt.yml", "cargo_clippy.yml", "cargo_test.yml"]
    for page in range(1, 31):
        files = api.request("GET", f"pulls/{number}/files?per_page=100&page={page}")
        if any(file["filename"] in MSRV_PATHS or file.get("previous_filename") in MSRV_PATHS for file in files):
            return workflows + ["cargo_msrv.yml"]
        if len(files) < 100:
            return workflows
    return workflows + ["cargo_msrv.yml"]


def wait_for_published_head(api, number, previous_sha, generated_sha):
    for attempt in range(6):
        pr = api.pull(number)
        if current_pull(pr, api.repository, generated_sha):
            return True
        if not current_pull(pr, api.repository, previous_sha):
            summary("The generated commit was superseded; newer source runs take precedence.")
            return False
        if attempt < 5:
            time.sleep(2)
    raise RuntimeError("GitHub has not exposed the generated PR head yet. Retry Update SVG examples for this PR.")


def dispatch_checks(api, number, sha, branch):
    runs, errors = [], []
    for workflow in validation_workflows(api, number):
        if not current_pull(api.pull(number), api.repository, sha):
            summary("The PR advanced before validation dispatch completed; newer source runs take precedence.")
            return runs
        try:
            response = api.request("POST", f"actions/workflows/{workflow}/dispatches", {
                "ref": branch,
                "inputs": {"pull_request": str(number), "expected_head_sha": sha},
            })
            run_id = response.get("workflow_run_id") if isinstance(response, dict) else None
            if not isinstance(run_id, int):
                raise ValueError(f"{workflow}: dispatch did not return a workflow run ID")
            for attempt in range(3):
                try:
                    run = api.request("GET", f"actions/runs/{run_id}")
                    break
                except HTTPError as error:
                    if error.code != 404 or attempt == 2:
                        raise
                    time.sleep(attempt + 1)
            if (run["head_sha"] != sha or run["event"] != "workflow_dispatch"
                    or run["path"].split("@", 1)[0] != f".github/workflows/{workflow}"
                    or run["head_branch"] != branch):
                raise ValueError(f"{workflow}: dispatched run does not match the expected PR commit")
            runs.append(run["html_url"])
            summary(f"Requested [{workflow}]({run['html_url']}) for `{sha}`; see the run for its validation result.")
        except (HTTPError, ValueError) as error:
            errors.append(f"{workflow}: {error}")
    if errors:
        raise RuntimeError("Some validation runs could not be verified. Retry Update SVG examples for this PR. "
                           + "; ".join(errors))
    return runs


def publish(api, bundle, sha, number):
    pr = api.pull(number)
    if not current_pull(pr, api.repository, sha):
        summary("PR is read-only, closed or superseded; no automatic commit. The SVG patch remains in the artifact.")
        return
    manifest = api.request("GET", f"contents/{MANIFEST}?ref={sha}")
    allowed = manifest_paths(base64.b64decode(manifest["content"]).decode("utf-8"))
    files = validate_bundle(bundle, allowed, sha, number)
    commit = api.request("GET", f"git/commits/{sha}")
    updated_sha = sha
    changed = False
    if files:
        try:
            tree = api.request("POST", "git/trees", {
                "base_tree": commit["tree"]["sha"],
                "tree": [{"path": entry["path"], "mode": "100644", "type": "blob",
                          "content": entry["content"]} for entry in files],
            })
            if tree["sha"] != commit["tree"]["sha"]:
                generated = api.request("POST", "git/commits", {
                    "message": f"{COMMIT_TITLE}\n\n{SOURCE_TRAILER}{sha}",
                    "tree": tree["sha"], "parents": [sha],
                    "author": {"name": "github-actions[bot]", "email": "41898282+github-actions[bot]@users.noreply.github.com"},
                })
                if not current_pull(api.pull(number), api.repository, sha):
                    summary("The PR advanced during generation; discarded stale SVG output.")
                    return
                updated_sha = generated["sha"]
                api.update_ref(pr["base"]["repo"]["node_id"], pr["head"]["ref"], sha, updated_sha)
                changed = True
        except RefUpdateError:
            summary("The branch changed or rejected writeback; discarded the generated commit. "
                    "No branch was overwritten. The SVG patch remains in the artifact.")
            return
        except HTTPError as error:
            if error.code not in (403, 409, 422):
                raise
            summary(f"Automatic writeback was unavailable (HTTP {error.code}); no source changes were overwritten. "
                    "Download this run's artifact and apply `updates.patch` on the matching source revision.")
            return
    if changed:
        summary(f"## SVG updates committed\n\nSource: `{sha}`\n\nGenerated commit: "
                f"[{updated_sha}](https://github.com/{api.repository}/commit/{updated_sha})\n\n"
                + "\n".join(f"- `{entry['path']}`" for entry in files))
        if not wait_for_published_head(api, number, sha, updated_sha):
            return
    else:
        summary(f"SVGs are up to date at `{sha}`; no commit created.")
    if changed or (commit["message"].startswith(COMMIT_TITLE + "\n")
                   and SOURCE_TRAILER in commit["message"]):
        dispatch_checks(api, number, updated_sha, pr["head"]["ref"])


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=["prepare", "package", "publish"])
    parser.add_argument("paths", nargs="*")
    args = parser.parse_args()
    if args.command == "package":
        package(Path(args.paths[0]), Path(args.paths[1]), source_sha(os.environ["SVG_SOURCE_SHA"]),
                pull_number(os.environ["SVG_PULL_REQUEST"]))
        return
    api = GitHub(os.environ["GITHUB_REPOSITORY"], os.environ["GH_TOKEN"])
    if args.command == "prepare":
        event = json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_text(encoding="utf-8"))
        output(**prepare(api, event, os.environ["GITHUB_EVENT_NAME"], os.environ.get("SVG_PULL_REQUEST", "")))
    else:
        path = Path(args.paths[0])
        if path.stat().st_size > MAX_BUNDLE_BYTES:
            raise ValueError("Generated SVG bundle is too large")
        publish(api, json.loads(path.read_text(encoding="utf-8")), source_sha(os.environ["SVG_SOURCE_SHA"]),
                pull_number(os.environ["SVG_PULL_REQUEST"]))


if __name__ == "__main__":
    try:
        main()
    except (HTTPError, OSError, RuntimeError, ValueError) as error:
        print(f"SVG automation failed: {error}", file=sys.stderr)
        sys.exit(1)

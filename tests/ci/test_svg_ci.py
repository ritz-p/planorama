import base64
import importlib.util
import json
from pathlib import Path
from subprocess import CompletedProcess
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import Mock, patch


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("svg_ci", ROOT / ".github/scripts/svg_ci.py")
svg_ci = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(svg_ci)

REPOSITORY = "ritz-p/planorama"
REPOSITORY_ID = "R_test_repository"
NUMBER = 17
BRANCH = "feature/svg-refresh"
SOURCE = "1" * 40
NEWER = "2" * 40
GENERATED = "3" * 40
TREE = "a" * 40
UPDATED_TREE = "b" * 40
SVG = "examples/fixture/diagram.svg"
SECOND_SVG = "examples/fixture/planorama.svg"
WORKFLOWS = ("cargo_fmt.yml", "cargo_clippy.yml", "cargo_test.yml", "cargo_msrv.yml")


def pull(sha=SOURCE, state="open", repository=REPOSITORY):
    return {
        "number": NUMBER,
        "state": state,
        "head": {
            "sha": sha,
            "ref": BRANCH,
            "repo": {
                "full_name": repository,
                "node_id": REPOSITORY_ID if repository == REPOSITORY else "R_fork_repository",
            },
        },
        "base": {"repo": {"full_name": REPOSITORY, "node_id": REPOSITORY_ID}},
    }


def bundle(files=None, sha=SOURCE, number=NUMBER):
    return {
        "schema": 1,
        "source_sha": sha,
        "pull_request": number,
        "files": [] if files is None else files,
    }


def changed_file():
    return {"path": SVG, "content": "<svg>updated</svg>\n"}


class SvgAutomationTests(unittest.TestCase):
    def setUp(self):
        quiet = patch.object(svg_ci, "summary")
        self.summary = quiet.start()
        self.addCleanup(quiet.stop)

    def github(self, message="Update source"):
        api = Mock(spec=svg_ci.GitHub)
        api.repository = REPOSITORY
        api.pull.return_value = pull()
        api.update_ref.return_value = None
        responses = {
            ("GET", f"contents/{svg_ci.MANIFEST}?ref={SOURCE}"): {
                "content": base64.b64encode(f"{SVG}\n".encode()).decode(),
            },
            ("GET", f"git/commits/{SOURCE}"): {"tree": {"sha": TREE}, "message": message},
            ("POST", "git/trees"): {"sha": UPDATED_TREE},
            ("POST", "git/commits"): {"sha": GENERATED},
            ("GET", f"pulls/{NUMBER}/files?per_page=100&page=1"): [],
        }
        for run_id, workflow in enumerate(WORKFLOWS, 101):
            responses[("POST", f"actions/workflows/{workflow}/dispatches")] = {
                "workflow_run_id": run_id,
            }
            responses[("GET", f"actions/runs/{run_id}")] = {
                "head_sha": SOURCE,
                "head_branch": BRANCH,
                "event": "workflow_dispatch",
                "path": f".github/workflows/{workflow}",
                "html_url": f"https://github.com/{REPOSITORY}/actions/runs/{run_id}",
            }

        def request(method, path, data=None):
            self.assertIn((method, path), responses, f"Unexpected API request: {method} {path}")
            return responses[(method, path)]

        api.request.side_effect = request
        return api, responses

    def requests(self, api, method, path=None):
        return [
            call for call in api.request.call_args_list
            if call.args[0] == method and (path is None or call.args[1] == path)
        ]

    def dispatches(self, api):
        return [
            call for call in self.requests(api, "POST")
            if call.args[1].startswith("actions/workflows/")
        ]

    def test_publish_rejects_artifacts_before_creating_git_objects(self):
        invalid = {
            "unlisted SVG": bundle([{"path": "examples/unlisted.svg", "content": "<svg/>"}]),
            "source file": bundle([{"path": "src/main.rs", "content": "fn main() {}"}]),
            "duplicate": bundle([changed_file(), changed_file()]),
            "source revision": bundle([changed_file()], sha=NEWER),
            "pull request": bundle([changed_file()], number=NUMBER + 1),
        }
        for label, artifact in invalid.items():
            with self.subTest(label=label):
                api, _ = self.github()
                with self.assertRaises(ValueError):
                    svg_ci.publish(api, artifact, SOURCE, NUMBER)
                self.assertEqual(self.requests(api, "POST"), [])
                api.update_ref.assert_not_called()

    def test_noop_never_creates_a_commit_or_updates_the_branch(self):
        for files in ([], [changed_file()]):
            with self.subTest(empty_bundle=not files):
                api, responses = self.github()
                responses[("POST", "git/trees")] = {"sha": TREE}
                svg_ci.publish(api, bundle(files), SOURCE, NUMBER)
                self.assertEqual(self.requests(api, "POST", "git/commits"), [])
                api.update_ref.assert_not_called()
                self.assertEqual(self.dispatches(api), [])

    def test_superseded_event_does_not_start_generation(self):
        api, _ = self.github()
        api.pull.return_value = pull(NEWER)
        event = {"number": NUMBER, "pull_request": {"head": {"sha": SOURCE}}}
        result = svg_ci.prepare(api, event, "pull_request", "")
        self.assertEqual(result, {"active": False})
        api.request.assert_not_called()
        api.update_ref.assert_not_called()

    def test_stale_closed_and_fork_prs_cannot_publish(self):
        for pr in (pull(NEWER), pull(state="closed"), pull(repository="contributor/planorama")):
            with self.subTest(head=pr["head"], state=pr["state"]):
                api, _ = self.github()
                api.pull.return_value = pr
                svg_ci.publish(api, bundle([changed_file()]), SOURCE, NUMBER)
                api.request.assert_not_called()
                api.update_ref.assert_not_called()

    def test_head_change_during_generation_does_not_update_the_branch(self):
        api, _ = self.github()
        api.pull.side_effect = [pull(), pull(NEWER)]
        svg_ci.publish(api, bundle([changed_file()]), SOURCE, NUMBER)
        commits = self.requests(api, "POST", "git/commits")
        self.assertEqual(len(commits), 1)
        self.assertEqual(commits[0].args[2]["parents"], [SOURCE])
        api.update_ref.assert_not_called()
        self.assertEqual(self.dispatches(api), [])

    def test_atomic_ref_conflict_does_not_retry_or_dispatch_stale_checks(self):
        api, _ = self.github()
        api.update_ref.side_effect = svg_ci.RefUpdateError("Head advanced after the last PR read")
        svg_ci.publish(api, bundle([changed_file()]), SOURCE, NUMBER)
        api.update_ref.assert_called_once_with(REPOSITORY_ID, BRANCH, SOURCE, GENERATED)
        self.assertEqual(self.requests(api, "PATCH"), [])
        self.assertEqual(len(self.requests(api, "POST", "git/commits")), 1)
        self.assertEqual(self.dispatches(api), [])

    def test_successful_writeback_updates_only_declared_svgs_and_validates_the_new_commit(self):
        api, responses = self.github()

        def update_ref(repository_id, branch, before_sha, after_sha):
            api.pull.return_value = pull(after_sha)

        api.update_ref.side_effect = update_ref
        for run_id in range(101, 105):
            responses[("GET", f"actions/runs/{run_id}")]["head_sha"] = GENERATED
        with patch.object(svg_ci, "wait_for_published_head", return_value=True) as visible:
            svg_ci.publish(api, bundle([changed_file()]), SOURCE, NUMBER)
        trees = self.requests(api, "POST", "git/trees")
        self.assertEqual(len(trees), 1)
        self.assertEqual(trees[0].args[2], {
            "base_tree": TREE,
            "tree": [{"path": SVG, "mode": "100644", "type": "blob",
                      "content": changed_file()["content"]}],
        })
        commits = self.requests(api, "POST", "git/commits")
        self.assertEqual(len(commits), 1)
        self.assertEqual(commits[0].args[2]["parents"], [SOURCE])
        api.update_ref.assert_called_once_with(REPOSITORY_ID, BRANCH, SOURCE, GENERATED)
        visible.assert_called_once_with(api, NUMBER, SOURCE, GENERATED)
        dispatched = self.dispatches(api)
        self.assertEqual(len(dispatched), 3)
        for call in dispatched:
            self.assertEqual(call.args[2]["inputs"], {
                "pull_request": str(NUMBER), "expected_head_sha": GENERATED,
            })

    def test_update_ref_compares_the_old_oid_and_never_forces(self):
        for conflict in (False, True):
            with self.subTest(conflict=conflict):
                api = svg_ci.GitHub(REPOSITORY, "test-token")
                result = ({"errors": [{"message": "beforeOid does not match"}], "data": {"updateRefs": None}}
                          if conflict else {"data": {"updateRefs": {"clientMutationId": None}}})
                with patch.object(svg_ci, "urlopen") as opened:
                    opened.return_value.__enter__.return_value.read.return_value = json.dumps(result).encode()
                    if conflict:
                        with self.assertRaises(svg_ci.RefUpdateError):
                            api.update_ref(REPOSITORY_ID, BRANCH, SOURCE, GENERATED)
                    else:
                        api.update_ref(REPOSITORY_ID, BRANCH, SOURCE, GENERATED)
                self.assertEqual(opened.call_count, 1)
                request = opened.call_args.args[0]
                self.assertEqual(request.full_url, "https://api.github.com/graphql")
                self.assertEqual(request.get_method(), "POST")
                payload = json.loads(request.data)
                self.assertEqual(payload["variables"]["input"], {
                    "repositoryId": REPOSITORY_ID,
                    "refUpdates": [{"name": f"refs/heads/{BRANCH}", "beforeOid": SOURCE,
                                    "afterOid": GENERATED, "force": False}],
                })

    def test_incomplete_or_superseded_generation_packages_nothing(self):
        for checkout in (SOURCE, NEWER):
            with self.subTest(checkout=checkout), TemporaryDirectory() as directory:
                root = Path(directory) / "source"
                manifest = root / svg_ci.MANIFEST
                manifest.parent.mkdir(parents=True)
                manifest.write_text(f"{SVG}\n{SECOND_SVG}\n", encoding="utf-8")
                generated = root / SVG
                generated.parent.mkdir(parents=True)
                generated.write_text("<svg>updated</svg>\n", encoding="utf-8")
                destination = Path(directory) / "artifact"
                previous = CompletedProcess([], 0, stdout=b"<svg>previous</svg>\n")
                with patch.object(svg_ci.subprocess, "check_output", return_value=checkout + "\n"), \
                        patch.object(svg_ci.subprocess, "run", return_value=previous):
                    with self.assertRaises(ValueError):
                        svg_ci.package(root, destination, SOURCE, NUMBER)
                self.assertFalse(destination.exists())

    def test_dispatch_rejects_a_run_for_a_different_source_sha(self):
        api, responses = self.github()
        responses[("GET", "actions/runs/101")]["head_sha"] = NEWER
        with self.assertRaises(RuntimeError):
            svg_ci.dispatch_checks(api, NUMBER, SOURCE, BRANCH)
        for call in self.dispatches(api):
            self.assertEqual(call.args[2], {
                "ref": BRANCH,
                "inputs": {"pull_request": str(NUMBER), "expected_head_sha": SOURCE},
            })

    def test_dispatch_includes_msrv_from_later_pages_of_the_whole_pr(self):
        for file in ({"filename": "Cargo.toml"},
                     {"filename": "Cargo.previous", "previous_filename": "Cargo.toml"}):
            with self.subTest(file=file):
                api, responses = self.github()
                responses[("GET", f"pulls/{NUMBER}/files?per_page=100&page=1")] = [
                    {"filename": f"examples/fixture-{index}/diagram.svg"} for index in range(100)
                ]
                responses[("GET", f"pulls/{NUMBER}/files?per_page=100&page=2")] = [file]
                runs = svg_ci.dispatch_checks(api, NUMBER, SOURCE, BRANCH)
                self.assertEqual(len(runs), 4)
                self.assertEqual({call.args[1] for call in self.dispatches(api)}, {
                    f"actions/workflows/{workflow}/dispatches" for workflow in WORKFLOWS
                })
                self.assertEqual(len(self.requests(api, "GET", f"pulls/{NUMBER}/files?per_page=100&page=2")), 1)

    def test_unchanged_generated_commit_still_repairs_validation_dispatch(self):
        message = f"{svg_ci.COMMIT_TITLE}\n\n{svg_ci.SOURCE_TRAILER}{NEWER}"
        api, _ = self.github(message=message)
        svg_ci.publish(api, bundle(), SOURCE, NUMBER)
        self.assertEqual(self.requests(api, "POST", "git/trees"), [])
        self.assertEqual(self.requests(api, "POST", "git/commits"), [])
        api.update_ref.assert_not_called()
        dispatched = self.dispatches(api)
        self.assertEqual(len(dispatched), 3)
        self.assertEqual({call.args[1] for call in dispatched}, {
            f"actions/workflows/{workflow}/dispatches" for workflow in WORKFLOWS[:3]
        })
        for call in dispatched:
            self.assertEqual(call.args[2]["inputs"], {
                "pull_request": str(NUMBER), "expected_head_sha": SOURCE,
            })


if __name__ == "__main__":
    unittest.main()

import argparse
from contextlib import redirect_stdout
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import bench_night as night
import bench_scheduler as scheduler


class SchedulerTests(unittest.TestCase):
    def test_cpu_ranges_deduplicate_and_sort(self):
        self.assertEqual(scheduler.parse_cpu_ids("8-11,2,9"), [2, 8, 9, 10, 11])

    def test_cpu_ranges_reject_bad_input(self):
        for value in ("", "-1", "3-1", "a", "1,", "1-2-3", "0-99999999", "１"):
            with self.subTest(value=value), self.assertRaises(ValueError):
                scheduler.parse_cpu_ids(value)

    def test_mac_discovers_named_p_class_instead_of_all_cpus_or_level_zero(self):
        values = {"hw.nperflevels": "2", "hw.perflevel0.name": "Efficiency",
                  "hw.perflevel1.name": "Performance", "hw.perflevel1.physicalcpu": "8"}
        with patch.object(sys, "platform", "darwin"), patch.object(scheduler, "sysctl", side_effect=values.__getitem__):
            policy = scheduler.policy_for_host()
        self.assertEqual(policy["worker_limit"], 8)
        self.assertFalse(policy["hard_affinity"])
        self.assertIsNone(policy["cpu_ids"])

    def test_mac_does_not_guess_when_topology_is_unavailable(self):
        with patch.object(sys, "platform", "darwin"), patch.object(scheduler, "sysctl", side_effect=OSError):
            with self.assertRaisesRegex(ValueError, "refusing an all-core fallback"):
                scheduler.policy_for_host()

    def test_linux_requires_explicit_cpu_selection_and_respects_current_cpuset(self):
        with patch.object(sys, "platform", "linux"), patch.object(os, "sched_getaffinity", return_value={2, 3, 6, 7}, create=True):
            with self.assertRaisesRegex(ValueError, "requires --cpus"):
                scheduler.policy_for_host()
            with self.assertRaisesRegex(ValueError, "outside"):
                scheduler.policy_for_host("0-3")
            policy = scheduler.policy_for_host("2-3,6")
        self.assertEqual(policy["cpu_ids"], [2, 3, 6])
        self.assertEqual(policy["worker_limit"], 3)
        self.assertTrue(policy["hard_affinity"])
        self.assertIn("nice -10", policy["priority"])

    def test_linux_affinity_is_set_and_read_back(self):
        with patch.object(os, "sched_getaffinity", side_effect=[{0, 1, 2}, {0, 2}], create=True), patch.object(os, "sched_setaffinity", create=True) as setter:
            scheduler.pin_worker([0, 2])
        setter.assert_called_once_with(0, {0, 2})

    def test_linux_affinity_silent_kernel_restriction_is_refused(self):
        with patch.object(os, "sched_getaffinity", side_effect=[{0, 1}, {0}], create=True), patch.object(os, "sched_setaffinity", create=True):
            with self.assertRaisesRegex(ValueError, "readback differs"):
                scheduler.pin_worker([0, 1])

    def test_linux_affinity_permission_failure_is_not_ignored(self):
        with patch.object(os, "sched_getaffinity", return_value={0}, create=True), patch.object(os, "sched_setaffinity", side_effect=PermissionError, create=True):
            with self.assertRaises(PermissionError):
                scheduler.pin_worker([0])

    @unittest.skipUnless(sys.platform == "linux", "real Linux affinity inheritance")
    def test_linux_child_inherits_affinity_through_exec(self):
        cpu = min(os.sched_getaffinity(0))
        command = scheduler.worker_command([sys.executable, "-c", "import os,json; print(json.dumps(sorted(os.sched_getaffinity(0))))"], [cpu])
        result = subprocess.check_output(command, text=True)
        self.assertEqual(json.loads(result), [cpu])


class NightTests(unittest.TestCase):
    def test_latency_measurements_default_to_one_worker(self):
        self.assertEqual(night.parser().parse_args(["/fresh/out"]).jobs, 1)
        self.assertIsNone(night.parser().parse_args(["/fresh/out", "--jobs", "auto"]).jobs)

    def test_jobs_are_positive_or_auto(self):
        self.assertIsNone(night.job_count("auto"))
        self.assertEqual(night.job_count("8"), 8)
        for value in ("0", "-1", "eight", "1.5"):
            with self.assertRaises(argparse.ArgumentTypeError):
                night.job_count(value)

    def test_full_roster_and_data_paths_are_independent(self):
        jobs = night.lanes(Path("/bin/bench"), Path("/corpus with spaces"), Path("/out"), True)
        self.assertEqual(len(jobs), 13)
        self.assertEqual(len({job.name for job in jobs}), 13)
        data_paths = [job.command[job.command.index("--dir") + 1] for job in jobs if "--dir" in job.command]
        self.assertEqual(len(data_paths), len(set(data_paths)))
        read = next(job for job in jobs if job.name == "reads")
        self.assertIn("/corpus with spaces/reads", read.command)
        self.assertEqual(read.command[read.command.index("--read-batch") + 1], "1")

    def test_worker_pool_overlaps_jobs_without_exceeding_limit(self):
        with tempfile.TemporaryDirectory() as directory, redirect_stdout(io.StringIO()):
            out = Path(directory)
            records = {}
            peak = [0]
            def persist():
                peak[0] = max(peak[0], sum(row["status"] == "RUNNING" for row in records.values()))
            jobs = [night.Job(str(i), [sys.executable, "-c", "import time; time.sleep(0.15)"]) for i in range(5)]
            self.assertTrue(night.run_jobs(jobs, 2, out, os.environ, records, persist))
            self.assertEqual(peak[0], 2)
            self.assertEqual(len(records), 5)

    def test_default_worker_finishes_each_lane_before_starting_the_next(self):
        with tempfile.TemporaryDirectory() as directory, redirect_stdout(io.StringIO()):
            out = Path(directory)
            records = {}
            workers = night.parser().parse_args([str(out)]).jobs
            peak = [0]
            def persist():
                peak[0] = max(peak[0], sum(row["status"] == "RUNNING" for row in records.values()))
            jobs = [night.Job(str(i), [sys.executable, "-c", "import time; time.sleep(0.02)"]) for i in range(3)]
            self.assertTrue(night.run_jobs(jobs, workers, out, os.environ, records, persist))
            self.assertEqual(peak[0], 1)
            for i in range(1, 3):
                self.assertLessEqual(records[str(i-1)]["finished"], records[str(i)]["started"])

    def test_failures_and_missing_reports_do_not_hide_successful_other_jobs(self):
        with tempfile.TemporaryDirectory() as directory, redirect_stdout(io.StringIO()):
            out = Path(directory)
            records = {}
            jobs = [night.Job("bad", [sys.executable, "-c", "raise SystemExit(7)"]),
                    night.Job("missing", [sys.executable, "-c", "pass"], out / "missing.json"),
                    night.Job("good", [sys.executable, "-c", "pass"])]
            self.assertFalse(night.run_jobs(jobs, 2, out, os.environ, records, lambda: None))
            self.assertEqual(records["bad"]["exit_code"], 7)
            self.assertEqual(records["missing"]["status"], "RUN-FAIL")
            self.assertEqual(records["good"]["status"], "RUN-OK")

    def test_interruption_reaps_started_workers(self):
        with tempfile.TemporaryDirectory() as directory, redirect_stdout(io.StringIO()):
            records = {}
            def interrupt():
                if any(row["status"] == "RUNNING" for row in records.values()):
                    raise KeyboardInterrupt
            with self.assertRaises(KeyboardInterrupt):
                night.run_jobs([night.Job("child", [sys.executable, "-c", "import time; time.sleep(30)"])],
                               1, Path(directory), os.environ, records, interrupt)
            self.assertEqual(records["child"]["status"], "INTERRUPTED")
            with self.assertRaises(ProcessLookupError):
                os.kill(records["child"]["pid"], 0)

    def test_setup_failure_stops_all_measurement_lanes(self):
        with tempfile.TemporaryDirectory() as directory, redirect_stdout(io.StringIO()):
            out = Path(directory) / "run"
            policy = {"worker_limit": 8, "cpu_ids": None}
            args = ["bench-night", str(out), "--allow-macos-qos"]
            env = {"BUMBLEDB_BENCH_BIN": sys.executable, "BENCH_NIGHT_UNDER_LOCK": "1"}
            with patch.object(sys, "argv", args), patch.dict(os.environ, env), patch.object(night, "policy_for_host", return_value=policy), patch.object(night, "run_jobs", return_value=False) as runner:
                self.assertEqual(night.main(), 1)
            self.assertEqual(runner.call_count, 1)
            self.assertEqual(json.loads((out / "MANIFEST.json").read_text())["status"], "SETUP-FAILED")


if __name__ == "__main__":
    unittest.main()

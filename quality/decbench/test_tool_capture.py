"""Verify process failures retain their scope and output remains bounded."""

import os
import subprocess
import sys
import unittest

from tool_capture import OutputLimit, run


class ToolCaptureTest(unittest.TestCase):
    def execute(self, body, timeout=3):
        return run([sys.executable, "-c", body], timeout=timeout, output_limit=128, diagnostic_limit=128)

    def test_captures_both_streams_with_exit_status(self):
        result = self.execute("import sys; print('result'); print('detail', file=sys.stderr); sys.exit(3)")
        self.assertEqual((result.returncode, result.stdout, result.stderr), (3, b"result\n", b"detail\n"))

    def test_extra_publication_is_reported_as_a_capture_failure(self):
        with self.assertRaisesRegex(OutputLimit, "stdout"):
            self.execute("import sys; sys.stdout.write('a' * 8192)")
        with self.assertRaisesRegex(OutputLimit, "stderr"):
            self.execute("import sys; sys.stderr.write('a' * 8192)")

    @unittest.skipUnless(os.name == "posix", "process groups require POSIX")
    def test_timeout_releases_a_pipe_still_open_in_a_child(self):
        with self.assertRaises(subprocess.TimeoutExpired):
            self.execute("import subprocess, sys; subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(5)'])",
                         timeout=0.25)

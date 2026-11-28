"""Run a benchmark subprocess with bounded captured output."""

from concurrent.futures import ThreadPoolExecutor
import os
import signal
import subprocess
import time


class OutputLimit(ValueError):
    """The subprocess published more data than the evaluator can retain."""


def run(argv, *, timeout, output_limit, diagnostic_limit, cwd=None, env=None):
    process_group = os.name == "posix"
    end = time.monotonic() + timeout
    with subprocess.Popen(argv, stdout=subprocess.PIPE, stderr=subprocess.PIPE, cwd=cwd, env=env,
                          start_new_session=process_group) as child:
        def stop():
            if not process_group:
                child.kill()
                return
            try:
                os.killpg(child.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass

        def read(pipe, limit):
            result = bytearray()
            while len(result) <= limit:
                block = pipe.read1(min(64 * 1024, limit + 1 - len(result)))
                if not block:
                    return result
                result.extend(block)
            stop()
            return result

        with ThreadPoolExecutor(max_workers=2) as readers:
            output = readers.submit(read, child.stdout, output_limit)
            diagnostics = readers.submit(read, child.stderr, diagnostic_limit)
            try:
                code = child.wait(timeout=max(0, end - time.monotonic()))
                stdout = output.result(timeout=max(0, end - time.monotonic()))
                stderr = diagnostics.result(timeout=max(0, end - time.monotonic()))
            except (subprocess.TimeoutExpired, TimeoutError):
                stop()
                raise subprocess.TimeoutExpired(argv, timeout) from None
            if len(stdout) > output_limit:
                raise OutputLimit(f"subprocess stdout exceeds the byte bound of {output_limit}")
            if len(stderr) > diagnostic_limit:
                raise OutputLimit(f"subprocess stderr exceeds the byte bound of {diagnostic_limit}")
    return subprocess.CompletedProcess(argv, code, bytes(stdout), bytes(stderr))

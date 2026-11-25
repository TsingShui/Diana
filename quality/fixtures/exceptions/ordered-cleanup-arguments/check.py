"""Run the published Java body with the nine helper models in the fixture README."""

import re
import subprocess
import sys
from pathlib import Path
from tempfile import TemporaryDirectory


def helper(match):
    result, name, arguments = match.groups()
    params = [p.removeprefix("int ") for p in arguments.split(", ")]
    if name == "read":
        body = f'event("read", E, {params[0]}); return {params[0]} == 10 ? 3 : 7;'
    elif name == "mark":
        body = f'event("mark", M, {params[0]});'
    else:
        body = f'event("close", C, {params[0]}, {params[1]});'
    return f"public static {result} {name}({arguments}) {{ {body} }}"


MODEL = """
    static final RuntimeException E = new RuntimeException("read");
    static final RuntimeException C = new RuntimeException("close");
    static final RuntimeException M = new RuntimeException("mark");
    static final java.util.ArrayList<String> trace = new java.util.ArrayList<>();
    static String[] faults;

    static void event(String operation, RuntimeException fault, int... args) {
        String call = operation + java.util.Arrays.toString(args);
        trace.add(call);
        String selected = trace.size() + ":" + call;
        if (java.util.Arrays.asList(faults).contains(selected)) throw fault;
    }

    public static void main(String[] args) {
        String r1 = "read[10]", r2 = "read[20]", mark = "mark[3]";
        String c12 = "close[10, 20]", c21 = "close[20, 10]";
        String[][] inputs = {
            {}, {"1:" + r1}, {"1:" + r1, "2:" + c12}, {"2:" + c12},
            {"3:" + r2}, {"3:" + r2, "4:" + mark}, {"3:" + r2, "5:" + c12},
            {"4:" + mark}, {"5:" + c21}
        };
        String[][] traces = {
            {r1,c12,r2,mark,c21}, {r1,c12}, {r1,c12}, {r1,c12},
            {r1,c12,r2,mark,c12}, {r1,c12,r2,mark}, {r1,c12,r2,mark,c12},
            {r1,c12,r2,mark}, {r1,c12,r2,mark,c21}
        };
        Throwable[] exceptions = {null,E,C,C,E,M,C,M,C};
        int failed = 0;
        for (int i=0; i<inputs.length; i++) {
            faults = inputs[i];
            trace.clear();
            int result = 0;
            Throwable thrown = null;
            try { result = run(10, 20); } catch (Throwable e) { thrown = e; }
            if (!trace.equals(java.util.Arrays.asList(traces[i])) ||
                thrown != exceptions[i] || (thrown == null && result != 10)) {
                failed++;
                System.err.println("scenario " + i + ", trace " + trace + ", exit " +
                    (thrown == null ? result : thrown.getMessage()));
            }
        }
        System.out.println("scenarios=" + inputs.length + " failed=" + failed);
        if (failed > 0) System.exit(1);
    }
"""


def check(source):
    program, count = re.subn(
        r"public static native (void|int) (read|mark|close)\((int \w+(?:, int \w+)?)\);",
        helper, source.read_text(),
    )
    assert count == 3, "expected all three native helper declarations"
    body, end = program.rsplit("}", 1)
    with TemporaryDirectory() as tmp:
        main = Path(tmp, "OrderedCleanupArguments.java")
        main.write_text(body + MODEL + "}" + end)
        return subprocess.run(["java", str(main)]).returncode


if __name__ == "__main__":
    sys.exit(check(Path(sys.argv[1])))

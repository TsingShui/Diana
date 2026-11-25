"""Exercise the recovered run body with the fault matrix from README.md."""

import re
import subprocess
import sys
from pathlib import Path
from tempfile import TemporaryDirectory


def helper(match):
    result, name, args = match.groups()
    arg = args.split()[-1] if args else ""
    argument = '""' if not arg else f"String.valueOf({arg})"
    if name == "log":
        argument = f'({arg} == I ? "I" : "?")'
    body = f'event("{name}", {argument});'
    throws = ""
    if name in ("acquire", "read"):
        throws = " throws IOException"
        body += f' if (faults.contains("{name}I")) throw I;'
    if name == "acquire":
        body += " return 21;"
    if name == "read":
        body += ' if (faults.contains("readR")) throw R; return 8;'
    return f"public static {result} {name}({args}){throws} {{ {body} }}"


MAIN = """
    static final IOException I = new IOException("I");
    static final RuntimeException P = new RuntimeException("P"), R = new RuntimeException("R"),
        L = new RuntimeException("L"), A = new RuntimeException("A"), C = new RuntimeException("C");
    static java.util.Set<String> faults;
    static String trace;
    static void event(String helper, String arg) {
        trace += helper + arg + " ";
        if (!faults.contains(helper)) return;
        throw switch (helper) {
            case "probe" -> P; case "log" -> L; case "audit" -> A; case "close" -> C;
            default -> new IllegalArgumentException(helper);
        };
    }
    public static void main(String[] args) {
        String first = "acquire7 ", second = first + "probe ", third = second + "read21 ";
        String logged = third + "logI ", audited = logged + "audit21 ", closed = "close21 ";
        String[][] selectors = {{},{"acquireI"},{"probe"},{"readI"},{"readI","log"},
            {"readI","log","close"},{"readI","audit"},{"readI","close"},{"readR"},
            {"readR","close"},{"close"}};
        String[] expected = {third+closed, first, second, audited+closed, logged+closed,
            logged+closed, audited, audited+closed, third+closed, third+closed, third+closed};
        Throwable[] errors = {null,I,P,null,L,C,A,C,R,C,C};
        int failed = 0;
        for (int i=0; i<selectors.length; i++) {
            faults = java.util.Set.of(selectors[i]); trace = "";
            Throwable thrown = null; int result = 0;
            try { result = run(7); } catch (Throwable t) { thrown=t; }
            if (!trace.equals(expected[i]) || thrown != errors[i] ||
                (thrown == null && result != (i == 3 ? -2 : 8))) {
                failed++;
                System.err.println("scenario " + i + ", trace " + trace + ", exit " +
                    (thrown == null ? result : thrown.getMessage()));
            }
        }
        System.out.println("scenarios=" + selectors.length + " failed=" + failed);
        if (failed > 0) System.exit(1);
    }
"""


def check(source):
    program, count = re.subn(
        r"public static native (int|void) (acquire|probe|read|log|audit|close)\(([^)]*)\)(?: throws IOException)?;",
        helper, source.read_text(),
    )
    assert count == 6
    program, count = re.subn(
        r"(public static int run\(int \w+\))(?: throws (?:IOException|Throwable))? \{",
        r"\1 throws Throwable {", program,
    )
    assert count == 1
    prefix, suffix = program.rsplit("}", 1)
    with TemporaryDirectory() as tmp:
        main = Path(tmp, "HandlerCleanupBoundary.java")
        main.write_text(prefix + MAIN + "}" + suffix)
        return subprocess.run(["java", str(main)]).returncode


if __name__ == "__main__":
    sys.exit(check(Path(sys.argv[1])))

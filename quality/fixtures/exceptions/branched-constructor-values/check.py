"""Run the fixture's Java body with observable constructor and handler effects."""

import re
import subprocess
import sys
from pathlib import Path
from tempfile import TemporaryDirectory


def implement_native(match):
    result, name, signature = match.groups()
    args = [part.split()[-1] for part in signature.split(",")]
    if name == "record":
        body = f'event("record(" + {args[0]} + ")");'
    elif name == "consume":
        x, y = args
        body = f'event("consume(" + {x} + "," + {y} + ")"); return {x} * 100 + {y};'
    else:
        body = f'event("recover(" + {args[0]} + ")"); return -{args[0]};'
    return f"private static {result} {name}({signature}) {{ {body} }}"


MODEL = '''
    private static String trace;
    private static String primarySite;
    private static String otherSite;
    private static final IllegalStateException PRIMARY = new IllegalStateException();
    private static final ArithmeticException OTHER = new ArithmeticException();

    private static void event(String call) {
        trace = trace.isEmpty() ? call : trace + " " + call;
        if (call.equals(primarySite)) throw PRIMARY;
        if (call.equals(otherSite)) throw OTHER;
    }

    private static void check(boolean first, boolean second, String primary, String other,
            String expectedTrace, String expectedExit) {
        trace = "";
        primarySite = primary;
        otherSite = other;
        String exit;
        try {
            exit = Integer.toString(run(first, second));
        } catch (RuntimeException e) {
            if (e == PRIMARY) exit = "primary";
            else if (e == OTHER) exit = "other";
            else throw e;
        }
        if (!trace.equals(expectedTrace) || !exit.equals(expectedExit)) {
            throw new AssertionError(trace + " -> " + exit);
        }
    }

    public static void main(String[] args) {
        check(true, true, "", "", "record(3) record(5) consume(3,5)", "305");
        check(true, false, "", "", "record(3) record(11) consume(3,11)", "311");
        check(false, true, "", "", "record(7) record(5) consume(7,5)", "705");
        check(false, false, "", "", "record(7) record(11) consume(7,11)", "711");
        check(false, true, "record(7)", "", "record(7)", "primary");
        check(false, false, "record(11)", "", "record(7) record(11) recover(7)", "-7");
        check(true, false, "consume(3,11)", "", "record(3) record(11) consume(3,11) recover(3)", "-3");
        check(false, true, "record(5)", "recover(7)", "record(7) record(5) recover(7)", "other");
        check(true, false, "", "record(11)", "record(3) record(11)", "other");
        System.out.println("9 constructor paths passed");
    }
'''


def check(source):
    text = Path(source).read_text()
    text, count = re.subn(
        r"private static native (void|int) (record|consume|recover)\(([^)]*)\);",
        implement_native, text,
    )
    if count != 3:
        raise ValueError(f"Expected three fixture helpers, found {count}")
    with TemporaryDirectory() as tmp:
        java = Path(tmp, "BranchedConstructorValues.java")
        java.write_text(text.rstrip().removesuffix("}") + MODEL + "}\n")
        subprocess.run(["java", str(java)], check=True)


if __name__ == "__main__":
    check(sys.argv[1])

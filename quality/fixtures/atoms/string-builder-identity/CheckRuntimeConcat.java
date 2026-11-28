import java.util.function.Supplier;

public final class CheckRuntimeConcat {
    private static void check(String name, Supplier<String> operation, String expectedText, boolean expectedInterned) {
        String first = operation.get();
        String second = operation.get();
        boolean content = expectedText.equals(first) && expectedText.equals(second);
        boolean interned = first == expectedText && second == expectedText;
        boolean repeated = first == second;
        boolean pass = content && (first == expectedText) == expectedInterned
                && (second == expectedText) == expectedInterned && repeated == expectedInterned;
        System.out.println(name + "\t" + (pass ? "PASS" : "FAIL") + "\t" + content + "\t" + interned
                + "\t" + repeated + "\t" + expectedInterned);
    }

    public static void main(String[] args) {
        check("literal-builder", RuntimeConcat::literalBuilder, "prefixsuffix", false);
        check("literal-buffer", RuntimeConcat::literalBuffer, "value=7", false);
        check("equal-branch-builder-true", () -> RuntimeConcat.equalBranchBuilder(true), "prefixsuffix", false);
        check("equal-branch-builder-false", () -> RuntimeConcat.equalBranchBuilder(false), "prefixsuffix", false);
        check("equal-branch-buffer-true", () -> RuntimeConcat.equalBranchBuffer(true), "value=7", false);
        check("equal-branch-buffer-false", () -> RuntimeConcat.equalBranchBuffer(false), "value=7", false);
        check("blank-final-string-indy", RuntimeConcat::blankStringIndy, "prefixsuffix", false);
        check("blank-final-primitive-indy", RuntimeConcat::blankPrimitiveIndy, "value=7", false);
        check("blank-final-field-builder", RuntimeConcat::blankFieldBuilder, "prefixsuffix", false);
        check("compiled-string-literal", RuntimeConcat::compilerStringLiteral, "prefixsuffix", true);
        check("compiled-primitive-literal", RuntimeConcat::compilerPrimitiveLiteral, "value=7", true);
        check("compiled-fields-builder", RuntimeConcat::compilerFieldsBuilder, "prefix7", false);
        check("runtime-final-field-indy", RuntimeConcat::runtimeFieldIndy, "prefixsuffix", false);
        check("mutable-field-indy", RuntimeConcat::mutableFieldIndy, "value=7", false);
        check("runtime-builder", () -> RuntimeConcat.runtimeBuilder(7), "value=7", false);
        check("runtime-buffer", () -> RuntimeConcat.runtimeBuffer("suffix"), "prefixsuffix", false);
        check("runtime-buffer-empty", () -> RuntimeConcat.runtimeBuffer(""), "prefix", false);
        check("runtime-buffer-null", () -> RuntimeConcat.runtimeBuffer(null), "prefixnull", false);
        check("runtime-indy", () -> RuntimeConcat.runtimeIndy("prefix"), "prefixsuffix", false);
        check("runtime-indy-empty", () -> RuntimeConcat.runtimeIndy(""), "suffix", false);
    }
}

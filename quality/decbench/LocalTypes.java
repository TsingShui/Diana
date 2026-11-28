import com.sun.source.tree.AnnotatedTypeTree;
import com.sun.source.tree.ArrayTypeTree;
import com.sun.source.tree.BindingPatternTree;
import com.sun.source.tree.CatchTree;
import com.sun.source.tree.ClassTree;
import com.sun.source.tree.CompilationUnitTree;
import com.sun.source.tree.ExpressionTree;
import com.sun.source.tree.IdentifierTree;
import com.sun.source.tree.ImportTree;
import com.sun.source.tree.LambdaExpressionTree;
import com.sun.source.tree.MethodTree;
import com.sun.source.tree.ParameterizedTypeTree;
import com.sun.source.tree.ParenthesizedTree;
import com.sun.source.tree.ReturnTree;
import com.sun.source.tree.Tree;
import com.sun.source.tree.TypeCastTree;
import com.sun.source.tree.TypeParameterTree;
import com.sun.source.tree.VariableTree;
import com.sun.source.util.JavacTask;
import com.sun.source.util.TreeScanner;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.util.Base64;
import java.util.List;
import javax.tools.Diagnostic;
import javax.tools.DiagnosticCollector;
import javax.tools.JavaFileObject;
import javax.tools.StandardJavaFileManager;
import javax.tools.ToolProvider;

/** Read declarations from recovered Java syntax through the public JDK parser. */
class LocalTypes {
    private static void emit(String... values) {
        var encoder = Base64.getUrlEncoder().withoutPadding();
        var line = new StringBuilder(values[0]);
        for (int index = 1; index < values.length; index++) {
            line.append('\t').append(encoder.encodeToString(values[index].getBytes(StandardCharsets.UTF_8)));
        }
        System.out.println(line);
    }

    private static String erasedSpelling(Tree type) {
        if (type == null) return "";
        return switch (type.getKind()) {
            case ARRAY_TYPE -> {
                String item = erasedSpelling(((ArrayTypeTree) type).getType());
                yield item.isEmpty() ? "" : item + "[]";
            }
            case PARAMETERIZED_TYPE -> erasedSpelling(((ParameterizedTypeTree) type).getType());
            case ANNOTATED_TYPE -> erasedSpelling(((AnnotatedTypeTree) type).getUnderlyingType());
            case PRIMITIVE_TYPE, IDENTIFIER, MEMBER_SELECT -> type.toString();
            default -> "";
        };
    }

    private static class Locals extends TreeScanner<Void, Void> {
        private final String index;

        Locals(String index) { this.index = index; }

        void variable(String kind, VariableTree tree) {
            emit("V", index, kind, tree.getName().toString(), erasedSpelling(tree.getType()));
        }

        @Override public Void visitVariable(VariableTree tree, Void ignored) {
            variable("local", tree);
            return super.visitVariable(tree, ignored);
        }

        @Override public Void visitCatch(CatchTree tree, Void ignored) {
            variable("catch", tree.getParameter());
            return scan(tree.getBlock(), ignored);
        }

        @Override public Void visitReturn(ReturnTree tree, Void ignored) {
            ExpressionTree value = tree.getExpression();
            while (true) {
                if (value instanceof ParenthesizedTree parens) {
                    value = parens.getExpression();
                } else if (value instanceof TypeCastTree cast) {
                    value = cast.getExpression();
                } else {
                    break;
                }
            }
            emit("R", index, value instanceof IdentifierTree name ? name.getName().toString() : "");
            return null;
        }

        // These bodies introduce different scopes and return values.
        @Override public Void visitClass(ClassTree tree, Void ignored) { return null; }
        @Override public Void visitLambdaExpression(LambdaExpressionTree tree, Void ignored) { return null; }
        @Override public Void visitBindingPattern(BindingPatternTree tree, Void ignored) {
            variable("pattern", tree.getVariable());
            return null;
        }
    }

    private static void collect(CompilationUnitTree unit) {
        var imported = unit.getImports();
        String pkg = unit.getPackageName() == null ? "" : unit.getPackageName().toString() + ".";
        for (ImportTree anImport : imported) {
            emit("I", anImport.getQualifiedIdentifier().toString(), Boolean.toString(anImport.isStatic()));
        }
        new TreeScanner<Void, Void>() {
            @Override public Void visitClass(ClassTree tree, Void ignored) {
                emit("S", tree.getSimpleName().toString());
                return super.visitClass(tree, ignored);
            }

            @Override public Void visitTypeParameter(TypeParameterTree tree, Void ignored) {
                emit("S", tree.getName().toString());
                return super.visitTypeParameter(tree, ignored);
            }
        }.scan(unit, null);

        int index = 0;
        for (Tree definition : unit.getTypeDecls()) {
            if (!(definition instanceof ClassTree owner)) continue;
            for (Tree member : owner.getMembers()) {
                if (!(member instanceof MethodTree method) || method.getBody() == null) continue;
                String identifier = Integer.toString(index++);
                emit("M", identifier, pkg + owner.getSimpleName(), method.getName().toString(),
                     method.getReturnType() == null ? "void" : erasedSpelling(method.getReturnType()));
                var locals = new Locals(identifier);
                for (VariableTree parameter : method.getParameters()) {
                    locals.variable("parameter", parameter);
                }
                locals.scan(method.getBody(), null);
            }
        }
    }

    public static void main(String[] args) throws Exception {
        if (args.length != 1) throw new IllegalArgumentException("expected one recovered source file");
        var compiler = ToolProvider.getSystemJavaCompiler();
        if (compiler == null) throw new IllegalStateException("a JDK compiler is required");
        var diagnostics = new DiagnosticCollector<JavaFileObject>();
        try (StandardJavaFileManager manager =
                 compiler.getStandardFileManager(diagnostics, null, StandardCharsets.UTF_8)) {
            var task = (JavacTask) compiler.getTask(
                null, manager, diagnostics, List.of("-proc:none"), null,
                manager.getJavaFileObjectsFromPaths(List.of(Path.of(args[0])))
            );
            var units = task.parse();
            for (var diagnostic : diagnostics.getDiagnostics()) {
                if (diagnostic.getKind() == Diagnostic.Kind.ERROR) {
                    System.out.println("UNPARSEABLE");
                    return;
                }
            }
            for (var unit : units) collect(unit);
        }
    }
}

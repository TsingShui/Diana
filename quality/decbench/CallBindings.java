import com.sun.source.tree.*;
import com.sun.source.util.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.util.*;
import javax.lang.model.element.*;
import javax.lang.model.type.*;
import javax.lang.model.util.Elements;
import javax.lang.model.util.Types;
import javax.tools.*;

/** Resolves ordinary call targets in a recovered Java class without emitting or executing it. */
public final class CallBindings {
    private final Trees trees;
    private final Elements elements;
    private final Types types;
    private final List<Diagnostic<? extends JavaFileObject>> diagnostics;

    private CallBindings(JavacTask task, DiagnosticCollector<JavaFileObject> diagnostics) {
        trees = Trees.instance(task);
        elements = task.getElements();
        types = task.getTypes();
        this.diagnostics = diagnostics.getDiagnostics();
    }

    private static String field(String value) {
        return Base64.getUrlEncoder().withoutPadding().encodeToString(value.getBytes(StandardCharsets.UTF_8));
    }

    private static void emit(String event, String... values) {
        StringJoiner row = new StringJoiner("\t");
        row.add(event);
        for (String value : values) row.add(field(value));
        System.out.println(row);
    }

    private String descriptor(TypeMirror raw) {
        TypeMirror type = types.erasure(raw);
        return switch (type.getKind()) {
            case BOOLEAN -> "Z";
            case BYTE -> "B";
            case CHAR -> "C";
            case SHORT -> "S";
            case INT -> "I";
            case LONG -> "J";
            case FLOAT -> "F";
            case DOUBLE -> "D";
            case VOID -> "V";
            case ARRAY -> "[" + descriptor(((ArrayType) type).getComponentType());
            case DECLARED -> owner((TypeElement) ((DeclaredType) type).asElement());
            default -> throw new IllegalArgumentException("unresolved type");
        };
    }

    private String owner(TypeElement type) {
        String binary = elements.getBinaryName(type).toString();
        if (binary.isEmpty() || type.asType().getKind() != TypeKind.DECLARED) {
            throw new IllegalArgumentException("unresolved owner");
        }
        return "L" + binary.replace('.', '/') + ";";
    }

    private String[] executable(Element element) {
        if (!(element instanceof ExecutableElement target)
                || !(target.getEnclosingElement() instanceof TypeElement enclosing)) {
            throw new IllegalArgumentException("unresolved call");
        }
        StringJoiner arguments = new StringJoiner("", "(", ")");
        for (VariableElement parameter : target.getParameters()) {
            arguments.add(descriptor(parameter.asType()));
        }
        return new String[] {
            owner(enclosing), target.getSimpleName().toString(),
            arguments + descriptor(target.getReturnType()),
        };
    }

    private boolean hasError(long start, long end) {
        for (Diagnostic<? extends JavaFileObject> diagnostic : diagnostics) {
            if (diagnostic.getKind() == Diagnostic.Kind.ERROR
                    && diagnostic.getPosition() >= start && diagnostic.getPosition() < end) {
                return true;
            }
        }
        return false;
    }

    private final class Calls extends TreePathScanner<Void, Void> {
        private final CompilationUnitTree unit;
        private final String member;
        private String unsupported;
        private boolean returned;

        Calls(CompilationUnitTree unit, String member) {
            this.unit = unit;
            this.member = member;
        }

        @Override
        public Void scan(Tree tree, Void unused) {
            if (tree != null) {
                if (returned) {
                    unsupported = "statement after return";
                    return null;
                }
                switch (tree.getKind()) {
                    case IF, CONDITIONAL_EXPRESSION, CONDITIONAL_AND, CONDITIONAL_OR,
                         FOR_LOOP, ENHANCED_FOR_LOOP, WHILE_LOOP, DO_WHILE_LOOP,
                         SWITCH, SWITCH_EXPRESSION, TRY, SYNCHRONIZED, THROW,
                         LABELED_STATEMENT, BREAK, CONTINUE, ASSERT,
                         LAMBDA_EXPRESSION, MEMBER_REFERENCE, CLASS, ERRONEOUS -> {
                        if (unsupported == null) unsupported = tree.getKind().toString();
                        return null;
                    }
                    default -> {}
                }
            }
            return super.scan(tree, unused);
        }

        @Override
        public Void visitReturn(ReturnTree tree, Void unused) {
            super.visitReturn(tree, unused);
            returned = true;
            return null;
        }

        private String dispatch(ExecutableElement target, boolean superReceiver) {
            if (target.getKind() == ElementKind.CONSTRUCTOR) {
                TypeElement type = (TypeElement) target.getEnclosingElement();
                if (type.getKind() == ElementKind.ENUM
                        || type.getNestingKind().isNested() && !type.getModifiers().contains(Modifier.STATIC)) {
                    throw new IllegalArgumentException("constructor can have synthetic bytecode parameters");
                }
                return "constructor";
            }
            if (target.getModifiers().contains(Modifier.STATIC)) return "static";
            return superReceiver ? "super" : "instance";
        }

        private void checkExpression(TreePath parent, ExpressionTree expression) {
            if (expression == null) return;
            TypeMirror mirror = trees.getTypeMirror(new TreePath(parent, expression));
            while (mirror instanceof ArrayType array) mirror = array.getComponentType();
            if (mirror == null || mirror.getKind() == TypeKind.ERROR) {
                throw new IllegalArgumentException("unresolved receiver or argument type");
            }
        }

        private void call(Tree tree, String lexical, boolean superReceiver) {
            long start = trees.getSourcePositions().getStartPosition(unit, tree);
            long end = trees.getSourcePositions().getEndPosition(unit, tree);
            if (start == Diagnostic.NOPOS || end == Diagnostic.NOPOS) {
                return;  // Attributed implicit calls have no source site.
            }
            long line = unit.getLineMap().getLineNumber(start);
            long column = unit.getLineMap().getColumnNumber(start);
            try {
                if (hasError(start, end)) throw new IllegalArgumentException("call has a compiler diagnostic");
                TreePath path = getCurrentPath();
                if (tree instanceof MethodInvocationTree method) {
                    if (method.getMethodSelect() instanceof MemberSelectTree select) {
                        checkExpression(new TreePath(path, select), select.getExpression());
                    }
                    for (ExpressionTree argument : method.getArguments()) checkExpression(path, argument);
                } else if (tree instanceof NewClassTree creation) {
                    checkExpression(path, creation.getEnclosingExpression());
                    for (ExpressionTree argument : creation.getArguments()) checkExpression(path, argument);
                }
                Element element = trees.getElement(path);
                String[] signature = executable(element);
                emit("call", member, lexical, signature[0], signature[1], signature[2],
                        dispatch((ExecutableElement) element, superReceiver), Long.toString(line),
                        Long.toString(column), "");
            } catch (IllegalArgumentException error) {
                emit("call", member, lexical, "", "", "", "", Long.toString(line),
                        Long.toString(column), error.getMessage());
            }
        }

        @Override
        public Void visitMethodInvocation(MethodInvocationTree tree, Void unused) {
            super.visitMethodInvocation(tree, unused);
            String lexical = "";
            boolean superReceiver = false;
            if (tree.getMethodSelect() instanceof IdentifierTree name) {
                lexical = name.getName().toString();
            } else if (tree.getMethodSelect() instanceof MemberSelectTree name) {
                lexical = name.getIdentifier().toString();
                ExpressionTree receiver = name.getExpression();
                superReceiver = receiver instanceof IdentifierTree simple && simple.getName().contentEquals("super")
                        || receiver instanceof MemberSelectTree qualified && qualified.getIdentifier().contentEquals("super");
            }
            call(tree, lexical.equals("this") || lexical.equals("super") ? "<init>" : lexical, superReceiver);
            return null;
        }

        @Override
        public Void visitNewClass(NewClassTree tree, Void unused) {
            if (tree.getClassBody() != null) {
                unsupported = "anonymous class";
            } else {
                super.visitNewClass(tree, unused);
                call(tree, "<init>", false);
            }
            return null;
        }
    }

    private void process(CompilationUnitTree unit, Set<String> want) {
        new TreePathScanner<Void, Void>() {
            @Override
            public Void visitMethod(MethodTree tree, Void unused) {
                String[] signature;
                try {
                    signature = executable(trees.getElement(getCurrentPath()));
                } catch (IllegalArgumentException error) {
                    return null;
                }
                String member = signature[0] + "->" + signature[1] + signature[2];
                if (!want.contains(member)) return null;
                if (tree.getBody() == null
                        || trees.getSourcePositions().getEndPosition(unit, tree) == Diagnostic.NOPOS) {
                    emit("unchecked", member, "recovered method body is absent");
                    return null;
                }
                // Receiver construction and arguments are visited before their invocation.
                Calls calls = new Calls(unit, member);
                calls.scan(new TreePath(getCurrentPath(), tree.getBody()), null);
                if (calls.unsupported == null) emit("complete", member);
                else emit("unchecked", member, "nonlinear or unsupported source construct: " + calls.unsupported);
                return null;
            }
        }.scan(unit, null);
    }

    public static void main(String[] args) throws Exception {
        Path recovered = Path.of(args[0]);
        Path isolated = recovered.getParent();
        Set<String> want = new HashSet<>(Arrays.asList(args).subList(1, args.length));
        JavaCompiler compiler = ToolProvider.getSystemJavaCompiler();
        if (compiler == null) throw new IllegalStateException("JDK source attribution is unavailable");
        DiagnosticCollector<JavaFileObject> diagnostics = new DiagnosticCollector<>();
        try (StandardJavaFileManager files = compiler.getStandardFileManager(diagnostics, Locale.ROOT, StandardCharsets.UTF_8)) {
            List<String> options = List.of("-proc:none", "-classpath", isolated.toString(),
                    "-sourcepath", isolated.toString(), "-encoding", "UTF-8");
            JavacTask task = (JavacTask) compiler.getTask(null, files, diagnostics, options, null,
                    files.getJavaFileObjectsFromPaths(List.of(recovered)));
            Iterable<? extends CompilationUnitTree> units = task.parse();
            task.analyze();
            CallBindings bindings = new CallBindings(task, diagnostics);
            for (CompilationUnitTree unit : units) bindings.process(unit, want);
        }
    }
}

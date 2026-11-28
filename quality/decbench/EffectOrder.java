/* Project a conservative straight-line Java effect and value-provenance witness. */
import com.sun.source.tree.*;
import com.sun.source.util.JavacTask;
import com.sun.source.util.SourcePositions;
import com.sun.source.util.TreePath;
import com.sun.source.util.Trees;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import javax.lang.model.element.*;
import javax.lang.model.type.*;
import javax.lang.model.util.Elements;
import javax.lang.model.util.Types;
import javax.tools.*;

final class EffectOrder {
    private static final String STRING = "Ljava/lang/String;";
    private static final int MAX_EVENTS = 512;
    private static final int MAX_DEPTH = 64;
    private static final int MAX_VALUE = 8192;

    private static final class Unchecked extends RuntimeException {
        Unchecked(String reason) { super(reason); }
    }

    private record Value(String descriptor, String origin, boolean constant) {
        Value(String descriptor, String origin) {
            this(descriptor, origin, false);
        }
        String json() {
            return "{\"type\":" + quote(descriptor) + ",\"origin\":" + origin + "}";
        }
    }
    private record Field(String member, String type) {}

    private static String quote(String text) {
        var result = new StringBuilder("\"");
        for (int i = 0; i < text.length(); i++) {
            char value = text.charAt(i);
            if (value == '"' || value == '\\') result.append('\\').append(value);
            else if (value < 32 || Character.isSurrogate(value))
                result.append(String.format(Locale.ROOT, "\\u%04x", (int) value));
            else result.append(value);
        }
        return result.append('"').toString();
    }

    private static String descriptor(TypeMirror type, Types types, Elements elements) {
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
            case ARRAY -> "[" + descriptor(((ArrayType) type).getComponentType(), types, elements);
            case DECLARED -> {
                TypeElement element = (TypeElement) ((DeclaredType) type).asElement();
                yield "L" + elements.getBinaryName(element).toString().replace('.', '/') + ";";
            }
            case TYPEVAR -> descriptor(types.erasure(type), types, elements);
            default -> throw new Unchecked("type is unresolved or outside the effect model");
        };
    }

    private static String signature(ExecutableElement method, Types types, Elements elements) {
        var result = new StringBuilder(method.getSimpleName().toString()).append('(');
        for (VariableElement parameter : method.getParameters())
            result.append(descriptor(parameter.asType(), types, elements));
        return result.append(')').append(descriptor(method.getReturnType(), types, elements)).toString();
    }

    private static final class Witness {
        private final Trees trees;
        private final Types types;
        private final Elements elements;
        private final TypeElement owner;
        private final String ownerDescriptor;
        private final String returnType;
        private final Map<Element, Value> locals = new HashMap<>();
        private final List<String> events = new ArrayList<>();
        private boolean returned;

        Witness(JavacTask task, ExecutableElement method) {
            if (method.getModifiers().contains(Modifier.SYNCHRONIZED))
                throw new Unchecked("synchronized methods are outside the sequential effect model");
            trees = Trees.instance(task);
            types = task.getTypes();
            elements = task.getElements();
            owner = (TypeElement) method.getEnclosingElement();
            ownerDescriptor = descriptor(owner.asType(), types, elements);
            returnType = descriptor(method.getReturnType(), types, elements);
            int index = 0;
            for (VariableElement parameter : method.getParameters()) {
                String type = descriptor(parameter.asType(), types, elements);
                locals.put(parameter, new Value(type, "{\"parameter\":" + index++ + "}"));
            }
        }

        private String type(TreePath path) {
            TypeMirror type = trees.getTypeMirror(path);
            if (type == null) throw new Unchecked("expression type is not established");
            return descriptor(type, types, elements);
        }

        private int event(String kind, String suffix) {
            if (events.size() >= MAX_EVENTS) throw new Unchecked("effect witness exceeds its event bound");
            int index = events.size();
            events.add("{\"kind\":" + quote(kind) + suffix + "}");
            return index;
        }

        private Element binding(TreePath path) {
            Element item = trees.getElement(path);
            if (item == null) throw new Unchecked("variable or field binding is not established");
            return item;
        }

        private Field field(TreePath path, Element item) {
            if (item.getKind() != ElementKind.FIELD || !item.getEnclosingElement().equals(owner)
                || item.getModifiers().contains(Modifier.STATIC))
                throw new Unchecked("field is not an instance field of the selected class");
            if (item.getModifiers().contains(Modifier.VOLATILE))
                throw new Unchecked("volatile fields are outside the sequential effect model");
            if (((VariableElement) item).getConstantValue() != null)
                throw new Unchecked("compile-time constant field reads are outside the effect model");
            Tree tree = path.getLeaf();
            if (tree instanceof MemberSelectTree select &&
                (!(select.getExpression() instanceof IdentifierTree receiver)
                    || !receiver.getName().contentEquals("this")))
                throw new Unchecked("field receiver is outside the explicit this model");
            String type = descriptor(item.asType(), types, elements);
            requireValueType(type);
            return new Field(ownerDescriptor + "->" + item.getSimpleName() + ":" + type, type);
        }

        private static void requireValueType(String type) {
            if (!type.equals("I") && !type.equals(STRING))
                throw new Unchecked("only int and String values are modeled");
        }

        private Value read(TreePath path) {
            Element item = binding(path);
            if (locals.containsKey(item)) {
                Value local = locals.get(item);
                if (local == null) throw new Unchecked("local value has no modeled definition");
                requireValueType(local.descriptor());
                if (((VariableElement) item).getConstantValue() != null)
                    throw new Unchecked("compile-time constant local reads are outside the effect model");
                return new Value(local.descriptor(), local.origin());
            }
            Field field = field(path, item);
            int index = event("read", ",\"receiver\":\"this\",\"member\":" + quote(field.member()));
            return new Value(field.type(), "{\"read\":" + index + "}");
        }

        private void write(TreePath path, Value value) {
            Element item = binding(path);
            String targetType = descriptor(item.asType(), types, elements);
            if (!targetType.equals(value.descriptor()))
                throw new Unchecked("assignment conversion is outside the effect model");
            if (locals.containsKey(item)) {
                locals.put(item, value);
                return;
            }
            Field field = field(path, item);
            event("write", ",\"receiver\":\"this\",\"member\":" + quote(field.member()) + ",\"value\":" + value.json());
        }

        private Value operation(String op, Value left, Value right) {
            boolean concatenate = op.equals("+")
                && (left.descriptor().equals(STRING) || right.descriptor().equals(STRING));
            requireValueType(left.descriptor());
            requireValueType(right.descriptor());
            boolean constant = left.constant() && right.constant();
            if (concatenate && constant)
                throw new Unchecked("interned constant concatenation is outside the effect model");
            if (!concatenate && (!left.descriptor().equals("I") || !right.descriptor().equals("I")))
                throw new Unchecked("numeric operation is outside the int model");
            if (left.origin().length() + right.origin().length() > MAX_VALUE)
                throw new Unchecked("value provenance exceeds its size bound");
            String origin = "{\"operation\":" + quote(concatenate ? "concat" : op)
                + ",\"operands\":[" + left.json() + "," + right.json() + "]}";
            return new Value(concatenate ? STRING : "I", origin, constant);
        }

        private Value literal(LiteralTree literal) {
            Object value = literal.getValue();
            if (value instanceof Integer number)
                return new Value("I", "{\"literal\":" + number + "}", true);
            if (value instanceof String string) {
                if (string.length() > MAX_VALUE) throw new Unchecked("literal exceeds the model size bound");
                return new Value(STRING, "{\"literal\":" + quote(string) + "}", true);
            }
            throw new Unchecked("literal is outside the int and String model");
        }

        private Value value(TreePath path, int depth) {
            if (depth > MAX_DEPTH) throw new Unchecked("expression exceeds the effect depth bound");
            Tree leaf = path.getLeaf();
            return switch (leaf.getKind()) {
                case IDENTIFIER, MEMBER_SELECT -> read(path);
                case PARENTHESIZED -> value(new TreePath(path, ((ParenthesizedTree) leaf).getExpression()), depth + 1);
                case INT_LITERAL, STRING_LITERAL -> literal((LiteralTree) leaf);
                case PLUS, MINUS -> {
                    BinaryTree operation = (BinaryTree) leaf;
                    Value left = value(new TreePath(path, operation.getLeftOperand()), depth + 1);
                    Value right = value(new TreePath(path, operation.getRightOperand()), depth + 1);
                    yield operation(leaf.getKind() == Tree.Kind.PLUS ? "+" : "-", left, right);
                }
                case UNARY_MINUS -> {
                    ExpressionTree operand = ((UnaryTree) leaf).getExpression();
                    if (operand instanceof LiteralTree constant && constant.getValue() instanceof Integer number)
                        yield new Value("I", "{\"literal\":" + -number + "}", true);
                    yield operation("-", new Value("I", "{\"literal\":0}", true),
                                    value(new TreePath(path, operand), depth + 1));
                }
                case TYPE_CAST -> {
                    Value inside = value(new TreePath(path, ((TypeCastTree) leaf).getExpression()), depth + 1);
                    if (!type(path).equals(inside.descriptor()))
                        throw new Unchecked("conversion changes the modeled expression type");
                    yield inside;
                }
                default -> throw new Unchecked("unsupported expression in effect witness: " + leaf.getKind());
            };
        }

        private void expressionStatement(TreePath path) {
            Tree tree = path.getLeaf();
            switch (tree.getKind()) {
                case ASSIGNMENT -> {
                    AssignmentTree assignment = (AssignmentTree) tree;
                    TreePath target = new TreePath(path, assignment.getVariable());
                    // An explicit receiver has to be the effect-free, non-null this receiver.
                    Element item = binding(target);
                    if (!locals.containsKey(item)) field(target, item);
                    write(target, value(new TreePath(path, assignment.getExpression()), 0));
                }
                case PLUS_ASSIGNMENT, MINUS_ASSIGNMENT -> {
                    CompoundAssignmentTree assignment = (CompoundAssignmentTree) tree;
                    TreePath target = new TreePath(path, assignment.getVariable());
                    Value left = read(target);
                    Value right = value(new TreePath(path, assignment.getExpression()), 0);
                    write(target, operation(tree.getKind() == Tree.Kind.PLUS_ASSIGNMENT ? "+" : "-", left, right));
                }
                case PREFIX_INCREMENT, POSTFIX_INCREMENT, PREFIX_DECREMENT, POSTFIX_DECREMENT -> {
                    UnaryTree update = (UnaryTree) tree;
                    TreePath target = new TreePath(path, update.getExpression());
                    Value previous = read(target);
                    if (!previous.descriptor().equals("I"))
                        throw new Unchecked("increment or decrement is outside the int model");
                    boolean increment = tree.getKind() == Tree.Kind.PREFIX_INCREMENT
                        || tree.getKind() == Tree.Kind.POSTFIX_INCREMENT;
                    write(target, operation(increment ? "+" : "-", previous,
                                            new Value("I", "{\"literal\":1}")));
                }
                default -> throw new Unchecked("unsupported statement effect: " + tree.getKind());
            }
        }

        private void statement(TreePath path, int depth) {
            if (depth > MAX_DEPTH) throw new Unchecked("statement exceeds the effect depth bound");
            if (returned) throw new Unchecked("statement after a straight-line return");
            Tree tree = path.getLeaf();
            switch (tree.getKind()) {
                case BLOCK -> {
                    for (StatementTree child : ((BlockTree) tree).getStatements())
                        statement(new TreePath(path, child), depth + 1);
                }
                case EMPTY_STATEMENT -> { }
                case VARIABLE -> {
                    VariableTree declaration = (VariableTree) tree;
                    Element item = binding(path);
                    String targetType = descriptor(item.asType(), types, elements);
                    requireValueType(targetType);
                    ExpressionTree source = declaration.getInitializer();
                    Value initial = source == null ? null : value(new TreePath(path, source), 0);
                    if (initial != null && !targetType.equals(initial.descriptor()))
                        throw new Unchecked("local initialization conversion is not modeled");
                    locals.put(item, initial);
                }
                case EXPRESSION_STATEMENT -> expressionStatement(
                    new TreePath(path, ((ExpressionStatementTree) tree).getExpression())
                );
                case RETURN -> {
                    ExpressionTree result = ((ReturnTree) tree).getExpression();
                    String extra = "";
                    if (result != null) {
                        Value value = value(new TreePath(path, result), 0);
                        if (!returnType.equals(value.descriptor()))
                            throw new Unchecked("return conversion is outside the effect model");
                        extra = ",\"value\":" + value.json();
                    } else if (!returnType.equals("V")) {
                        throw new Unchecked("non-void method has no return value");
                    }
                    event("return", extra);
                    returned = true;
                }
                default -> throw new Unchecked("unsupported straight-line statement: " + tree.getKind());
            }
        }

        String run(TreePath path) {
            MethodTree method = (MethodTree) path.getLeaf();
            if (method.getBody() == null) throw new Unchecked("selected method has no recovered body");
            statement(new TreePath(path, method.getBody()), 0);
            if (!returned) {
                if (!returnType.equals("V")) throw new Unchecked("return value is not established");
                event("return", "");
            }
            return "[" + String.join(",", events) + "]";
        }
    }

    private static boolean contains(Diagnostic<?> error, CompilationUnitTree unit,
                                    Tree tree, SourcePositions positions) {
        long start = positions.getStartPosition(unit, tree);
        long end = positions.getEndPosition(unit, tree);
        return start >= 0 && end >= start && error.getPosition() >= start && error.getPosition() < end;
    }

    private static void evaluate(int side, Path source, Path scratch, String owner,
                                 List<String> signatures) throws Exception {
        JavaCompiler compiler = ToolProvider.getSystemJavaCompiler();
        var diagnostics = new DiagnosticCollector<JavaFileObject>();
        try (StandardJavaFileManager files = compiler.getStandardFileManager(diagnostics, Locale.ROOT, StandardCharsets.UTF_8)) {
            var options = List.of("-proc:none", "-implicit:none", "-source", "17", "-Xlint:none",
                "-classpath", scratch.toString(), "-sourcepath", scratch.toString());
            var task = (JavacTask) compiler.getTask(null, files, diagnostics, options, null,
                                                  files.getJavaFileObjects(source));
            CompilationUnitTree unit = task.parse().iterator().next();
            task.analyze();
            Trees trees = Trees.instance(task);
            SourcePositions positions = trees.getSourcePositions();
            List<TreePath> members = new ArrayList<>();
            Map<String, List<TreePath>> selected = new HashMap<>();
            TreePath root = new TreePath(unit);
            for (Tree declaration : unit.getTypeDecls()) {
                if (!(declaration instanceof ClassTree candidate)) continue;
                TreePath location = new TreePath(root, candidate);
                Element element = trees.getElement(location);
                if (!(element instanceof TypeElement type) ||
                    !descriptor(type.asType(), task.getTypes(), task.getElements()).equals(owner)) continue;
                for (Tree member : candidate.getMembers()) {
                    if (!(member instanceof MethodTree)) continue;
                    TreePath path = new TreePath(location, member);
                    members.add(path);
                    if (trees.getElement(path) instanceof ExecutableElement executable) {
                        try {
                            String signature = signature(executable, task.getTypes(), task.getElements());
                            selected.computeIfAbsent(signature, ignored -> new ArrayList<>()).add(path);
                        } catch (Unchecked ignored) {
                            // An unresolved overload cannot count as evidence for a different signature.
                        }
                    }
                }
            }
            for (int i = 0; i < signatures.size(); i++) {
                String start = side + "\t" + i + "\t";
                try {
                    var choices = selected.getOrDefault(signatures.get(i), List.of());
                    if (choices.size() != 1)
                        throw new Unchecked("modeled method owner and descriptor do not select one declaration");
                    var path = choices.get(0);
                    for (var error : diagnostics.getDiagnostics()) {
                        if (error.getKind() != Diagnostic.Kind.ERROR) continue;
                        boolean otherMethod = members.stream().anyMatch(
                            member -> member != path && contains(error, unit, member.getLeaf(), positions)
                        );
                        if (!otherMethod)
                            throw new Unchecked("source attribution unavailable: " + error.getMessage(Locale.ROOT));
                    }
                    var method = (ExecutableElement) trees.getElement(path);
                    String witness = new Witness(task, method).run(path);
                    System.out.println(start + "witness\t" + witness);
                } catch (Unchecked problem) {
                    System.out.println(start + "unchecked\t" + quote(problem.getMessage()));
                }
            }
        }
    }

    public static void main(String[] args) throws Exception {
        if (args.length < 5) throw new IllegalArgumentException("expected sources, scratch directory, owner and methods");
        List<String> signatures = List.of(args).subList(4, args.length);
        evaluate(0, Path.of(args[0]), Path.of(args[2]), args[3], signatures);
        evaluate(1, Path.of(args[1]), Path.of(args[2]), args[3], signatures);
    }
}

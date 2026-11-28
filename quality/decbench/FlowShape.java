/* Parse Java source and project its supported statement-level control flow. */
import com.sun.source.tree.*;
import com.sun.source.util.JavacTask;
import com.sun.source.util.SourcePositions;
import com.sun.source.util.Trees;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.IdentityHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.TreeMap;
import javax.tools.Diagnostic;
import javax.tools.DiagnosticCollector;
import javax.tools.JavaFileObject;
import javax.tools.ToolProvider;

final class FlowShape {
    private static final int MAX_NODES = 4096;
    private static final int MAX_DEPTH = 128;

    private static final class Unchecked extends RuntimeException {
        Unchecked(String detail) {
            super(detail);
        }
    }

    private record Selector(String name, int arity) {}
    private record Targets(Node leave, Node continueAt) {}
    private record Edge(String kind, Node target) {}

    private static final class Node {
        final String kind;
        final List<Edge> edges = new ArrayList<>();

        Node(String kind) {
            this.kind = kind;
        }

        Node to(String kind, Node target) {
            edges.add(new Edge(kind, target));
            return this;
        }
    }

    private static final class Graph {
        private int allocated;

        Node node(String kind) {
            if (++allocated > MAX_NODES) throw new Unchecked("method exceeds the CFG node bound");
            return new Node(kind);
        }

        Node graph(BlockTree body) {
            return node("entry").to("next", statement(body, node("exit"), new Targets(null, null), 0));
        }

        static boolean basic(StatementTree tree) {
            return switch (tree.getKind()) {
                case VARIABLE, EXPRESSION_STATEMENT, EMPTY_STATEMENT -> true;
                default -> false;
            };
        }

        Node sequence(List<? extends StatementTree> statements, Node after, Targets targets, int depth) {
            boolean effects = false;
            for (int index = statements.size() - 1; index >= 0; index--) {
                StatementTree tree = statements.get(index);
                if (basic(tree)) {
                    effects |= tree.getKind() != Tree.Kind.EMPTY_STATEMENT;
                } else {
                    if (effects) after = node("flow").to("next", after);
                    effects = false;
                    after = statement(tree, after, targets, depth);
                }
            }
            return effects ? node("flow").to("next", after) : after;
        }

        Node test(ExpressionTree condition, Node yes, Node no, int depth) {
            if (depth > MAX_DEPTH) throw new Unchecked("predicate exceeds the CFG depth bound");
            return switch (condition.getKind()) {
                case PARENTHESIZED -> test(((ParenthesizedTree) condition).getExpression(), yes, no, depth + 1);
                case LOGICAL_COMPLEMENT -> test(((UnaryTree) condition).getExpression(), no, yes, depth + 1);
                case CONDITIONAL_AND -> {
                    BinaryTree binary = (BinaryTree) condition;
                    Node right = test(binary.getRightOperand(), yes, no, depth + 1);
                    yield test(binary.getLeftOperand(), right, no, depth + 1);
                }
                case CONDITIONAL_OR -> {
                    BinaryTree binary = (BinaryTree) condition;
                    Node right = test(binary.getRightOperand(), yes, no, depth + 1);
                    yield test(binary.getLeftOperand(), yes, right, depth + 1);
                }
                case ERRONEOUS -> throw new Unchecked("predicate could not be parsed");
                default -> node("test").to("arm", yes).to("arm", no);
            };
        }

        Node statement(StatementTree tree, Node after, Targets targets, int depth) {
            if (depth > MAX_DEPTH) throw new Unchecked("method exceeds the CFG depth bound");
            if (tree == null) return after;
            return switch (tree.getKind()) {
                case BLOCK -> sequence(((BlockTree) tree).getStatements(), after, targets, depth + 1);
                case RETURN -> node("return");
                case THROW -> node("throw");
                case EMPTY_STATEMENT -> after;
                case VARIABLE, EXPRESSION_STATEMENT -> node("flow").to("next", after);
                case IF -> {
                    IfTree branch = (IfTree) tree;
                    Node yes = statement(branch.getThenStatement(), after, targets, depth + 1);
                    Node no = statement(branch.getElseStatement(), after, targets, depth + 1);
                    yield test(branch.getCondition(), yes, no, depth + 1);
                }
                case WHILE_LOOP -> {
                    WhileLoopTree loop = (WhileLoopTree) tree;
                    Node head = node("while");
                    Node body = statement(loop.getStatement(), head, new Targets(after, head), depth + 1);
                    yield head.to("condition", test(loop.getCondition(), body, after, depth + 1));
                }
                case DO_WHILE_LOOP -> {
                    DoWhileLoopTree loop = (DoWhileLoopTree) tree;
                    Node entry = node("do");
                    Node continueAt = node("do-condition");
                    Node body = statement(loop.getStatement(), continueAt,
                        new Targets(after, continueAt), depth + 1);
                    continueAt.to("condition", test(loop.getCondition(), entry, after, depth + 1));
                    yield entry.to("body", body);
                }
                case ENHANCED_FOR_LOOP -> {
                    EnhancedForLoopTree loop = (EnhancedForLoopTree) tree;
                    Node head = node("enhanced-for");
                    Node body = statement(loop.getStatement(), head, new Targets(after, head), depth + 1);
                    yield head.to("next-element", body).to("exhausted", after);
                }
                case FOR_LOOP -> {
                    ForLoopTree loop = (ForLoopTree) tree;
                    Node head = node("for");
                    Node update = sequence(loop.getUpdate(), head, targets, depth + 1);
                    Node body = statement(loop.getStatement(), update, new Targets(after, update), depth + 1);
                    if (loop.getCondition() == null) head.to("body", body);
                    else head.to("condition", test(loop.getCondition(), body, after, depth + 1));
                    yield sequence(loop.getInitializer(), head, targets, depth + 1);
                }
                case SWITCH -> switchGraph((SwitchTree) tree, after, targets, depth + 1);
                case BREAK -> {
                    if (((BreakTree) tree).getLabel() != null || targets.leave() == null)
                        throw new Unchecked("break target cannot be established");
                    yield targets.leave();
                }
                case CONTINUE -> {
                    if (((ContinueTree) tree).getLabel() != null || targets.continueAt() == null)
                        throw new Unchecked("continue target cannot be established");
                    yield targets.continueAt();
                }
                default -> throw new Unchecked("unsupported statement control: " + tree.getKind());
            };
        }

        Node switchGraph(SwitchTree tree, Node after, Targets targets, int depth) {
            Node dispatch = node("switch");
            Map<String, Node> arms = new TreeMap<>();
            Node tail = after;
            List<? extends CaseTree> cases = tree.getCases();
            Targets switchTargets = new Targets(after, targets.continueAt());
            for (int index = cases.size() - 1; index >= 0; index--) {
                CaseTree arm = cases.get(index);
                if (arm.getCaseKind() == CaseTree.CaseKind.STATEMENT) {
                    tail = sequence(arm.getStatements(), tail, switchTargets, depth);
                } else if (arm.getBody() instanceof StatementTree body) {
                    tail = statement(body, after, switchTargets, depth);
                } else {
                    throw new Unchecked("unsupported switch arm body");
                }

                List<? extends ExpressionTree> labels = arm.getExpressions();
                if (labels.isEmpty()) {
                    addArm(arms, "default", tail);
                } else {
                    for (ExpressionTree label : labels)
                        addArm(arms, "case:" + constantLabel(label), tail);
                }
            }
            arms.putIfAbsent("default", after);
            arms.forEach(dispatch::to);
            return dispatch;
        }

        static void addArm(Map<String, Node> arms, String label, Node target) {
            if (arms.size() >= MAX_NODES) throw new Unchecked("switch exceeds the CFG case bound");
            if (arms.putIfAbsent(label, target) != null)
                throw new Unchecked("switch contains repeated case labels");
        }

        static String constantLabel(ExpressionTree expression) {
            if (expression instanceof LiteralTree literal)
                return literal.getKind().name() + ":" + String.valueOf(literal.getValue());
            if (expression instanceof UnaryTree unary && unary.getKind() == Tree.Kind.UNARY_MINUS
                && unary.getExpression() instanceof LiteralTree literal)
                return literal.getKind().name() + ":-" + literal.getValue();
            throw new Unchecked("switch case is not an established literal");
        }
    }

    private static String quote(String text) {
        var result = new StringBuilder("\"");
        for (int i = 0; i < text.length(); i++) {
            char value = text.charAt(i);
            if (value == '"' || value == '\\') {
                result.append('\\').append(value);
            } else if (value < 32 || Character.isSurrogate(value)) {
                result.append(String.format(Locale.ROOT, "\\u%04x", (int) value));
            } else {
                result.append(value);
            }
        }
        return result.append('"').toString();
    }

    private static Map<Node, Integer> incoming(Node entry) {
        var counts = new IdentityHashMap<Node, Integer>();
        var pending = new ArrayList<Node>();
        pending.add(entry);
        counts.put(entry, 0);
        for (int index = 0; index < pending.size(); index++) {
            for (Edge edge : pending.get(index).edges) {
                if (!counts.containsKey(edge.target())) pending.add(edge.target());
                counts.merge(edge.target(), 1, Integer::sum);
            }
        }
        return counts;
    }

    private static List<Edge> combineLinearFlow(Node node, Map<Node, Integer> incoming) {
        if (!node.kind.equals("flow")) return node.edges;
        Edge edge = node.edges.get(0);
        while (edge.target().kind.equals("flow") && incoming.get(edge.target()) == 1) {
            // No branch enters the second block. The lexical boundary is irrelevant here.
            edge = edge.target().edges.get(0);
        }
        return List.of(edge);
    }

    private static String encode(Node entry) {
        var incoming = incoming(entry);
        var identifiers = new IdentityHashMap<Node, Integer>();
        var order = new ArrayList<Node>();
        identifiers.put(entry, 0);
        order.add(entry);
        var result = new StringBuilder("[");
        for (int index = 0; index < order.size(); index++) {
            Node node = order.get(index);
            if (index > 0) result.append(',');
            result.append("{\"kind\":").append(quote(node.kind)).append(",\"edges\":[");
            var outgoing = combineLinearFlow(node, incoming);
            for (int i = 0; i < outgoing.size(); i++) {
                Edge edge = outgoing.get(i);
                Integer target = identifiers.get(edge.target());
                if (target == null) {
                    target = order.size();
                    identifiers.put(edge.target(), target);
                    order.add(edge.target());
                }
                if (i > 0) result.append(',');
                result.append('[').append(quote(edge.kind())).append(',').append(target).append(']');
            }
            result.append("]}");
        }
        return result.append(']').toString();
    }

    private static ClassTree findClass(List<? extends Tree> declarations, String name) {
        for (Tree declaration : declarations)
            if (declaration instanceof ClassTree type && type.getSimpleName().contentEquals(name))
                return type;
        return null;
    }

    private static ClassTree selectClass(CompilationUnitTree unit, String owner) {
        int slash = owner.lastIndexOf('/');
        String expectedPackage = owner.substring(0, slash + 1).replace('/', '.');
        String actualPackage = unit.getPackageName() == null ? "" : unit.getPackageName() + ".";
        if (!expectedPackage.equals(actualPackage)) return null;
        String name = owner.substring(slash + 1);
        ClassTree type = findClass(unit.getTypeDecls(), name);
        if (type != null) return type;
        String[] parts = name.split("\\$");
        type = findClass(unit.getTypeDecls(), parts[0]);
        for (int i = 1; type != null && i < parts.length; i++)
            type = findClass(type.getMembers(), parts[i]);
        return type;
    }

    private static boolean contains(CompilationUnitTree unit, SourcePositions positions, Tree tree, long point) {
        long start = positions.getStartPosition(unit, tree);
        long end = positions.getEndPosition(unit, tree);
        return point >= 0 && start >= 0 && start <= point && (end < 0 || point < end);
    }

    private static void output(int side, int index, String value) {
        System.out.println(side + "\t" + index + "\tgraph\t" + value);
    }

    private static void unavailable(int side, int index, String detail) {
        System.out.println(side + "\t" + index + "\tunchecked\t" + quote(detail));
    }

    private static void scan(int side, Path path, String owner, List<Selector> selectors) throws Exception {
        var diagnostics = new DiagnosticCollector<JavaFileObject>();
        var javac = ToolProvider.getSystemJavaCompiler();
        try (var files = javac.getStandardFileManager(diagnostics, Locale.ROOT, StandardCharsets.UTF_8)) {
            JavacTask task = (JavacTask) javac.getTask(null, files, diagnostics, List.of("-proc:none", "--source", "17"),
                null, files.getJavaFileObjects(path));
            CompilationUnitTree unit = task.parse().iterator().next();
            ClassTree type = selectClass(unit, owner);
            if (type == null) {
                for (int index = 0; index < selectors.size(); index++)
                    unavailable(side, index, "source class does not identify the selected original owner");
                return;
            }
            var methods = type.getMembers().stream().filter(MethodTree.class::isInstance)
                .map(MethodTree.class::cast).toList();
            var positions = Trees.instance(task).getSourcePositions();
            var syntaxErrors = diagnostics.getDiagnostics().stream()
                .filter(diagnostic -> diagnostic.getKind() == Diagnostic.Kind.ERROR).toList();
            for (int index = 0; index < selectors.size(); index++) {
                Selector selector = selectors.get(index);
                var matches = methods.stream().filter(method ->
                    method.getName().contentEquals(selector.name())
                    && method.getParameters().size() == selector.arity()).toList();
                if (matches.size() != 1 || matches.get(0).getBody() == null) {
                    unavailable(side, index, "method body is missing or its name and arity are ambiguous");
                    continue;
                }
                MethodTree method = matches.get(0);
                var syntax = syntaxErrors.stream().filter(error ->
                    contains(unit, positions, method, error.getPosition())
                    || methods.stream().noneMatch(other -> contains(unit, positions, other, error.getPosition()))
                ).findFirst();
                if (syntax.isPresent()) {
                    unavailable(side, index, "source syntax could not be parsed: " + syntax.get().getCode());
                    continue;
                }
                try {
                    output(side, index, encode(new Graph().graph(method.getBody())));
                } catch (Unchecked unsupported) {
                    unavailable(side, index, unsupported.getMessage());
                } catch (StackOverflowError limit) {
                    unavailable(side, index, "method exceeds the parser nesting bound");
                }
            }
        }
    }

    public static void main(String[] args) throws Exception {
        var selectors = new ArrayList<Selector>();
        for (int offset = 3; offset < args.length; offset += 2)
            selectors.add(new Selector(args[offset], Integer.parseInt(args[offset + 1])));
        for (int side = 0; side < 2; side++) {
            try {
                scan(side, Path.of(args[side]), args[2], selectors);
            } catch (StackOverflowError limit) {
                for (int index = 0; index < selectors.size(); index++)
                    unavailable(side, index, "source parser exceeded the nesting bound");
            }
        }
    }
}

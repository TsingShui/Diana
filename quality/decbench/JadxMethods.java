import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import java.io.BufferedWriter;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.IdentityHashMap;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import jadx.api.ICodeInfo;
import jadx.api.JadxArgs;
import jadx.api.JadxDecompiler;
import jadx.api.metadata.ICodeAnnotation;
import jadx.api.metadata.annotations.NodeDeclareRef;
import jadx.api.plugins.input.data.AccessFlags;
import jadx.api.plugins.input.data.IClassData;
import jadx.api.utils.CodeUtils;
import jadx.core.dex.attributes.AFlag;
import jadx.core.dex.attributes.AType;
import jadx.core.dex.attributes.nodes.JadxCommentsAttr;
import jadx.core.dex.attributes.nodes.JadxError;
import jadx.core.dex.attributes.nodes.MethodReplaceAttr;
import jadx.core.dex.nodes.ClassNode;
import jadx.core.dex.nodes.MethodNode;
import jadx.core.utils.Utils;

/** Publishes slices of JADX's own output using original DEX identities and code metadata. */
public final class JadxMethods {
    private record PhysicalClass(String owner, String input_name, int offset) {}
    private record Request(String at, String member, PhysicalClass physical) {}
    private record Span(int start_utf16, int definition_utf16, int end_utf16) {}
    private record MethodResult(String at, String member, boolean identified, String status, boolean complete,
                                boolean has_body, String text, Span source_span, List<String> diagnostics) {}
    private record ClassResult(String class_at, String text) {}
    private record ToolResult(String version) {}

    private static final Gson JSON = new GsonBuilder().serializeNulls().disableHtmlEscaping().create();

    private static void write(BufferedWriter output, Object row) throws Exception {
        output.write(JSON.toJson(row));
        output.newLine();
        output.flush();
    }

    private static MethodResult unavailable(Request request, boolean identified, String reason) {
        return unavailable(request, identified, new ArrayList<>(), reason);
    }

    private static MethodResult unavailable(Request request, boolean identified, List<String> notes, String reason) {
        notes.add(reason);
        return new MethodResult(request.at, request.member, identified, "UNAVAILABLE", false, false,
                null, null, notes);
    }

    // JADX MethodGen writes the original implementation's body but may attach
    // the declaration to a different node through METHOD_REPLACE.
    private static MethodNode declarationNode(MethodNode method) {
        MethodReplaceAttr replace = method.get(AType.METHOD_REPLACE);
        return replace == null || method.isConstructor() || method.getMethodInfo().isClassInit() ? method : replace.getReplaceMth();
    }

    private static Map<MethodNode, List<MethodNode>> declarationAliases(ClassNode top) {
        Set<ClassNode> enclosing = Collections.newSetFromMap(new IdentityHashMap<>());
        enclosing.add(top);
        top.getInnerAndInlinedClassesRecursive(enclosing);
        Map<MethodNode, List<MethodNode>> aliases = new IdentityHashMap<>();
        for (ClassNode cls : enclosing) {
            for (MethodNode body : cls.getMethods()) {
                MethodNode declaration = declarationNode(body);
                if (declaration != body && !body.contains(AFlag.DONT_GENERATE)) {
                    aliases.computeIfAbsent(declaration, unused -> new ArrayList<>()).add(body);
                }
            }
        }
        return aliases;
    }

    // MethodGen.dumpInstructions can emit this stub without retaining an error
    // attribute (for example, AUTO codegen with an absent region).
    private static boolean containsFallbackDump(String text, MethodNode method) {
        if (!text.contains("Method not decompiled: ")) return false;
        String subject = method.getParentClass().getClassInfo().getAliasFullName() + "." + method.getAlias()
                + "(" + Utils.listToString(method.getMethodInfo().getArgumentsTypes()) + "):"
                + method.getMethodInfo().getReturnType();
        String stub = "throw new UnsupportedOperationException(\"Method not decompiled: " + subject + "\");";
        List<String> lines = text.lines().map(String::strip).toList();
        int closing = lines.size() - 1;
        return closing >= 5
                && lines.get(0).endsWith("{") && lines.get(1).equals("/*")
                && lines.get(closing).equals("}") && lines.get(closing - 1).equals(stub)
                && lines.get(closing - 2).equals("*/")
                && lines.subList(2, closing - 2).stream().noneMatch(line -> line.contains("*/"));
    }

    private static MethodResult method(Request request, MethodNode method, ICodeInfo code,
                                       Map<MethodNode, List<MethodNode>> aliases) {
        List<String> notes = new ArrayList<>();
        List<JadxError> errors = method.getAll(AType.JADX_ERROR);
        for (JadxError error : errors) notes.add(error.toString());
        boolean inconsistent = method.contains(AFlag.INCONSISTENT_CODE);
        if (inconsistent) notes.add("JADX marked this method inconsistent");
        JadxCommentsAttr comments = method.get(AType.JADX_COMMENTS);
        if (comments != null) notes.addAll(comments.formatAndFilter(method.getCommentsLevel()));
        if (method.contains(AFlag.DONT_GENERATE)) {
            return unavailable(request, true, notes, "JADX suppressed this method during code generation");
        }
        if (aliases.containsKey(method)) {
            return unavailable(request, true, notes, "JADX assigned this declaration node to another method body");
        }
        MethodNode declarationNode = declarationNode(method);
        if (declarationNode != method) {
            List<MethodNode> bodies = aliases.getOrDefault(declarationNode, List.of());
            int bridgeFlags = AccessFlags.BRIDGE | AccessFlags.SYNTHETIC;
            if (bodies.size() != 1 || bodies.get(0) != method
                    || method.getParentClass() != declarationNode.getParentClass()
                    || (method.getRawAccessFlags() & AccessFlags.BRIDGE) != 0
                    || (declarationNode.getRawAccessFlags() & bridgeFlags) != bridgeFlags
                    || !method.getName().equals(declarationNode.getName())) {
                return unavailable(request, true, notes, "JADX declaration alias is not a unique same-class synthetic bridge");
            }
            long occurrences = code.getCodeMetadata().getAsMap().values().stream()
                    .filter(annotation -> annotation instanceof NodeDeclareRef ref && ref.getNode() == declarationNode)
                    .limit(2)
                    .count();
            if (occurrences != 1) {
                return unavailable(request, true, notes, "JADX bridge has no unique source declaration");
            }
            notes.add("JADX attaches this implementation's declaration to its synthetic bridge "
                    + declarationNode.getMethodInfo().getShortId());
        }
        String text = code.getCodeStr();
        int definition = declarationNode.getDefPosition();
        if (!code.hasMetadata() || definition <= 0 || definition >= text.length()) {
            return unavailable(request, true, notes, "JADX returned no method declaration position");
        }
        ICodeAnnotation declaration = code.getCodeMetadata().getAt(definition);
        if (!(declaration instanceof NodeDeclareRef ref) || ref.getNode() != declarationNode) {
            return unavailable(request, true, notes, "JADX source position does not identify this method declaration");
        }
        boolean noBody = method.getCodeReader() == null
                && (method.getRawAccessFlags() & (AccessFlags.ABSTRACT | AccessFlags.NATIVE)) != 0;
        int end;
        if (noBody) {
            end = CodeUtils.getLineEndForPos(text, definition);
            if (!text.substring(definition, end).stripTrailing().endsWith(";")) {
                return unavailable(request, true, notes, "JADX declaration has no verified single-line boundary");
            }
        } else {
            end = CodeUtils.getMethodEnd(declarationNode, code);
            if (end <= definition || end > text.length()) {
                return unavailable(request, true, notes, "JADX returned no verified end position for the method");
            }
        }
        int start = CodeUtils.getLineStartForPos(text, definition);
        String recovered = text.substring(start, end);
        boolean fallback = containsFallbackDump(recovered, method);
        if (fallback) notes.add("JADX emitted an instruction dump and a synthetic decompilation failure stub");
        return new MethodResult(request.at, request.member, true, "AVAILABLE",
                errors.isEmpty() && !inconsistent && !fallback,
                !noBody && !fallback, recovered, new Span(start, definition, end), notes);
    }

    private static void publish(JadxDecompiler jadx, Request[] requests, BufferedWriter output) throws Exception {
        Map<PhysicalClass, List<Request>> selection = new LinkedHashMap<>();
        for (Request request : requests) {
            selection.computeIfAbsent(request.physical, unused -> new ArrayList<>()).add(request);
        }
        Map<PhysicalClass, List<ClassNode>> physicalClasses = new HashMap<>();
        for (ClassNode cls : jadx.getRoot().getClasses()) {
            IClassData data = cls.getClsData();
            if (data == null) continue;
            PhysicalClass identity = new PhysicalClass(data.getType(), data.getInputFileName(), data.getInputFileOffset());
            if (selection.containsKey(identity)) {
                physicalClasses.computeIfAbsent(identity, unused -> new ArrayList<>()).add(cls);
            }
        }
        Map<Request, MethodNode> methods = new HashMap<>();
        for (Map.Entry<PhysicalClass, List<Request>> selected : selection.entrySet()) {
            List<ClassNode> matches = physicalClasses.getOrDefault(selected.getKey(), List.of());
            if (matches.size() != 1) continue;
            ClassNode cls = matches.get(0);
            for (Request request : selected.getValue()) {
                String signature = request.member.substring(request.member.indexOf("->") + 2);
                List<MethodNode> matching = cls.getMethods().stream()
                        .filter(method -> method.getMethodInfo().getShortId().equals(signature))
                        .toList();
                if (matching.size() == 1) methods.put(request, matching.get(0));
            }
        }
        for (Map.Entry<PhysicalClass, List<Request>> selected : selection.entrySet()) {
            List<Request> wanted = selected.getValue();
            List<ClassNode> matches = physicalClasses.getOrDefault(selected.getKey(), List.of());
            if (matches.size() != 1) {
                for (Request request : wanted) {
                    write(output, unavailable(request, false, "JADX did not load a unique class at the original DEX location"));
                }
                continue;
            }
            publishClass(matches.get(0), wanted, methods, output);
        }
    }

    private static void publishClass(ClassNode cls, List<Request> requests, Map<Request, MethodNode> methods,
                                     BufferedWriter output) throws Exception {
        ICodeInfo code = null;
        Map<MethodNode, List<MethodNode>> aliases = Map.of();
        String generationError = null;
        for (Request request : requests) {
            MethodNode selected = methods.get(request);
            if (selected == null) {
                write(output, unavailable(request, false, "JADX did not retain a unique original method descriptor"));
                continue;
            }
            if (code == null && generationError == null) {
                try {
                    ClassNode top = cls.getTopParentClass();
                    ICodeInfo rendered = top.getCode();
                    aliases = declarationAliases(top);
                    code = rendered;
                } catch (RuntimeException error) {
                    generationError = error.toString();
                }
                if (code != null && !code.getCodeStr().isEmpty()) {
                    String classAt = request.at.substring(0, request.at.lastIndexOf(":m"));
                    write(output, new ClassResult(classAt, code.getCodeStr()));
                }
            }
            if (code == null) {
                write(output, unavailable(request, true, "JADX class generation failed: " + generationError));
            } else {
                MethodResult row;
                try {
                    row = method(request, selected, code, aliases);
                } catch (RuntimeException error) {
                    row = unavailable(request, true, "JADX method metadata failed: " + error);
                }
                write(output, row);
            }
        }
    }

    public static void main(String[] argv) throws Exception {
        Path artifact = Path.of(argv[0]);
        Request[] requests = JSON.fromJson(Files.readString(Path.of(argv[1])), Request[].class);
        try (BufferedWriter output = Files.newBufferedWriter(Path.of(argv[2]))) {
            write(output, new ToolResult(JadxDecompiler.getVersion()));
            JadxArgs args = new JadxArgs();
            args.setInputFile(artifact.toFile());
            args.setOutDir(Path.of(argv[2]).getParent().resolve("unused-output").toFile());
            args.setThreadsCount(1);
            args.setSkipResources(true);
            args.setSkipFilesSave(true);
            try (JadxDecompiler jadx = new JadxDecompiler(args)) {
                jadx.load();
                publish(jadx, requests, output);
            }
        }
    }
}

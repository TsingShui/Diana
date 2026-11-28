---
name: diana
description: >-
  Inspect, decompile, compare, or export code from Android APKs and DEX files or
  JVM JAR archives and class files. Use Diana to locate classes or methods,
  inspect references, recover Java-like source, or compare bytecode across builds.
---

# Diana

Start with the narrowest command that answers the question:

| What you know or need | First command |
| --- | --- |
| Exact method descriptor | `diana get-method INPUT 'Lcom/example/Main;->run()V'` |
| Class name and you want its source | `diana get-class INPUT com.example.Main` |
| Class name but not the method descriptor | `diana methods INPUT --class com.example.Main` |
| Package structure or a partial class name | `diana tree INPUT` or `diana classes INPUT --filter NAME` |
| Where a method is referenced | `diana xref INPUT method 'Lcom/example/Main;->run()V' --exact` |
| Differences between two builds | `diana diff OLD NEW` |
| A whole source tree | `diana export INPUT -o NEW_DIRECTORY` |

Use the full descriptor to distinguish overloaded methods. Quote descriptors so
the shell does not interpret punctuation. Use `--format json` for structured
results where the command supports it. Run `diana <command> --help` only when
you need syntax or flags not shown here.

APK code queries inspect root DEX files. For a DEX stored elsewhere in an APK,
find it with `entries`, extract it with `extract`, then query the standalone
DEX. Diana does not decompile native ELF libraries or run apps.

Recovered output is Java-like, not necessarily compilable. Keep diagnostics
with the relevant output. An export can produce useful files and
`export-report.jsonl` even when recovery issues cause exit status 1.

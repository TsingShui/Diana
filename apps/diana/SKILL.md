---
name: diana
description: >-
  Inspect, decompile, compare, and export Android APKs and DEX files, JVM JARs
  and class files, and JVM projects. Use Diana to find original definitions,
  strings, references, and call outlines, recover Java-like source, or inspect
  manifests, archive entries, and resources.
---

# Diana

Start with the smallest query that answers the question.

| Task | Recommended starting point |
| --- | --- |
| Browse packages or find definitions | `tree` for packages, `classes` for class names, `methods` and `fields` for members |
| Find a literal and its uses | `strings` to find the value, then `xref string` to find references |
| Read code or follow its callees | `get-method` for one method, `get-class` for a class, `outline-method` or `outline-class` for structure and calls |
| Find callers or type relationships | `xref` with the relevant reference kind or relationship mode |
| Inspect archive contents | `entries` for physical files, `manifest` for the manifest, `resources` for resource queries, `extract` for a selected file |
| Compare builds or save a source tree | `diff OLD NEW` for method comparisons, `export INPUT -o NEW_DIRECTORY` for bulk recovery |

## Select an original definition

Copy the complete `at` value from a matching row, quote it, and reuse it with
the same input. Use a class coordinate from `classes` with `get-class` or
`outline-class`. Use a method coordinate from `methods` with `get-method`
or `outline-method`. Preserve any descriptor suffix printed with the coordinate.

For example, on a DEX containing `fixture.SyncBlock`, look up the method and
use the coordinate returned by that lookup.

```sh
diana methods sample.dex --class fixture.SyncBlock --filter add --exact
diana get-method sample.dex 'e0:l0:c0:m2'
```

On a standalone JVM class the same workflow can return a coordinate such as
`j0:m1`.

```sh
diana methods SyncBlock.class --class fixture.SyncBlock --filter add --exact
diana get-method SyncBlock.class 'j0:m1'
```

JAR coordinates can include a nested archive path such as `j6/j4:m2`. A JVM
project can include an artifact prefix such as `a1/j4:m2`. For several archives
or a specific Java release view, pass a `.diana-jvm.json` descriptor to JVM
analysis commands. Its `archives` array lists paths relative to the descriptor
directory. An optional `java_version` selects the target release.

When the definition is unique, pass a dotted class name such as
`com.example.Main` or a complete method reference such as
`'Lcom/example/Main;->run()V'` directly. The complete prototype distinguishes
overloads. If several definitions match, list them and pick the original
coordinate for the intended definition.

## Browse and search

```sh
diana tree app.apk --prefix com.example -L 2 --classes
diana classes app.apk --filter MainActivity --substring
diana fields app.apk --class com.example.MainActivity
diana strings app.apk --filter 'https?://' --regex
```

Use the default ranked fuzzy matching for a partial name. Use `--substring`
for a contiguous fragment, `--exact` for a complete case-sensitive value, or
`--regex` for a pattern. On list commands, pass the search term with `--filter`.
`classes` matches descriptors, member lists match names, and `strings` matches
literal values. Use `--class` to narrow a member search to one owner. For direct
hierarchy filters on class definitions, see `classes --help`.

`classes`, `methods`, `fields`, `strings`, and `xref` show up to 50 rows by
default. Check for truncation before treating the displayed rows as exhaustive.
Narrow the query first. Use `--limit N` for a larger sample or `--no-limit -o FILE`
when you need every match.

## Read code and follow references

Prefer `get-method` when one body answers the question. Use `get-class` when you
need surrounding declarations and sibling methods. For control structure and
calls, use an outline. Start with the selected method or class, then add
`--call-depth 1` to inspect one level of callees.

```sh
diana outline-class sample.dex 'e0:l0:c0'
diana outline-method sample.dex 'e0:l0:c0:m2' --call-depth 1
diana xref sample.dex method 'Lfixture/SyncBlock;->add(I)I' --exact --transitive 2
diana xref app.apk string 'https://example.invalid/' --exact
```

`xref method --transitive N` follows callers upstream. For a direct reference
query, use `xref INPUT KIND TARGET` with `string`, `type`, `method`, `field`, or
`resource`. Use `--exact` with a complete value or signature. DEX member
coordinates used as reference targets include the `::` signature suffix.
Read `context` for the referring location, `match` for the
target, and `use` for the kind of evidence. For hierarchy queries, use `impl`
or `extend` and add `--recursive` for indirect relationships. Use
`xref method --relation implementations` or `--relation overrides` to inspect
declaration relationships. Outline links and reference hits describe evidence
in the input. Preserve unresolved dispatch and other local uncertainties.

## Inspect archives, manifests, and resources

Code queries on APKs read root `classes.dex` and `classes<N>.dex` entries. If
`entries` shows a DEX elsewhere in the archive, extract that exact entry and
use the resulting standalone DEX as the input.

```sh
diana entries app.apk
diana extract app.apk 'assets/secondary.dex' -o secondary.dex
diana classes secondary.dex
```

Use `manifest` to read `AndroidManifest.xml` from an APK or
`META-INF/MANIFEST.MF` from a JAR. Use `resources` to find Android resource
configurations and values. For example, run
`diana resources app.apk --filter app_name --substring`. On a JAR or JVM
project, `resources` lists original non-class entries. With `extract`,
use `--entry-index` with the entry name if it repeats, or use `--at` with
a JVM archive coordinate. Use `--raw` to preserve original binary XML bytes.
Use `entries` and `extract` for native libraries stored in the archive.

## Compare builds and export output

```sh
diana diff old.apk new.apk --format json -o changes.json
diana export app.apk -o recovered-app
```

Compare two DEX-side inputs or two JVM-side inputs. Review the pairing evidence,
changes, and unresolved groups. To inspect a pair, give the old `at` to
`get-method` on the old input and the new `at` to `get-method` on the new input.
Export sources and available archive resources into a new directory with an
existing parent. Review `export-report.jsonl` when present. If recovery issues
cause exit status 1, inspect the report and the files the command recovered.

## Read results and check the installed tool

Use text for inspection and `--format json` for structured consumption where
the command supports it. Use `-o FILE` to save large results. Keep original
coordinates and diagnostics with the relevant result. Treat recovered source
as Java-like output with possible local gaps. Check coverage and warnings
before concluding a definition or reference is absent. A single-object lookup
can return 3 for a missing selection or 4 for an ambiguous one. On status 1,
check diagnostics and any delivered output before retrying.

Run `diana --version` to check the installed version and
`diana help COMMAND` or `diana COMMAND --help` for command-specific syntax.
Run `diana skill` to print this guide. Use `diana skill -i` for a first-time
installation. Review the printed guide before replacing an installed copy.

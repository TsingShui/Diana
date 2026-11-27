# Explicit JVM project archives

The fixture uses the independent class assembler in
[physical-inventory](../physical-inventory/README.md). Python 3 writes the
archives and project descriptors to an ignored output directory.

- Spring Boot library path `BOOT-INF/lib/dependency.jar` holds
  `Lfixture/project/Library;->probe()I` at `j1/j0:m0`. It returns 37. The
  malformed library at `j3` leaves that declaration accessible. A JAR in
  `assets/` lies outside the library layout.
- The WAR contains a normal library and a provided library at
  `j0/j0` and `j1/j0` respectively. The former returns 37 and the latter
  returns 47. The first is deflated in the WAR and the other is stored.
- In the project, `Lfixture/project/Caller;->probe()I` at `j0:m0`
  invokes `Library.probe` at byte zero and returns the result. The declaration
  belongs to project artifact one at `a1/j0/j0:m0`. Missing, non-JVM,
  nonportable and escaping-symlink inputs retain their own ordinals.
- The multi-release archive has body variants returning 8, 9, 17 and 21.
  A mixed-case main attribute enables the overlay.
  Releases 8, 11, 17 and 25 select original entries `j1`, `j3`, `j4`
  and `j5` respectively. Explicit physical lookups read any variant.
  A leading-zero version and a versioned entry in `META-INF/` are hidden
  in the configured runtime view.
- A named manifest section cannot enable the overlay. A corrupt manifest
  reports its entry and retains base class declarations. Two physical
  copies at the effective version remain ambiguous by class name.
- A second valid manifest folds its main attribute value over two records.
  The configured release view uses the joined value `true`. OpenJDK 21's
  manifest reader also parses `true`. Its runtime `JarFile.isMultiRelease()`
  reports `false` for the folded recording.

```sh
python3 quality/fixtures/jvm/project-archives/prepare.py project .cache/fixtures/jvm-project
python3 quality/fixtures/jvm/project-archives/prepare.py multi-release .cache/fixtures/jvm-release
cargo nextest run --locked -p diana --test jvm_project_test
```

The ZIP writer independently preserves the record ordering. Each body directly
returns its authored literal or returns the value from the recorded invocation.
These behaviors determine the CLI assertions. OpenJDK 21's `JarFile` lookup
confirmed the four effective entry names for the ordinary mixed-case manifest.
See the [JAR File Specification](https://docs.oracle.com/en/java/javase/26/docs/specs/jar/jar.html).

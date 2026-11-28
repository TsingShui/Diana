# Semantic review

The original literal getters return metadata-matched constants without
an active field access. The reference and reconstructed classes both
preserve those observations, including the String reference identity.

When the original consumer first reads either blank final, it runs the
provider initialization. The two stores complete before the begin and
completion calls. A second successful read uses that already initialized
state. Under a completion failure, the first read throws with the modeled
exception as its cause and a second read fails against the erroneous
provider without calling the hooks again.

In the 0.4.13-final source reconstruction, both blank finals appear
as declaration-time static final constants. The paired consumer still
spells field references. The Java compiler inlines those references,
so the recovered consumer reads the expected values without the original
hook events. Both expected exception transitions disappear in the
failing-initializer scenarios. The field text values and String identity
alone cannot distinguish those executions.

The 0.4.14 provider declares both runtime fields without an initializer
and assigns them at the start of the static block. Both first reads now
trigger the ordered stores and hooks. When the begin hook fails, the
same throwable becomes the initializer error's cause and later active
use observes failed class initialization. The literal-only control still
returns the interned String and the encoded int without a hook event.

The independent original raw replay uses the recorded field values,
original instruction order and modeled hook faults. Java execution
observes actual class initialization and its errors in fresh processes.

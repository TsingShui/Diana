# Semantic review

The original `HierarchyAlias.PREPARED_LONG:J` field reference resolves
to the prepared-only declaration in `DefaultHierarchyOwner`. The first
inherited read runs the root before returning zero while the alias
remains uninitialized. The later active alias read runs its initializer
and observes the ancestor zero through the same symbolic field read.
Starting with a direct null read or an active alias read preserves the
order appropriate to the corresponding declaring class.

In 0.4.16 the exact recovery adds a ConstantValue to the prepared long
after recompilation. The inherited accessor becomes `lconst_0`,
so the root starts only at the later direct null access in the
inherited-first scenario. Under a root fault the inherited accessor
returns before the exception and again after the declaring class is
erroneous. Both effects contradict the independent original trace.

The repaired exact publication recompiles to an active `getstatic` for
the inherited read. The first access now observes the original root
execution or its exact failure cause. A later attempt observes the
erroneous declaring class. The alias-only fault still leaves the
declaring class healthy, as shown by the next inherited and direct
reads, while the alias's own retry reports a prior failure. The
recompiled literal controls still bypass both initializers.

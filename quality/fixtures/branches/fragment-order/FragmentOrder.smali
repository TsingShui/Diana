.class public final Lfixture/FragmentOrder;
.super Ljava/lang/Object;

.method public static branchLoop(I)I
    .registers 3
    const/4 v0, 0x0
    const/4 v1, 0x3
    if-eqz p0, :early
    :repeat
    if-ge v0, v1, :done
    add-int/lit8 v0, v0, 0x1
    goto :repeat
    :done
    return v0
    :early
    const/4 v0, 0x7
    return v0
.end method

.method public static irreducible(I)I
    .registers 3
    const/4 v0, 0x0
    const/4 v1, 0x4
    if-eqz p0, :a
    goto :b
    :a
    add-int/lit8 v0, v0, 0x1
    if-lt v0, v1, :b
    goto :done
    :b
    add-int/lit8 v0, v0, 0x2
    if-lt v0, v1, :a
    :done
    return v0
.end method

.method public static selfCycle()V
    .registers 0
    :repeat
    nop
    goto :repeat
.end method

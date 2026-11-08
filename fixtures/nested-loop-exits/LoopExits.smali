.class public final Lfixture/LoopExits;
.super Ljava/lang/Object;

# The scenario host records the argument of each call and returns normally,
# except for the explicit throwing scenario of protectedExit.
.method public static native mark(I)V
.end method

.method public static nestedBreak(III)I
    .locals 3
    const/4 v0, 0x0
    const/4 v2, 0x0
    :outer
    if-ge v0, p0, :done
    const/4 v1, 0x0
    :inner
    if-ge v1, p1, :outer_step
    if-eq v1, p2, :matched
    add-int/2addr v2, v1
    add-int/lit8 v1, v1, 0x1
    goto :inner
    :matched
    invoke-static {v0}, Lfixture/LoopExits;->mark(I)V
    add-int/lit8 v2, v2, 0xa
    goto :outer_step
    :outer_step
    add-int/lit8 v0, v0, 0x1
    goto :outer
    :done
    return v2
.end method

.method public static outerBreak(III)I
    .locals 3
    const/4 v0, 0x0
    const/4 v2, 0x0
    :outer
    if-ge v0, p0, :done
    const/4 v1, 0x0
    :inner
    if-ge v1, p1, :outer_step
    if-eq v1, p2, :matched
    add-int/2addr v2, v1
    add-int/lit8 v1, v1, 0x1
    goto :inner
    :matched
    invoke-static {v0}, Lfixture/LoopExits;->mark(I)V
    add-int/lit8 v2, v2, 0xa
    goto :done
    :outer_step
    add-int/lit8 v0, v0, 0x1
    goto :outer
    :done
    return v2
.end method

.method public static outerContinue(III)I
    .locals 3
    const/4 v0, 0x0
    const/4 v2, 0x0
    :outer
    if-ge v0, p0, :done
    const/4 v1, 0x0
    :inner
    if-ge v1, p1, :outer_step
    if-eq v1, p2, :matched
    add-int/2addr v2, v1
    add-int/lit8 v1, v1, 0x1
    goto :inner
    :matched
    invoke-static {v0}, Lfixture/LoopExits;->mark(I)V
    add-int/lit8 v2, v2, 0xa
    add-int/lit8 v0, v0, 0x1
    goto :outer
    :outer_step
    add-int/lit8 v0, v0, 0x1
    goto :outer
    :done
    return v2
.end method

# Two original exits enter the same matched block: neither may claim it private.
.method public static sharedExit(III)I
    .locals 4
    const/4 v0, 0x0
    const/4 v2, 0x0
    const/4 v3, 0x1
    :outer
    if-ge v0, p0, :done
    const/4 v1, 0x0
    :inner
    if-ge v1, p1, :outer_step
    if-eq v1, p2, :matched
    if-eq v1, v3, :matched
    add-int/2addr v2, v1
    add-int/lit8 v1, v1, 0x1
    goto :inner
    :matched
    invoke-static {v0}, Lfixture/LoopExits;->mark(I)V
    add-int/lit8 v2, v2, 0xa
    goto :outer_step
    :outer_step
    add-int/lit8 v0, v0, 0x1
    goto :outer
    :done
    return v2
.end method

# An exit enters a separate loop, not a lexically enclosing core.
.method public static siblingLoop(II)I
    .locals 2
    const/4 v0, 0x0
    const/4 v1, 0x0
    :first
    if-ge v0, p0, :done
    if-eq v0, p1, :second
    add-int/lit8 v0, v0, 0x1
    goto :first
    :second
    if-ge v1, p0, :done
    invoke-static {v1}, Lfixture/LoopExits;->mark(I)V
    add-int/lit8 v1, v1, 0x1
    goto :second
    :done
    return v1
.end method

# The private exit has a local handler: it must retain its protection owner.
.method public static protectedExit(III)I
    .locals 4
    const/4 v0, 0x0
    const/4 v2, 0x0
    :outer
    if-ge v0, p0, :done
    const/4 v1, 0x0
    :inner
    if-ge v1, p1, :outer_step
    if-eq v1, p2, :matched
    add-int/2addr v2, v1
    add-int/lit8 v1, v1, 0x1
    goto :inner
    :matched
    :try_start
    invoke-static {v0}, Lfixture/LoopExits;->mark(I)V
    :try_end
    .catch Ljava/lang/RuntimeException; {:try_start .. :try_end} :handler
    add-int/lit8 v2, v2, 0xa
    goto :outer_step
    :outer_step
    add-int/lit8 v0, v0, 0x1
    goto :outer
    :handler
    move-exception v3
    const/4 v2, -0x1
    return v2
    :done
    return v2
.end method

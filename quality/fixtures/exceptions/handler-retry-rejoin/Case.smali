.class public final Lfixture/RetryRejoin;
.super Ljava/lang/Object;

.method public static native fast(I)I
.end method
.method public static native reset(I)V
.end method
.method public static native attempt(II)I
.end method

.method public static run(III)I
    .registers 7
    :fast_start
    invoke-static {p0}, Lfixture/RetryRejoin;->fast(I)I
    move-result v3
    :fast_end
    return v3
    :fallback
    const/4 v0, 0
    :head
    if-ge v0, p2, :done
    invoke-static {v0}, Lfixture/RetryRejoin;->reset(I)V
    :attempt_start
    invoke-static {v0, p0}, Lfixture/RetryRejoin;->attempt(II)I
    move-result v1
    if-ne v1, p1, :increment
    return v1
    :attempt_end
    :caught
    :increment
    add-int/lit8 v0, v0, 1
    goto :head
    :done
    const/4 v1, -1
    return v1
    .catch Ljava/lang/IllegalArgumentException; {:fast_start .. :fast_end} :fallback
    .catch Ljava/lang/IllegalStateException; {:attempt_start .. :attempt_end} :caught
.end method

.class public final Lfixture/RetryNesting;
.super Ljava/lang/Object;

.method public static native attempt(III)I
.end method
.method public static native advance(II)V
.end method

.method public static nested(III)I
    .registers 7
    const/4 v0, 0
    const/4 v1, 0
    :outer
    if-ge v0, p0, :done
    const/4 v2, 0
    :inner
    if-ge v2, p1, :next_row
    :attempt_start
    invoke-static {v0, v2, p2}, Lfixture/RetryNesting;->attempt(III)I
    move-result v3
    add-int/2addr v1, v3
    :attempt_end
    :advance
    invoke-static {v0, v2}, Lfixture/RetryNesting;->advance(II)V
    add-int/lit8 v2, v2, 1
    goto :inner
    :next_row
    add-int/lit8 v0, v0, 1
    goto :outer
    :done
    return v1
    .catch Ljava/lang/IllegalStateException; {:attempt_start .. :attempt_end} :advance
.end method

.method public static terminal(II)I
    .registers 6
    const/4 v0, 0
    const/4 v1, 0
    const/4 v3, 0
    :head
    if-ge v0, p0, :done
    move v2, v0
    add-int/lit8 v0, v0, 1
    :attempt_start
    invoke-static {v3, v2, p1}, Lfixture/RetryNesting;->attempt(III)I
    move-result v2
    add-int/2addr v1, v2
    :attempt_end
    :back
    goto :head
    :done
    return v1
    .catch Ljava/lang/IllegalStateException; {:attempt_start .. :attempt_end} :back
.end method

.method public static foreign(II)I
    .registers 7
    const/4 v0, 0
    const/4 v1, 0
    const/4 v3, -1
    const/4 v4, 0
    :pre_start
    invoke-static {v3, v4, p1}, Lfixture/RetryNesting;->attempt(III)I
    move-result v2
    add-int/2addr v1, v2
    :pre_end
    goto :head
    :head
    if-ge v0, p0, :done
    :body_start
    invoke-static {v4, v0, p1}, Lfixture/RetryNesting;->attempt(III)I
    move-result v2
    add-int/2addr v1, v2
    :body_end
    :increment
    add-int/lit8 v0, v0, 1
    goto :head
    :done
    return v1
    .catch Ljava/lang/IllegalStateException; {:pre_start .. :pre_end} :increment
    .catch Ljava/lang/IllegalStateException; {:body_start .. :body_end} :increment
.end method

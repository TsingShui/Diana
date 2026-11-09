.class public final Lfixture/ReadableCatch;
.super Ljava/lang/Object;
.method public static native step(II)I
.end method
.method public static native recover(Ljava/lang/Throwable;II)I
.end method
.method public static native outside(I)V
.end method

.method public static run(II)I
    .registers 7
    const/4 v0, 0
    const/4 v1, 0
    :head
    if-ge v0, p0, :done
    :body
    invoke-static {v0, p1}, Lfixture/ReadableCatch;->step(II)I
    move-result v2
    add-int/2addr v1, v2
    :body_end
    goto :update
    :caught
    move-exception v3
    invoke-static {v3, v0, p1}, Lfixture/ReadableCatch;->recover(Ljava/lang/Throwable;II)I
    move-result v2
    add-int/2addr v1, v2
    :update
    add-int/lit8 v0, v0, 1
    goto :head
    :done
    return v1
    .catch Ljava/lang/IllegalStateException; {:body .. :body_end} :caught
.end method

.method public static rethrow(ILjava/lang/Throwable;)I
    .registers 5
    const/4 v0, 0
    :body
    invoke-static {v0, p0}, Lfixture/ReadableCatch;->step(II)I
    move-result v1
    throw p1
    :body_end
    :caught
    move-exception v2
    invoke-static {v2, v0, p0}, Lfixture/ReadableCatch;->recover(Ljava/lang/Throwable;II)I
    move-result v1
    return v1
    .catch Ljava/lang/IllegalStateException; {:body .. :body_end} :caught
.end method

.method public static foreign(I)I
    .registers 5
    const/4 v0, 0
    :first
    invoke-static {v0, p0}, Lfixture/ReadableCatch;->step(II)I
    move-result v1
    :first_end
    const/4 v0, 1
    invoke-static {p0}, Lfixture/ReadableCatch;->outside(I)V
    :second
    invoke-static {v0, p0}, Lfixture/ReadableCatch;->step(II)I
    move-result v2
    add-int/2addr v1, v2
    :second_end
    return v1
    :caught
    move-exception v3
    invoke-static {v3, v0, p0}, Lfixture/ReadableCatch;->recover(Ljava/lang/Throwable;II)I
    move-result v2
    return v2
    .catch Ljava/lang/IllegalStateException; {:first .. :first_end} :caught
    .catch Ljava/lang/IllegalStateException; {:second .. :second_end} :caught
.end method

.method public static multi(I)I
    .registers 4
    const/4 v0, 0
    :body
    invoke-static {v0, p0}, Lfixture/ReadableCatch;->step(II)I
    move-result v1
    :body_end
    return v1
    :state
    move-exception v2
    invoke-static {v2, v0, p0}, Lfixture/ReadableCatch;->recover(Ljava/lang/Throwable;II)I
    move-result v1
    return v1
    :argument
    move-exception v2
    const/4 v1, -2
    return v1
    .catch Ljava/lang/IllegalStateException; {:body .. :body_end} :state
    .catch Ljava/lang/IllegalArgumentException; {:body .. :body_end} :argument
.end method

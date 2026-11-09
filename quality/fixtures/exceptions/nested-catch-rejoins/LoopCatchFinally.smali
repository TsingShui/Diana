.class public final Lfixture/LoopCatchFinally;
.super Ljava/lang/Object;

.field public static trace:I

.method public static mark(I)V
    .locals 1
    sget v0, Lfixture/LoopCatchFinally;->trace:I
    mul-int/lit8 v0, v0, 0x1f
    add-int/2addr v0, p0
    sput v0, Lfixture/LoopCatchFinally;->trace:I
    return-void
.end method

.method public static read(II)I
    .locals 3
    add-int/lit8 v0, p0, 0xa
    invoke-static {v0}, Lfixture/LoopCatchFinally;->mark(I)V
    const/4 v0, 0x1
    if-ne p1, v0, :not_nfe
    new-instance v1, Ljava/lang/NumberFormatException;
    const-string v2, "row"
    invoke-direct {v1, v2}, Ljava/lang/NumberFormatException;-><init>(Ljava/lang/String;)V
    throw v1
    :not_nfe
    const/4 v0, 0x2
    if-ne p1, v0, :not_npe
    new-instance v1, Ljava/lang/NullPointerException;
    const-string v2, "row"
    invoke-direct {v1, v2}, Ljava/lang/NullPointerException;-><init>(Ljava/lang/String;)V
    throw v1
    :not_npe
    const/4 v0, 0x3
    if-ne p1, v0, :not_ise
    new-instance v1, Ljava/lang/IllegalStateException;
    const-string v2, "row"
    invoke-direct {v1, v2}, Ljava/lang/IllegalStateException;-><init>(Ljava/lang/String;)V
    throw v1
    :not_ise
    const/4 v0, 0x4
    if-eq p1, v0, :error
    const/4 v0, 0x6
    if-ne p1, v0, :normal
    :error
    new-instance v1, Ljava/lang/AssertionError;
    const-string v2, "row"
    invoke-direct {v1, v2}, Ljava/lang/AssertionError;-><init>(Ljava/lang/Object;)V
    throw v1
    :normal
    add-int/lit8 v0, p0, 0x1
    return v0
.end method

.method public static cleanup(II)V
    .locals 3
    add-int/lit8 v0, p0, 0x1e
    invoke-static {v0}, Lfixture/LoopCatchFinally;->mark(I)V
    const/4 v0, 0x5
    if-eq p1, v0, :fail
    const/4 v0, 0x6
    if-ne p1, v0, :done
    :fail
    new-instance v1, Ljava/lang/IllegalArgumentException;
    const-string v2, "close"
    invoke-direct {v1, v2}, Ljava/lang/IllegalArgumentException;-><init>(Ljava/lang/String;)V
    throw v1
    :done
    return-void
.end method

.method public static scan(II)I
    .locals 6
    const/4 v0, 0x0
    const/4 v1, 0x0
    :loop
    if-ge v1, p0, :done
    :body
    invoke-static {v1, p1}, Lfixture/LoopCatchFinally;->read(II)I
    move-result v2
    :body_end
    goto :normal_cleanup
    :typed
    move-exception v3
    const/16 v4, 0x14
    invoke-static {v4}, Lfixture/LoopCatchFinally;->mark(I)V
    const/16 v2, 0x64
    :typed_end
    goto :normal_cleanup
    :broad
    move-exception v3
    const/16 v4, 0x15
    invoke-static {v4}, Lfixture/LoopCatchFinally;->mark(I)V
    const/4 v2, -0x2
    :broad_end
    :normal_cleanup
    invoke-static {v1, p1}, Lfixture/LoopCatchFinally;->cleanup(II)V
    if-ltz v2, :return_value
    const/4 v4, 0x2
    if-eq v2, v4, :update
    add-int/2addr v0, v2
    const/16 v4, 0xc8
    if-ge v0, v4, :done
    :update
    add-int/lit8 v1, v1, 0x1
    goto :loop
    :return_value
    return v2
    :done
    return v0
    :outer_cleanup
    move-exception v5
    invoke-static {v1, p1}, Lfixture/LoopCatchFinally;->cleanup(II)V
    throw v5
    .catch Ljava/lang/NumberFormatException; {:body .. :body_end} :typed
    .catch Ljava/lang/NullPointerException; {:body .. :body_end} :typed
    .catch Ljava/lang/RuntimeException; {:body .. :body_end} :broad
    .catchall {:body .. :body_end} :outer_cleanup
    .catchall {:typed .. :typed_end} :outer_cleanup
    .catchall {:broad .. :broad_end} :outer_cleanup
.end method

.method public static priority(I)I
    .locals 2
    const/4 v0, 0x0
    :body
    invoke-static {v0, p0}, Lfixture/LoopCatchFinally;->read(II)I
    move-result v0
    :body_end
    const/4 v0, 0x1
    return v0
    :first
    move-exception v1
    const/16 v0, 0x64
    return v0
    :second
    move-exception v1
    const/4 v0, -0x2
    return v0
    .catch Ljava/lang/NumberFormatException; {:body .. :body_end} :first
    .catch Ljava/lang/RuntimeException; {:body .. :body_end} :second
    .catch Ljava/lang/NullPointerException; {:body .. :body_end} :first
.end method

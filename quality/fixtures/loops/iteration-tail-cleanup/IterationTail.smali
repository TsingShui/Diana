.class public final Lfixture/IterationTail;
.super Ljava/lang/Object;

.field public static trace:I

.method public static mark(I)V
    .locals 1
    sget v0, Lfixture/IterationTail;->trace:I
    mul-int/lit8 v0, v0, 0x1f
    add-int/2addr v0, p0
    sput v0, Lfixture/IterationTail;->trace:I
    return-void
.end method

.method public static read(II)I
    .locals 3
    add-int/lit8 v0, p0, 0xa
    invoke-static {v0}, Lfixture/IterationTail;->mark(I)V
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
    invoke-static {v0}, Lfixture/IterationTail;->mark(I)V
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

.method public static nested(III)I
    .locals 9
    const/4 v0, 0x0
    const/4 v2, 0x0
    :outer
    if-ge v0, p0, :done
    const/4 v6, 0x1
    if-ne v0, v6, :outer_body
    if-nez p2, :outer_update
    :outer_body
    const/4 v1, 0x0
    :inner
    const/4 v6, 0x3
    if-ge v1, v6, :outer_update
    mul-int/lit8 v3, v0, 0x3
    add-int/2addr v3, v1
    :read_start
    invoke-static {v3, p1}, Lfixture/IterationTail;->read(II)I
    move-result v4
    :read_end
    goto :normal_cleanup
    :typed
    move-exception v5
    const/16 v6, 0x14
    invoke-static {v6}, Lfixture/IterationTail;->mark(I)V
    const/16 v4, 0x64
    :typed_end
    goto :normal_cleanup
    :broad
    move-exception v5
    const/16 v6, 0x15
    invoke-static {v6}, Lfixture/IterationTail;->mark(I)V
    const/4 v4, -0x2
    :broad_end
    goto :normal_cleanup
    :normal_cleanup
    invoke-static {v3, p1}, Lfixture/IterationTail;->cleanup(II)V
    if-ltz v4, :return_value
    const/4 v6, 0x2
    if-eq v4, v6, :inner_update
    add-int/2addr v2, v4
    const/4 v6, 0x5
    if-eq v4, v6, :outer_update
    :inner_update
    add-int/lit8 v1, v1, 0x1
    goto :inner
    :outer_update
    add-int/lit8 v0, v0, 0x1
    goto :outer
    :done
    return v2
    :return_value
    return v4
    :finally_handler
    move-exception v7
    invoke-static {v3, p1}, Lfixture/IterationTail;->cleanup(II)V
    throw v7
    .catch Ljava/lang/NumberFormatException; {:read_start .. :read_end} :typed
    .catch Ljava/lang/NullPointerException; {:read_start .. :read_end} :typed
    .catch Ljava/lang/RuntimeException; {:read_start .. :read_end} :broad
    .catchall {:read_start .. :read_end} :finally_handler
    .catchall {:typed .. :typed_end} :finally_handler
    .catchall {:broad .. :broad_end} :finally_handler
.end method

.method public static stride(II)I
    .locals 2
    const/4 v0, 0x0
    const/4 v1, 0x0
    :head
    if-ge v0, p0, :done
    invoke-static {v0}, Lfixture/IterationTail;->mark(I)V
    if-eqz v0, :step
    add-int/2addr v1, v0
    :step
    add-int/2addr v0, p1
    goto :head
    :done
    return v1
.end method

.method public static skipUpdate(II)I
    .locals 4
    const/4 v0, 0x0
    const/4 v1, 0x0
    :header
    if-ge v0, p0, :done
    add-int/lit8 v2, v0, 0x32
    invoke-static {v2}, Lfixture/IterationTail;->mark(I)V
    if-eqz p1, :advance
    add-int/lit8 v1, v1, 0x1
    const/4 v3, 0x2
    if-eq v1, v3, :done
    goto :header
    :advance
    add-int/lit8 v0, v0, 0x1
    goto :header
    :done
    return v1
.end method

.method public static locked(Ljava/lang/Object;I)I
    .locals 2
    const/4 v0, 0x0
    :header
    if-ge v0, p1, :done
    monitor-enter p0
    :body
    invoke-static {v0}, Lfixture/IterationTail;->mark(I)V
    :body_end
    monitor-exit p0
    goto :step
    :handler
    move-exception v1
    monitor-exit p0
    throw v1
    :step
    add-int/lit8 v0, v0, 0x1
    goto :header
    :done
    return v0
    .catchall {:body .. :body_end} :handler
.end method

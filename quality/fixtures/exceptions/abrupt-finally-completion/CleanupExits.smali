.class public final Lfixture/CleanupExits;
.super Ljava/lang/Object;
.field public static trace:I

.method public static mark(I)V
    .locals 1
    sget v0, Lfixture/CleanupExits;->trace:I
    mul-int/lit8 v0, v0, 0x1f
    add-int/2addr v0, p0
    sput v0, Lfixture/CleanupExits;->trace:I
    return-void
.end method

.method public static read(II)I
    .locals 2
    add-int/lit8 v0, p0, 0xa
    invoke-static {v0}, Lfixture/CleanupExits;->mark(I)V
    const/4 v0, 0x1
    shl-int/2addr v0, p0
    and-int/2addr v0, p1
    if-eqz v0, :normal
    new-instance v0, Ljava/lang/IllegalStateException;
    const-string v1, "read"
    invoke-direct {v0, v1}, Ljava/lang/IllegalStateException;-><init>(Ljava/lang/String;)V
    throw v0
    :normal
    add-int/lit8 v0, p0, 0x7
    return v0
.end method

.method public static close(II)V
    .locals 2
    add-int/lit8 v0, p0, 0x1e
    invoke-static {v0}, Lfixture/CleanupExits;->mark(I)V
    const/16 v0, 0x10
    shl-int/2addr v0, p0
    and-int/2addr v0, p1
    if-eqz v0, :normal
    new-instance v0, Ljava/lang/IllegalArgumentException;
    const-string v1, "close"
    invoke-direct {v0, v1}, Ljava/lang/IllegalArgumentException;-><init>(Ljava/lang/String;)V
    throw v0
    :normal
    return-void
.end method

# Both body preparation and first cleanup are AddInt operations with different reads.
.method public static capture(III)I
    .locals 5
    const/4 v3, 0x0
    const/16 v2, 0x64
    :body
    invoke-static {v3, p2}, Lfixture/CleanupExits;->read(II)I
    move-result v0
    :body_end
    add-int v1, v0, p1
    add-int p1, p1, v2
    invoke-static {v3, p2}, Lfixture/CleanupExits;->close(II)V
    invoke-static {p1}, Lfixture/CleanupExits;->mark(I)V
    and-int/lit8 v0, p0, 0x2
    if-eqz v0, :normal_throw_check
    const/4 v0, -0x7
    return v0
    :normal_throw_check
    and-int/lit8 v0, p0, 0x4
    if-eqz v0, :normal_return
    new-instance v0, Ljava/lang/IllegalArgumentException;
    const-string v2, "override"
    invoke-direct {v0, v2}, Ljava/lang/IllegalArgumentException;-><init>(Ljava/lang/String;)V
    throw v0
    :normal_return
    return v1
    :handler
    move-exception v4
    add-int p1, p1, v2
    invoke-static {v3, p2}, Lfixture/CleanupExits;->close(II)V
    invoke-static {p1}, Lfixture/CleanupExits;->mark(I)V
    and-int/lit8 v0, p0, 0x2
    if-eqz v0, :handler_throw_check
    const/4 v0, -0x7
    return v0
    :handler_throw_check
    and-int/lit8 v0, p0, 0x4
    if-eqz v0, :rethrow
    new-instance v0, Ljava/lang/IllegalArgumentException;
    const-string v2, "override"
    invoke-direct {v0, v2}, Ljava/lang/IllegalArgumentException;-><init>(Ljava/lang/String;)V
    throw v0
    :rethrow
    throw v4
    .catchall {:body .. :body_end} :handler
.end method

.method public static always(II)I
    .locals 3
    const/4 v0, 0x0
    :body
    invoke-static {v0, p1}, Lfixture/CleanupExits;->read(II)I
    move-result v1
    :body_end
    invoke-static {v0, p1}, Lfixture/CleanupExits;->close(II)V
    and-int/lit8 v1, p0, 0x4
    if-eqz v1, :normal_return
    new-instance v1, Ljava/lang/IllegalArgumentException;
    const-string v2, "override"
    invoke-direct {v1, v2}, Ljava/lang/IllegalArgumentException;-><init>(Ljava/lang/String;)V
    throw v1
    :normal_return
    const/16 v1, 0x8
    return v1
    :handler
    move-exception v1
    invoke-static {v0, p1}, Lfixture/CleanupExits;->close(II)V
    and-int/lit8 v1, p0, 0x4
    if-eqz v1, :handler_return
    new-instance v1, Ljava/lang/IllegalArgumentException;
    const-string v2, "override"
    invoke-direct {v1, v2}, Ljava/lang/IllegalArgumentException;-><init>(Ljava/lang/String;)V
    throw v1
    :handler_return
    const/16 v1, 0x8
    return v1
    .catchall {:body .. :body_end} :handler
.end method

.method public static voidAlways(I)V
    .locals 2
    const/4 v0, 0x0
    :body
    invoke-static {v0, p0}, Lfixture/CleanupExits;->read(II)I
    move-result v1
    :body_end
    invoke-static {v0, p0}, Lfixture/CleanupExits;->close(II)V
    return-void
    :handler
    move-exception v1
    invoke-static {v0, p0}, Lfixture/CleanupExits;->close(II)V
    return-void
    .catchall {:body .. :body_end} :handler
.end method

.method public static twoRethrows(II)I
    .locals 4
    const/4 v0, 0x0
    :body
    invoke-static {v0, p1}, Lfixture/CleanupExits;->read(II)I
    move-result v1
    :body_end
    and-int/lit8 v2, p0, 0x1
    if-eqz v2, :normal_second
    invoke-static {v0, p1}, Lfixture/CleanupExits;->close(II)V
    goto :normal_end
    :normal_second
    const/4 v0, 0x1
    invoke-static {v0, p1}, Lfixture/CleanupExits;->close(II)V
    goto :normal_end
    :normal_end
    const/16 v1, 0x8
    return v1
    :handler
    move-exception v3
    and-int/lit8 v2, p0, 0x1
    if-eqz v2, :handler_second
    invoke-static {v0, p1}, Lfixture/CleanupExits;->close(II)V
    throw v3
    :handler_second
    const/4 v0, 0x1
    invoke-static {v0, p1}, Lfixture/CleanupExits;->close(II)V
    throw v3
    .catchall {:body .. :body_end} :handler
.end method

.method public static bareVoid(I)V
    .locals 2
    const/4 v0, 0x0
    :body
    invoke-static {v0, p0}, Lfixture/CleanupExits;->read(II)I
    move-result v1
    :body_end
    return-void
    :handler
    move-exception v1
    return-void
    .catchall {:body .. :body_end} :handler
.end method

.method public static parameterThrow(Ljava/lang/Throwable;I)V
    .locals 2
    const/4 v0, 0x0
    :body
    invoke-static {v0, p1}, Lfixture/CleanupExits;->read(II)I
    move-result v1
    :body_end
    throw p0
    :handler
    move-exception v1
    throw p0
    .catchall {:body .. :body_end} :handler
.end method

.method public static differentReturn(I)I
    .locals 2
    const/4 v0, 0x0
    :body
    invoke-static {v0, p0}, Lfixture/CleanupExits;->read(II)I
    move-result v1
    :body_end
    invoke-static {v0, p0}, Lfixture/CleanupExits;->close(II)V
    const/4 v1, -0x7
    return v1
    :handler
    move-exception v1
    invoke-static {v0, p0}, Lfixture/CleanupExits;->close(II)V
    const/4 v1, -0x8
    return v1
    .catchall {:body .. :body_end} :handler
.end method

.method public static differentThrow(I)V
    .locals 3
    const/4 v0, 0x0
    :body
    invoke-static {v0, p0}, Lfixture/CleanupExits;->read(II)I
    move-result v1
    :body_end
    invoke-static {v0, p0}, Lfixture/CleanupExits;->close(II)V
    new-instance v1, Ljava/lang/IllegalArgumentException;
    const-string v2, "normal"
    invoke-direct {v1, v2}, Ljava/lang/IllegalArgumentException;-><init>(Ljava/lang/String;)V
    throw v1
    :handler
    move-exception v1
    invoke-static {v0, p0}, Lfixture/CleanupExits;->close(II)V
    new-instance v1, Ljava/lang/IllegalArgumentException;
    const-string v2, "handler"
    invoke-direct {v1, v2}, Ljava/lang/IllegalArgumentException;-><init>(Ljava/lang/String;)V
    throw v1
    .catchall {:body .. :body_end} :handler
.end method

.method public static foreign(II)I
    .locals 2
    const/4 v0, 0x0
    and-int/lit8 v1, p0, 0x8
    if-nez v1, :normal_cleanup
    :body
    invoke-static {v0, p1}, Lfixture/CleanupExits;->read(II)I
    move-result v1
    :body_end
    :normal_cleanup
    invoke-static {v0, p1}, Lfixture/CleanupExits;->close(II)V
    const/16 v1, 0x8
    return v1
    :handler
    move-exception v1
    invoke-static {v0, p1}, Lfixture/CleanupExits;->close(II)V
    const/16 v1, 0x8
    return v1
    .catchall {:body .. :body_end} :handler
.end method

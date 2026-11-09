.class public final Lfixture/BranchFinally;
.super Ljava/lang/Object;

.field public static trace:I

.method public static mark(I)V
    .locals 1
    sget v0, Lfixture/BranchFinally;->trace:I
    mul-int/lit8 v0, v0, 0x1f
    add-int/2addr v0, p0
    sput v0, Lfixture/BranchFinally;->trace:I
    return-void
.end method

.method public static read(II)I
    .locals 3
    add-int/lit8 v0, p0, 0xa
    invoke-static {v0}, Lfixture/BranchFinally;->mark(I)V
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

.method public static finish(II)V
    .locals 3
    add-int/lit8 v0, p0, 0x1e
    invoke-static {v0}, Lfixture/BranchFinally;->mark(I)V
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

.method public static run(III)I
    .locals 6
    const/4 v0, 0x0
    const/4 v1, 0x0
    :loop
    if-ge v0, p0, :normal_cleanup
    :read_start
    invoke-static {v0, p1}, Lfixture/BranchFinally;->read(II)I
    move-result v2
    :read_end
    goto :join
    :typed
    move-exception v5
    const/16 v3, 0x14
    invoke-static {v3}, Lfixture/BranchFinally;->mark(I)V
    const/16 v2, 0x64
    :typed_end
    :join
    add-int/2addr v1, v2
    add-int/lit8 v0, v0, 0x1
    goto :loop
    :normal_cleanup
    if-eqz p2, :normal_zero
    const/16 v3, 0x29
    invoke-static {v3}, Lfixture/BranchFinally;->mark(I)V
    invoke-static {p0, p1}, Lfixture/BranchFinally;->finish(II)V
    goto :return
    :normal_zero
    const/16 v3, 0x28
    invoke-static {v3}, Lfixture/BranchFinally;->mark(I)V
    :return
    return v1
    :finally_handler
    move-exception v5
    if-eqz p2, :exception_zero
    const/16 v3, 0x29
    invoke-static {v3}, Lfixture/BranchFinally;->mark(I)V
    invoke-static {p0, p1}, Lfixture/BranchFinally;->finish(II)V
    goto :rethrow
    :exception_zero
    const/16 v3, 0x28
    invoke-static {v3}, Lfixture/BranchFinally;->mark(I)V
    :rethrow
    throw v5
    .catch Ljava/lang/NumberFormatException; {:read_start .. :read_end} :typed
    .catch Ljava/lang/NullPointerException; {:read_start .. :read_end} :typed
    .catchall {:read_start .. :read_end} :finally_handler
    .catchall {:typed .. :typed_end} :finally_handler
.end method

.method public static different(I)I
    .locals 4
    const/4 v0, 0x0
    const/4 v1, 0x1
    const/4 v3, 0x2
    :body
    invoke-static {v0, p0}, Lfixture/BranchFinally;->read(II)I
    move-result v0
    :body_end
    invoke-static {v1, p0}, Lfixture/BranchFinally;->finish(II)V
    return v0
    :handler
    move-exception v2
    invoke-static {v3, p0}, Lfixture/BranchFinally;->finish(II)V
    throw v2
    .catchall {:body .. :body_end} :handler
.end method

.method public static guardedRethrow(II)I
    .locals 4
    const/4 v0, 0x0
    const/16 v1, 0x63
    :prefix
    invoke-static {v1, p1}, Lfixture/BranchFinally;->read(II)I
    move-result v3
    :prefix_end
    :body
    invoke-static {v0, p0}, Lfixture/BranchFinally;->read(II)I
    move-result v1
    :body_end
    invoke-static {v0}, Lfixture/BranchFinally;->mark(I)V
    return v1
    :handler
    move-exception v2
    invoke-static {v0}, Lfixture/BranchFinally;->mark(I)V
    :rethrow
    throw v2
    :rethrow_end
    :outside
    move-exception v2
    const/16 v1, 0x12c
    return v1
    .catchall {:prefix .. :prefix_end} :outside
    .catchall {:body .. :body_end} :handler
    .catchall {:rethrow .. :rethrow_end} :outside
.end method

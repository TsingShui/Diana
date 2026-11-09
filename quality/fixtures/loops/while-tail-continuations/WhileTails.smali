.class public final Lfixture/WhileTails;
.super Ljava/lang/Object;
.field public static trace:I

.method public static mark(I)V
    .locals 1
    sget v0, Lfixture/WhileTails;->trace:I
    mul-int/lit8 v0, v0, 0x1f
    add-int/2addr v0, p0
    sput v0, Lfixture/WhileTails;->trace:I
    return-void
.end method

.method public static touch(II)V
    .locals 2
    invoke-static {p0}, Lfixture/WhileTails;->mark(I)V
    if-ne p0, p1, :ok
    new-instance v0, Ljava/lang/IllegalStateException;
    const-string v1, "tail"
    invoke-direct {v0, v1}, Ljava/lang/IllegalStateException;-><init>(Ljava/lang/String;)V
    throw v0
    :ok
    return-void
.end method

.method public static read(II)I
    .locals 2
    add-int/lit8 v0, p0, 0xa
    invoke-static {v0}, Lfixture/WhileTails;->mark(I)V
    const/4 v0, 0x1
    if-eq p1, v0, :nfe
    const/4 v0, 0x4
    if-eq p1, v0, :error
    add-int/lit8 v0, p0, 0x1
    return v0
    :nfe
    new-instance v0, Ljava/lang/NumberFormatException;
    invoke-direct {v0}, Ljava/lang/NumberFormatException;-><init>()V
    throw v0
    :error
    new-instance v0, Ljava/lang/AssertionError;
    invoke-direct {v0}, Ljava/lang/AssertionError;-><init>()V
    throw v0
.end method

.method public static cleanup(II)V
    .locals 1
    add-int/lit8 v0, p0, 0x1e
    invoke-static {v0}, Lfixture/WhileTails;->mark(I)V
    const/4 v0, 0x2
    if-eq p1, v0, :fail
    const/4 v0, 0x4
    if-ne p1, v0, :ok
    :fail
    new-instance v0, Ljava/lang/IllegalArgumentException;
    invoke-direct {v0}, Ljava/lang/IllegalArgumentException;-><init>()V
    throw v0
    :ok
    return-void
.end method

.method public static walk(III)I
    .locals 5
    const/4 v0, 0x0
    const/4 v1, 0x0
    :head
    if-ge v0, p0, :done
    :body
    invoke-static {v0, p2}, Lfixture/WhileTails;->read(II)I
    move-result v2
    const/4 v4, 0x1
    if-eq v0, v4, :body_end
    add-int/2addr v1, v2
    :body_end
    invoke-static {v0, p2}, Lfixture/WhileTails;->cleanup(II)V
    const/4 v4, 0x3
    if-ne p2, v4, :update
    const/4 v4, 0x2
    if-eq v0, v4, :done
    :update
    add-int/2addr v0, p1
    goto :head
    :done
    return v1
    :handler
    move-exception v3
    invoke-static {v0, p2}, Lfixture/WhileTails;->cleanup(II)V
    throw v3
    .catchall {:body .. :body_end} :handler
.end method

.method public static tail(II)I
    .locals 3
    const/4 v0, 0x0
    const/4 v1, 0x0
    :body
    add-int/lit8 v2, v0, 0x3c
    invoke-static {v2, p1}, Lfixture/WhileTails;->touch(II)V
    add-int/2addr v1, v0
    add-int/lit8 v0, v0, 0x1
    if-lt v0, p0, :body
    return v1
.end method

.method public static nested(III)I
    .locals 6
    const/4 v0, 0x0
    const/4 v3, 0x0
    :outer
    if-ge v0, p0, :done
    const/4 v1, 0x0
    :middle
    const/4 v4, 0x2
    if-ge v1, v4, :outer_tail
    const/4 v2, 0x0
    :inner
    const/4 v4, 0x3
    if-ge v2, v4, :middle_tail
    if-eq v2, p1, :private_tail
    add-int/lit8 v4, v2, 0x1
    add-int/2addr v3, v4
    add-int/lit8 v2, v2, 0x1
    goto :inner
    :private_tail
    mul-int/lit8 v4, v0, 0xa
    add-int/2addr v4, v1
    add-int/lit8 v4, v4, 0x64
    invoke-static {v4, p2}, Lfixture/WhileTails;->touch(II)V
    add-int/lit8 v3, v3, 0xa
    add-int/lit8 v0, v0, 0x2
    goto :outer
    :middle_tail
    add-int/lit8 v1, v1, 0x1
    goto :middle
    :outer_tail
    add-int/lit16 v4, v0, 0xc8
    invoke-static {v4, p2}, Lfixture/WhileTails;->touch(II)V
    add-int/lit8 v0, v0, 0x1
    goto :outer
    :done
    return v3
.end method

.method public static shared(III)I
    .locals 6
    const/4 v0, 0x0
    const/4 v3, 0x0
    :outer
    if-ge v0, p0, :done
    const/4 v1, 0x0
    :middle
    const/4 v4, 0x2
    if-ge v1, v4, :outer_tail
    const/4 v2, 0x0
    :inner
    const/4 v4, 0x3
    if-ge v2, v4, :middle_tail
    if-eq v2, p1, :private_tail
    const/4 v5, 0x1
    if-eq v2, v5, :private_tail
    add-int/lit8 v4, v2, 0x1
    add-int/2addr v3, v4
    add-int/lit8 v2, v2, 0x1
    goto :inner
    :private_tail
    mul-int/lit8 v4, v0, 0xa
    add-int/2addr v4, v1
    add-int/lit8 v4, v4, 0x64
    invoke-static {v4, p2}, Lfixture/WhileTails;->touch(II)V
    add-int/lit8 v3, v3, 0xa
    add-int/lit8 v0, v0, 0x2
    goto :outer
    :middle_tail
    add-int/lit8 v1, v1, 0x1
    goto :middle
    :outer_tail
    add-int/lit16 v4, v0, 0xc8
    invoke-static {v4, p2}, Lfixture/WhileTails;->touch(II)V
    add-int/lit8 v0, v0, 0x1
    goto :outer
    :done
    return v3
.end method

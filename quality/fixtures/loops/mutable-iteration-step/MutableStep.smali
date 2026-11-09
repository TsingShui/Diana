.class public final Lfixture/MutableStep;
.super Ljava/lang/Object;

.field public static trace:I

.method public static mark(I)V
    .locals 1
    sget v0, Lfixture/MutableStep;->trace:I
    mul-int/lit8 v0, v0, 0x1f
    add-int/2addr v0, p0
    sput v0, Lfixture/MutableStep;->trace:I
    return-void
.end method

.method public static read(II)I
    .locals 3
    add-int/lit8 v0, p0, 0xa
    invoke-static {v0}, Lfixture/MutableStep;->mark(I)V
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
    invoke-static {v0}, Lfixture/MutableStep;->mark(I)V
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

.method public static walk(III)I
    .locals 7
    const/4 v0, 0x0
    const/4 v1, 0x0
    const/4 v2, 0x1
    :head
    if-ge v0, p0, :done
    :read_start
    invoke-static {v0, p1}, Lfixture/MutableStep;->read(II)I
    move-result v3
    :read_end
    goto :set_step
    :typed
    move-exception v4
    const/16 v5, 0x14
    invoke-static {v5}, Lfixture/MutableStep;->mark(I)V
    const/16 v3, 0x64
    :typed_end
    goto :set_step
    :set_step
    if-eqz v0, :two
    const/4 v2, 0x1
    goto :body_end
    :two
    const/4 v2, 0x2
    :body_end
    invoke-static {v0, p1}, Lfixture/MutableStep;->cleanup(II)V
    if-eqz p2, :after_continue
    if-eqz v0, :update
    :after_continue
    const/4 v5, 0x3
    if-eq v0, v5, :done
    const/4 v5, 0x2
    if-ne p2, v5, :add
    if-eq v0, v5, :done
    :add
    add-int/2addr v1, v3
    :update
    add-int/2addr v0, v2
    goto :head
    :done
    return v1
    :all
    move-exception v6
    invoke-static {v0, p1}, Lfixture/MutableStep;->cleanup(II)V
    throw v6
    .catch Ljava/lang/NumberFormatException; {:read_start .. :read_end} :typed
    .catch Ljava/lang/NullPointerException; {:read_start .. :read_end} :typed
    .catchall {:read_start .. :read_end} :all
    .catchall {:typed .. :typed_end} :all
    .catchall {:set_step .. :body_end} :all
.end method

.method public static nested(III)I
    .locals 5
    const/4 v0, 0x0
    const/4 v2, 0x0
    :outer
    if-ge v0, p0, :done
    const/4 v1, 0x0
    :inner
    const/4 v4, 0x3
    if-ge v1, v4, :outer_tail
    mul-int/lit8 v3, v0, 0xa
    add-int/2addr v3, v1
    invoke-static {v3}, Lfixture/MutableStep;->mark(I)V
    const/4 v4, 0x1
    if-ne p2, v4, :second
    if-eq v1, v4, :done
    :second
    const/4 v4, 0x2
    if-ne p2, v4, :add
    if-eqz v1, :outer_update
    :add
    if-eq v1, v4, :inner_update
    add-int/lit8 v2, v2, 0x1
    :inner_update
    add-int/2addr v1, p1
    goto :inner
    :outer_tail
    add-int/lit8 v3, v0, 0x5a
    invoke-static {v3}, Lfixture/MutableStep;->mark(I)V
    :outer_update
    add-int/lit8 v0, v0, 0x1
    goto :outer
    :done
    return v2
.end method

.method public static divide(II)I
    .locals 2
    const/4 v0, 0x4
    :head
    if-le v0, p0, :done
    invoke-static {v0}, Lfixture/MutableStep;->mark(I)V
    const/4 v1, 0x4
    if-eq v0, v1, :update
    const/16 v1, 0x63
    invoke-static {v1}, Lfixture/MutableStep;->mark(I)V
    :update
    div-int/2addr v0, p1
    goto :head
    :done
    return v0
.end method

.class public final Lfixture/TailPreparation;
.super Ljava/lang/Object;

.field public static trace:I

.method public static mark(I)V
    .locals 1
    sget v0, Lfixture/TailPreparation;->trace:I
    mul-int/lit8 v0, v0, 0x1f
    add-int/2addr v0, p0
    sput v0, Lfixture/TailPreparation;->trace:I
    return-void
.end method

.method public static read(II)I
    .locals 3
    add-int/lit8 v0, p0, 0xa
    invoke-static {v0}, Lfixture/TailPreparation;->mark(I)V
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
    if-eq p1, v0, :error
    const/16 v0, 0x8
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
    invoke-static {v0}, Lfixture/TailPreparation;->mark(I)V
    const/4 v0, 0x5
    if-eq p1, v0, :fail
    const/4 v0, 0x6
    if-eq p1, v0, :fail
    const/4 v0, 0x7
    if-eq p1, v0, :late
    const/16 v0, 0x8
    if-ne p1, v0, :done
    :late
    const/4 v0, 0x3
    if-ne p0, v0, :done
    :fail
    new-instance v1, Ljava/lang/IllegalArgumentException;
    const-string v2, "close"
    invoke-direct {v1, v2}, Ljava/lang/IllegalArgumentException;-><init>(Ljava/lang/String;)V
    throw v1
    :done
    return-void
.end method

.method public static finish(IIIII)I
    .locals 7
    :read_start
    invoke-static {p0, p1}, Lfixture/TailPreparation;->read(II)I
    move-result v0
    :read_end
    goto :adjust
    :typed
    move-exception v3
    const/16 v6, 0x14
    invoke-static {v6}, Lfixture/TailPreparation;->mark(I)V
    const/16 v0, 0x64
    :typed_end
    goto :adjust
    :adjust
    if-nez p2, :xor
    add-int/lit8 v0, v0, 0x1
    goto :delta
    :xor
    xor-int/lit8 v0, v0, 0x5
    :delta
    if-ltz p2, :sum
    const/4 p2, 0x2
    :sum
    add-int v2, v0, p2
    :body_end
    invoke-static {p0, p1}, Lfixture/TailPreparation;->cleanup(II)V
    if-nez p3, :mark
    if-eqz p4, :done
    :mark
    const/16 v6, 0x32
    invoke-static {v6}, Lfixture/TailPreparation;->mark(I)V
    :done
    return v2
    :all
    move-exception v4
    invoke-static {p0, p1}, Lfixture/TailPreparation;->cleanup(II)V
    throw v4
    .catch Ljava/lang/NumberFormatException; {:read_start .. :read_end} :typed
    .catch Ljava/lang/NullPointerException; {:read_start .. :read_end} :typed
    .catchall {:read_start .. :read_end} :all
    .catchall {:typed .. :typed_end} :all
    .catchall {:adjust .. :body_end} :all
.end method

.method public static literal(IIIII)I
    .locals 7
    :read_start
    invoke-static {p0, p1}, Lfixture/TailPreparation;->read(II)I
    move-result v0
    :read_end
    goto :adjust
    :typed
    move-exception v3
    const/16 v6, 0x14
    invoke-static {v6}, Lfixture/TailPreparation;->mark(I)V
    const/16 v0, 0x64
    :typed_end
    goto :adjust
    :adjust
    if-nez p2, :xor
    add-int/lit8 v0, v0, 0x1
    goto :delta
    :xor
    xor-int/lit8 v0, v0, 0x5
    :delta
    if-ltz p2, :sum
    const/4 p2, 0x2
    :sum
    add-int v2, v0, p2
    xor-int/lit8 v2, v2, 0x1
    :body_end
    const/16 v6, 0x7
    invoke-static {v6, p1}, Lfixture/TailPreparation;->cleanup(II)V
    invoke-static {p0, p1}, Lfixture/TailPreparation;->cleanup(II)V
    if-nez p3, :mark
    if-eqz p4, :done
    :mark
    const/16 v6, 0x32
    invoke-static {v6}, Lfixture/TailPreparation;->mark(I)V
    :done
    return v2
    :all
    move-exception v4
    const/16 v6, 0x7
    invoke-static {v6, p1}, Lfixture/TailPreparation;->cleanup(II)V
    invoke-static {p0, p1}, Lfixture/TailPreparation;->cleanup(II)V
    throw v4
    .catch Ljava/lang/NumberFormatException; {:read_start .. :read_end} :typed
    .catch Ljava/lang/NullPointerException; {:read_start .. :read_end} :typed
    .catchall {:read_start .. :read_end} :all
    .catchall {:typed .. :typed_end} :all
    .catchall {:adjust .. :body_end} :all
.end method

.method public static bypass(IIIII)I
    .locals 4
    :dispatch
    sparse-switch p2, :cases
    goto :read_start
    :zero
    const/16 v1, 0x9
    invoke-static {v1}, Lfixture/TailPreparation;->mark(I)V
    const/4 v0, -0x1
    goto :normal_cleanup
    :one
    const/4 v1, 0x1
    invoke-static {v1}, Lfixture/TailPreparation;->mark(I)V
    goto :read_start
    :two
    const/4 v1, 0x2
    invoke-static {v1}, Lfixture/TailPreparation;->mark(I)V
    goto :read_start
    :read_start
    invoke-static {p0, p1}, Lfixture/TailPreparation;->read(II)I
    move-result v0
    :read_end
    goto :plus
    :typed
    move-exception v2
    const/16 v1, 0x14
    invoke-static {v1}, Lfixture/TailPreparation;->mark(I)V
    const/16 v0, 0x64
    :typed_end
    :plus
    add-int/lit8 v0, v0, 0x2
    :normal_cleanup
    invoke-static {p0, p1}, Lfixture/TailPreparation;->cleanup(II)V
    add-int/lit8 v0, v0, 0x3
    if-nez p3, :mark
    if-eqz p4, :done
    :mark
    const/16 v1, 0x32
    invoke-static {v1}, Lfixture/TailPreparation;->mark(I)V
    :done
    return v0
    :all
    move-exception v3
    invoke-static {p0, p1}, Lfixture/TailPreparation;->cleanup(II)V
    throw v3
    .catchall {:dispatch .. :read_start} :all
    .catch Ljava/lang/NumberFormatException; {:read_start .. :read_end} :typed
    .catch Ljava/lang/NullPointerException; {:read_start .. :read_end} :typed
    .catchall {:read_start .. :read_end} :all
    .catchall {:typed .. :typed_end} :all
    .catchall {:plus .. :normal_cleanup} :all
    :cases
    .sparse-switch
        0x0 -> :zero
        0x1 -> :one
        0x2 -> :two
    .end sparse-switch
.end method

.method public static foreign(III)I
    .locals 4
    if-nez p2, :outside
    :read_start
    invoke-static {p0, p1}, Lfixture/TailPreparation;->read(II)I
    move-result v0
    :read_end
    goto :tail
    :outside
    add-int/lit8 v1, p0, 0x1
    invoke-static {v1, p1}, Lfixture/TailPreparation;->read(II)I
    move-result v0
    :tail
    add-int/lit8 v0, v0, 0x1
    :tail_end
    invoke-static {p0, p1}, Lfixture/TailPreparation;->cleanup(II)V
    const/16 v1, 0x32
    invoke-static {v1}, Lfixture/TailPreparation;->mark(I)V
    return v0
    :all
    move-exception v2
    invoke-static {p0, p1}, Lfixture/TailPreparation;->cleanup(II)V
    throw v2
    .catchall {:read_start .. :read_end} :all
.end method

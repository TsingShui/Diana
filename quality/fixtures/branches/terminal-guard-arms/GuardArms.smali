.class public final Lfixture/GuardArms;
.super Ljava/lang/Object;
.field public static trace:I

.method public static mark(I)V
    .locals 1
    sget v0, Lfixture/GuardArms;->trace:I
    mul-int/lit8 v0, v0, 0x1f
    add-int/2addr v0, p0
    sput v0, Lfixture/GuardArms;->trace:I
    return-void
.end method

.method public static probe(II)Z
    .locals 2
    add-int/lit8 v0, p0, 0xa
    invoke-static {v0}, Lfixture/GuardArms;->mark(I)V
    const/16 v0, 0x9
    if-ne p1, v0, :test
    new-instance v0, Ljava/lang/IllegalStateException;
    const-string v1, "test"
    invoke-direct {v0, v1}, Ljava/lang/IllegalStateException;-><init>(Ljava/lang/String;)V
    throw v0
    :test
    const/4 v0, 0x0
    if-nez p0, :done
    const/4 v0, 0x1
    :done
    return v0
.end method

.method public static read(II)I
    .locals 1
    add-int/lit8 v0, p0, 0x14
    invoke-static {v0}, Lfixture/GuardArms;->mark(I)V
    const/4 v0, 0x1
    if-eq p1, v0, :ise
    const/4 v0, 0x3
    if-eq p1, v0, :error
    add-int/lit8 v0, p0, 0x7
    return v0
    :ise
    new-instance v0, Ljava/lang/IllegalStateException;
    invoke-direct {v0}, Ljava/lang/IllegalStateException;-><init>()V
    throw v0
    :error
    new-instance v0, Ljava/lang/AssertionError;
    invoke-direct {v0}, Ljava/lang/AssertionError;-><init>()V
    throw v0
.end method

.method public static cleanup(II)V
    .locals 1
    add-int/lit8 v0, p0, 0x28
    invoke-static {v0}, Lfixture/GuardArms;->mark(I)V
    const/4 v0, 0x2
    if-eq p1, v0, :fail
    const/4 v0, 0x3
    if-ne p1, v0, :done
    :fail
    new-instance v0, Ljava/lang/IllegalArgumentException;
    invoke-direct {v0}, Ljava/lang/IllegalArgumentException;-><init>()V
    throw v0
    :done
    return-void
.end method

.method public static declared(II)I
    .locals 2
    invoke-static {p0, p1}, Lfixture/GuardArms;->probe(II)Z
    move-result v0
    if-nez v0, :early
    const/4 v0, 0x2
    invoke-static {v0, p1}, Lfixture/GuardArms;->read(II)I
    move-result v0
    invoke-static {v0}, Lfixture/GuardArms;->mark(I)V
    add-int/lit8 v1, v0, 0x1
    return v1
    :early
    const/4 v0, 0x0
    return v0
.end method

.method public static after(II)I
    .locals 3
    :body
    invoke-static {p0, p1}, Lfixture/GuardArms;->read(II)I
    move-result v0
    :body_end
    invoke-static {p0, p1}, Lfixture/GuardArms;->cleanup(II)V
    add-int/lit8 v2, v0, -0x8
    const/4 v1, 0x0
    invoke-static {v2, v1}, Lfixture/GuardArms;->probe(II)Z
    move-result v2
    if-eqz v2, :other
    return v0
    :other
    const/16 v2, 0x32
    invoke-static {v2}, Lfixture/GuardArms;->mark(I)V
    add-int/lit8 v0, v0, 0x1
    return v0
    :handler
    move-exception v1
    invoke-static {p0, p1}, Lfixture/GuardArms;->cleanup(II)V
    throw v1
    .catchall {:body .. :body_end} :handler
.end method

.method public static throwing(II)I
    .locals 2
    invoke-static {p0, p1}, Lfixture/GuardArms;->probe(II)Z
    move-result v0
    if-nez v0, :throw_guard
    const/16 v0, 0x32
    invoke-static {v0}, Lfixture/GuardArms;->mark(I)V
    invoke-static {p0, p1}, Lfixture/GuardArms;->read(II)I
    move-result v0
    return v0
    :throw_guard
    new-instance v0, Ljava/lang/IllegalStateException;
    const-string v1, "guard"
    invoke-direct {v0, v1}, Ljava/lang/IllegalStateException;-><init>(Ljava/lang/String;)V
    throw v0
.end method

.method public static select(III)I
    .locals 2
    move v0, p1
    if-eqz p0, :nine
    add-int/lit8 v1, p0, -0x1
    invoke-static {v1, p2}, Lfixture/GuardArms;->probe(II)Z
    move-result v1
    if-eqz v1, :join
    const/4 v1, 0x1
    invoke-static {v1, p2}, Lfixture/GuardArms;->read(II)I
    move-result v1
    return v1
    :nine
    const/16 p1, 0x9
    :join
    const/16 v1, 0x32
    invoke-static {v1}, Lfixture/GuardArms;->mark(I)V
    add-int v1, p1, v0
    return v1
.end method

.method public static outer(III)I
    .locals 2
    invoke-static {p0, p2}, Lfixture/GuardArms;->probe(II)Z
    move-result v0
    if-eqz v0, :other
    const/16 v0, 0x1e
    invoke-static {v0}, Lfixture/GuardArms;->mark(I)V
    const/4 v1, 0x7
    goto :join
    :other
    invoke-static {p1, p2}, Lfixture/GuardArms;->probe(II)Z
    move-result v0
    if-eqz v0, :body
    const/4 v0, 0x3
    invoke-static {v0, p2}, Lfixture/GuardArms;->read(II)I
    move-result v0
    return v0
    :body
    const/16 v0, 0x32
    invoke-static {v0}, Lfixture/GuardArms;->mark(I)V
    const/16 v1, 0x8
    :join
    const/16 v0, 0x46
    invoke-static {v0}, Lfixture/GuardArms;->mark(I)V
    return v1
.end method

.method public static recover(II)I
    .locals 2
    invoke-static {p0, p1}, Lfixture/GuardArms;->probe(II)Z
    move-result v0
    if-eqz v0, :other
    :body
    const/4 v0, 0x2
    invoke-static {v0, p1}, Lfixture/GuardArms;->read(II)I
    move-result v0
    :body_end
    return v0
    :handler
    move-exception v1
    const/16 v0, 0x3c
    invoke-static {v0}, Lfixture/GuardArms;->mark(I)V
    goto :join
    :other
    const/16 v0, 0x32
    invoke-static {v0}, Lfixture/GuardArms;->mark(I)V
    :join
    const/16 v0, 0x46
    invoke-static {v0}, Lfixture/GuardArms;->mark(I)V
    const/4 v0, 0x4
    return v0
    .catch Ljava/lang/IllegalStateException; {:body .. :body_end} :handler
.end method

.method public static loop(III)I
    .locals 4
    const/4 v0, 0x0
    const/4 v1, 0x0
    :body
    :head
    if-ge v0, p0, :body_end
    sub-int v2, v0, p1
    invoke-static {v2, p2}, Lfixture/GuardArms;->probe(II)Z
    move-result v2
    if-eqz v2, :other
    new-instance v2, Ljava/lang/IllegalStateException;
    const-string v3, "guard"
    invoke-direct {v2, v3}, Ljava/lang/IllegalStateException;-><init>(Ljava/lang/String;)V
    throw v2
    :other
    invoke-static {v0, p2}, Lfixture/GuardArms;->read(II)I
    move-result v2
    add-int/2addr v1, v2
    add-int/lit8 v0, v0, 0x1
    goto :head
    :body_end
    invoke-static {p0, p2}, Lfixture/GuardArms;->cleanup(II)V
    return v1
    :handler
    move-exception v2
    invoke-static {p0, p2}, Lfixture/GuardArms;->cleanup(II)V
    throw v2
    .catchall {:body .. :body_end} :handler
.end method

.method public static voided(I)V
    .locals 1
    const/4 v0, 0x0
    invoke-static {p0, v0}, Lfixture/GuardArms;->probe(II)Z
    move-result v0
    if-eqz v0, :other
    const/16 v0, 0x1e
    invoke-static {v0}, Lfixture/GuardArms;->mark(I)V
    return-void
    :other
    const/16 v0, 0x32
    invoke-static {v0}, Lfixture/GuardArms;->mark(I)V
    const/16 v0, 0x46
    invoke-static {v0}, Lfixture/GuardArms;->mark(I)V
    return-void
.end method

.class public final Lfixture/PrefixFrontiers;
.super Ljava/lang/Object;

.field public static trace:I

.method public static mark(I)V
    .locals 1
    sget v0, Lfixture/PrefixFrontiers;->trace:I
    mul-int/lit8 v0, v0, 0x1f
    add-int/2addr v0, p0
    sput v0, Lfixture/PrefixFrontiers;->trace:I
    return-void
.end method

.method public static read(II)I
    .locals 3
    add-int/lit8 v0, p0, 0xa
    invoke-static {v0}, Lfixture/PrefixFrontiers;->mark(I)V
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
    invoke-static {v0}, Lfixture/PrefixFrontiers;->mark(I)V
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

.method public static branch(III)I
    .locals 5
    :body_start
    sparse-switch p0, :lead_cases
    const/4 v1, 0x3
    invoke-static {v1}, Lfixture/PrefixFrontiers;->mark(I)V
    goto :inner_header
    :lead_zero
    const/4 v1, 0x1
    invoke-static {v1}, Lfixture/PrefixFrontiers;->mark(I)V
    goto :inner_header
    :lead_one
    const/4 v1, 0x2
    invoke-static {v1}, Lfixture/PrefixFrontiers;->mark(I)V
    :inner_header
    if-nez p1, :one_start
    :zero_start
    const/4 v2, 0x0
    invoke-static {v2, p2}, Lfixture/PrefixFrontiers;->read(II)I
    move-result v0
    :zero_end
    goto :after_inner
    :one_start
    const/4 v2, 0x1
    invoke-static {v2, p2}, Lfixture/PrefixFrontiers;->read(II)I
    move-result v0
    :one_end
    goto :after_inner
    :typed
    move-exception v3
    const/16 v1, 0x14
    invoke-static {v1}, Lfixture/PrefixFrontiers;->mark(I)V
    const/16 v0, 0x64
    :typed_end
    :after_inner
    const/16 v1, 0x28
    invoke-static {v1}, Lfixture/PrefixFrontiers;->mark(I)V
    :body_end
    const/4 v2, 0x2
    invoke-static {v2, p2}, Lfixture/PrefixFrontiers;->cleanup(II)V
    const/16 v1, 0x32
    invoke-static {v1}, Lfixture/PrefixFrontiers;->mark(I)V
    return v0
    :outer_handler
    move-exception v4
    const/4 v2, 0x2
    invoke-static {v2, p2}, Lfixture/PrefixFrontiers;->cleanup(II)V
    throw v4
    .catchall {:body_start .. :zero_start} :outer_handler
    .catch Ljava/lang/NumberFormatException; {:zero_start .. :zero_end} :typed
    .catch Ljava/lang/NullPointerException; {:zero_start .. :zero_end} :typed
    .catchall {:zero_start .. :zero_end} :outer_handler
    .catchall {:zero_end .. :one_start} :outer_handler
    .catch Ljava/lang/NumberFormatException; {:one_start .. :one_end} :typed
    .catch Ljava/lang/NullPointerException; {:one_start .. :one_end} :typed
    .catchall {:one_start .. :one_end} :outer_handler
    .catchall {:one_end .. :body_end} :outer_handler
    :lead_cases
    .sparse-switch
        0x0 -> :lead_zero
        0x1 -> :lead_one
    .end sparse-switch
.end method

.method public static split(II)I
    .locals 5
    :body_start
    if-nez p0, :one_prefix
    const/16 v1, 0x3c
    invoke-static {v1}, Lfixture/PrefixFrontiers;->mark(I)V
    :zero_start
    const/4 v2, 0x0
    invoke-static {v2, p1}, Lfixture/PrefixFrontiers;->read(II)I
    move-result v0
    :zero_end
    goto :after_inner
    :one_prefix
    const/16 v1, 0x3d
    invoke-static {v1}, Lfixture/PrefixFrontiers;->mark(I)V
    :one_start
    const/4 v2, 0x1
    invoke-static {v2, p1}, Lfixture/PrefixFrontiers;->read(II)I
    move-result v0
    :one_end
    goto :after_inner
    :typed
    move-exception v3
    const/16 v1, 0x14
    invoke-static {v1}, Lfixture/PrefixFrontiers;->mark(I)V
    const/16 v0, 0x64
    :typed_end
    :after_inner
    const/16 v1, 0x28
    invoke-static {v1}, Lfixture/PrefixFrontiers;->mark(I)V
    :body_end
    const/4 v2, 0x3
    invoke-static {v2, p1}, Lfixture/PrefixFrontiers;->cleanup(II)V
    const/16 v1, 0x32
    invoke-static {v1}, Lfixture/PrefixFrontiers;->mark(I)V
    return v0
    :outer_handler
    move-exception v4
    const/4 v2, 0x3
    invoke-static {v2, p1}, Lfixture/PrefixFrontiers;->cleanup(II)V
    throw v4
    .catchall {:body_start .. :zero_start} :outer_handler
    .catch Ljava/lang/NumberFormatException; {:zero_start .. :zero_end} :typed
    .catch Ljava/lang/NullPointerException; {:zero_start .. :zero_end} :typed
    .catchall {:zero_start .. :zero_end} :outer_handler
    .catchall {:zero_end .. :one_start} :outer_handler
    .catch Ljava/lang/NumberFormatException; {:one_start .. :one_end} :typed
    .catch Ljava/lang/NullPointerException; {:one_start .. :one_end} :typed
    .catchall {:one_start .. :one_end} :outer_handler
    .catchall {:one_end .. :body_end} :outer_handler
.end method

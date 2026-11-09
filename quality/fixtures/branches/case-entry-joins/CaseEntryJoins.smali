.class public final Lfixture/CaseEntryJoins;
.super Ljava/lang/Object;

.field public static trace:I

.method public static mark(I)V
    .locals 1
    sget v0, Lfixture/CaseEntryJoins;->trace:I
    mul-int/lit8 v0, v0, 0x1f
    add-int/2addr v0, p0
    sput v0, Lfixture/CaseEntryJoins;->trace:I
    return-void
.end method

.method public static read(II)I
    .locals 3
    add-int/lit8 v0, p0, 0xa
    invoke-static {v0}, Lfixture/CaseEntryJoins;->mark(I)V
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
    invoke-static {v0}, Lfixture/CaseEntryJoins;->mark(I)V
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

.method public static choice(II)I
    .locals 2
    sparse-switch p0, :cases
    const/16 v1, 0x9
    invoke-static {v1}, Lfixture/CaseEntryJoins;->mark(I)V
    const/4 v0, -0x1
    goto :done
    :one
    const/4 v1, 0x1
    invoke-static {v1}, Lfixture/CaseEntryJoins;->mark(I)V
    goto :three
    :two
    const/4 v1, 0x2
    invoke-static {v1}, Lfixture/CaseEntryJoins;->mark(I)V
    goto :three
    :five
    const/4 v1, 0x5
    invoke-static {v1}, Lfixture/CaseEntryJoins;->mark(I)V
    goto :three
    :four
    const/4 v1, 0x4
    invoke-static {v1}, Lfixture/CaseEntryJoins;->mark(I)V
    const/16 v0, 0x28
    goto :done
    :three
    const/4 v1, 0x3
    invoke-static {v1}, Lfixture/CaseEntryJoins;->mark(I)V
    if-eqz p1, :flag_zero
    const/4 v1, 0x7
    invoke-static {v1}, Lfixture/CaseEntryJoins;->mark(I)V
    const/16 v0, 0x14
    goto :done
    :flag_zero
    const/4 v1, 0x6
    invoke-static {v1}, Lfixture/CaseEntryJoins;->mark(I)V
    const/16 v0, 0xa
    :done
    const/16 v1, 0x32
    invoke-static {v1}, Lfixture/CaseEntryJoins;->mark(I)V
    return v0
    :cases
    .sparse-switch
        0x1 -> :one
        0x2 -> :two
        0x3 -> :three
        0x4 -> :four
        0x5 -> :five
    .end sparse-switch
.end method

.method public static matches(II)Z
    .locals 1
    add-int/lit8 v0, p0, 0x50
    invoke-static {v0}, Lfixture/CaseEntryJoins;->mark(I)V
    if-ne p0, p1, :different
    const/4 v0, 0x1
    return v0
    :different
    const/4 v0, 0x0
    return v0
.end method

.method public static nonCaseTail(II)I
    .locals 3
    sparse-switch p0, :cases
    const/4 v0, -0x1
    goto :done
    :one
    const/4 v1, 0x1
    invoke-static {v1, p1}, Lfixture/CaseEntryJoins;->matches(II)Z
    move-result v2
    if-eqz v2, :failed
    const/16 v0, 0xa
    goto :done
    :two
    const/4 v1, 0x2
    invoke-static {v1, p1}, Lfixture/CaseEntryJoins;->matches(II)Z
    move-result v2
    if-eqz v2, :failed
    const/16 v0, 0x14
    goto :done
    :three
    const/4 v1, 0x3
    invoke-static {v1, p1}, Lfixture/CaseEntryJoins;->matches(II)Z
    move-result v2
    if-eqz v2, :failed
    const/16 v0, 0x1e
    goto :done
    :failed
    const/4 v0, -0x2
    :done
    return v0
    :cases
    .sparse-switch
        0x1 -> :one
        0x2 -> :two
        0x3 -> :three
    .end sparse-switch
.end method

.method public static route(III)I
    .locals 5
    const/4 v2, 0x0
    :body_start
    sparse-switch p0, :key_cases
    const/16 v1, 0x9
    invoke-static {v1}, Lfixture/CaseEntryJoins;->mark(I)V
    const/4 v0, -0x1
    goto :normal_done
    :one
    const/4 v1, 0x1
    invoke-static {v1}, Lfixture/CaseEntryJoins;->mark(I)V
    goto :three
    :two
    const/4 v1, 0x2
    invoke-static {v1}, Lfixture/CaseEntryJoins;->mark(I)V
    :three
    :read_start
    invoke-static {v2, p2}, Lfixture/CaseEntryJoins;->read(II)I
    move-result v0
    :read_end
    goto :flag_switch
    :typed
    move-exception v3
    const/16 v1, 0x14
    invoke-static {v1}, Lfixture/CaseEntryJoins;->mark(I)V
    const/16 v0, 0x64
    :typed_end
    :flag_switch
    sparse-switch p1, :flag_cases
    const/4 v1, 0x5
    invoke-static {v1}, Lfixture/CaseEntryJoins;->mark(I)V
    add-int/lit8 v0, v0, -0xa
    goto :normal_done
    :flag_zero
    const/4 v1, 0x4
    invoke-static {v1}, Lfixture/CaseEntryJoins;->mark(I)V
    add-int/lit8 v0, v0, 0xa
    :normal_done
    :body_end
    invoke-static {v2, p2}, Lfixture/CaseEntryJoins;->cleanup(II)V
    const/16 v1, 0x28
    invoke-static {v1}, Lfixture/CaseEntryJoins;->mark(I)V
    return v0
    :outer_handler
    move-exception v4
    invoke-static {v2, p2}, Lfixture/CaseEntryJoins;->cleanup(II)V
    throw v4
    .catchall {:body_start .. :read_start} :outer_handler
    .catch Ljava/lang/NumberFormatException; {:read_start .. :read_end} :typed
    .catch Ljava/lang/NullPointerException; {:read_start .. :read_end} :typed
    .catchall {:read_start .. :read_end} :outer_handler
    .catchall {:read_end .. :body_end} :outer_handler
    :key_cases
    .sparse-switch
        0x1 -> :one
        0x2 -> :two
        0x3 -> :three
    .end sparse-switch
    :flag_cases
    .sparse-switch
        0x0 -> :flag_zero
    .end sparse-switch
.end method

.method public static parameterJoin(II)I
    .locals 1
    sparse-switch p0, :cases
    const/16 v0, 0x9
    invoke-static {v0}, Lfixture/CaseEntryJoins;->mark(I)V
    const/4 p1, -0x1
    goto :done
    :one
    const/4 v0, 0x1
    invoke-static {v0}, Lfixture/CaseEntryJoins;->mark(I)V
    add-int/lit8 p1, p1, 0x1
    goto :three
    :two
    const/4 v0, 0x2
    invoke-static {v0}, Lfixture/CaseEntryJoins;->mark(I)V
    add-int/lit8 p1, p1, 0x2
    :three
    const/4 v0, 0x3
    invoke-static {v0}, Lfixture/CaseEntryJoins;->mark(I)V
    add-int/lit8 p1, p1, 0x3
    :done
    return p1
    :cases
    .sparse-switch
        0x1 -> :one
        0x2 -> :two
        0x3 -> :three
    .end sparse-switch
.end method

.method public static literalCycle(I)I
    .locals 2
    const/4 v0, 0x2
    sparse-switch p0, :cases
    const/16 v1, 0x9
    invoke-static {v1}, Lfixture/CaseEntryJoins;->mark(I)V
    goto :done
    :one
    const/4 v1, 0x1
    invoke-static {v1}, Lfixture/CaseEntryJoins;->mark(I)V
    if-eqz v0, :done
    add-int/lit8 v0, v0, -0x1
    goto :two
    :two
    const/4 v1, 0x2
    invoke-static {v1}, Lfixture/CaseEntryJoins;->mark(I)V
    if-eqz v0, :done
    add-int/lit8 v0, v0, -0x1
    goto :one
    :done
    return v0
    :cases
    .sparse-switch
        0x1 -> :one
        0x2 -> :two
    .end sparse-switch
.end method

.method public static cyclic(II)I
    .locals 1
    sparse-switch p0, :cases
    const/16 v0, 0x9
    invoke-static {v0}, Lfixture/CaseEntryJoins;->mark(I)V
    goto :done
    :one
    const/4 v0, 0x1
    invoke-static {v0}, Lfixture/CaseEntryJoins;->mark(I)V
    if-eqz p1, :done
    add-int/lit8 p1, p1, -0x1
    goto :two
    :two
    const/4 v0, 0x2
    invoke-static {v0}, Lfixture/CaseEntryJoins;->mark(I)V
    if-eqz p1, :done
    add-int/lit8 p1, p1, -0x1
    goto :one
    :done
    return p1
    :cases
    .sparse-switch
        0x1 -> :one
        0x2 -> :two
    .end sparse-switch
.end method

.class public final Lfixture/TailCompletion;
.super Ljava/lang/Object;

.field public static trace:I

.method public static mark(I)V
    .locals 1
    sget v0, Lfixture/TailCompletion;->trace:I
    mul-int/lit8 v0, v0, 0x1f
    add-int/2addr v0, p0
    sput v0, Lfixture/TailCompletion;->trace:I
    return-void
.end method

.method public static read(II)I
    .locals 3
    add-int/lit8 v0, p0, 0x64
    invoke-static {v0}, Lfixture/TailCompletion;->mark(I)V
    const/4 v0, 0x1
    if-ne p1, v0, :normal
    new-instance v1, Ljava/lang/IllegalStateException;
    const-string v2, "read"
    invoke-direct {v1, v2}, Ljava/lang/IllegalStateException;-><init>(Ljava/lang/String;)V
    throw v1
    :normal
    return p0
.end method

.method public static matches(II)Z
    .locals 1
    add-int/lit8 v0, p0, 0x50
    invoke-static {v0}, Lfixture/TailCompletion;->mark(I)V
    if-ne p0, p1, :different
    const/4 v0, 0x1
    return v0
    :different
    const/4 v0, 0x0
    return v0
.end method

.method public static decorated(III)I
    .locals 5
    sparse-switch p0, :first_cases
    goto :shared_default
    :first
    const/4 v1, 0x0
    invoke-static {v1, p1}, Lfixture/TailCompletion;->matches(II)Z
    move-result v2
    if-eqz v2, :shared_default
    const/4 v0, 0x0
    goto :second_switch
    :second
    const/4 v1, 0x1
    invoke-static {v1, p1}, Lfixture/TailCompletion;->matches(II)Z
    move-result v2
    if-eqz v2, :shared_default
    const/4 v0, 0x1
    goto :second_switch
    :third
    const/4 v1, 0x2
    invoke-static {v1, p1}, Lfixture/TailCompletion;->matches(II)Z
    move-result v2
    if-eqz v2, :shared_default
    const/4 v0, 0x2
    goto :second_switch
    :shared_default
    const/4 v0, -0x1
    :second_switch
    sparse-switch v0, :value_cases
    const/4 v4, -0x1
    goto :read_start
    :value_first
    const/16 v4, 0xa
    goto :read_start
    :value_second
    const/16 v4, 0x14
    goto :read_start
    :value_third
    const/16 v4, 0x1e
    :read_start
    invoke-static {v4, p2}, Lfixture/TailCompletion;->read(II)I
    move-result v4
    :read_end
    const/16 v1, 0x5a
    invoke-static {v1}, Lfixture/TailCompletion;->mark(I)V
    const/16 v1, 0x5b
    invoke-static {v1}, Lfixture/TailCompletion;->mark(I)V
    return v4
    :handler
    move-exception v3
    const/16 v1, 0x5a
    invoke-static {v1}, Lfixture/TailCompletion;->mark(I)V
    throw v3
    .catchall {:read_start .. :read_end} :handler
    :first_cases
    .sparse-switch
        0x0 -> :first
        0x1 -> :second
        0x2 -> :third
    .end sparse-switch
    :value_cases
    .sparse-switch
        0x0 -> :value_first
        0x1 -> :value_second
        0x2 -> :value_third
    .end sparse-switch
.end method

.method public static select(III)I
    .locals 5
    :start
    sparse-switch p0, :cases
    goto :shared_default
    :first
    const/16 v1, 0xa
    invoke-static {v1}, Lfixture/TailCompletion;->mark(I)V
    if-nez p1, :shared_default
    const/16 v0, 0xa
    invoke-static {v0, p2}, Lfixture/TailCompletion;->read(II)I
    move-result v0
    goto :join
    :second
    const/16 v1, 0xb
    invoke-static {v1}, Lfixture/TailCompletion;->mark(I)V
    const/4 v1, 0x1
    if-ne p1, v1, :shared_default
    const/16 v0, 0x14
    invoke-static {v0, p2}, Lfixture/TailCompletion;->read(II)I
    move-result v0
    goto :join
    :third
    const/16 v1, 0xc
    invoke-static {v1}, Lfixture/TailCompletion;->mark(I)V
    const/4 v1, 0x2
    if-ne p1, v1, :shared_default
    const/16 v0, 0x1e
    invoke-static {v0, p2}, Lfixture/TailCompletion;->read(II)I
    move-result v0
    goto :join
    :shared_default
    const/4 v0, -0x1
    :join
    :body_end
    const/16 v1, 0x1e
    invoke-static {v1}, Lfixture/TailCompletion;->mark(I)V
    const/16 v1, 0x28
    invoke-static {v1}, Lfixture/TailCompletion;->mark(I)V
    return v0
    :handler
    move-exception v3
    const/16 v1, 0x1e
    invoke-static {v1}, Lfixture/TailCompletion;->mark(I)V
    throw v3
    .catchall {:start .. :body_end} :handler
    :cases
    .sparse-switch
        0x0 -> :first
        0x1 -> :second
        0x2 -> :third
    .end sparse-switch
.end method

.method public static caught(I)I
    .locals 3
    :body
    const/16 v1, 0x32
    invoke-static {v1}, Lfixture/TailCompletion;->mark(I)V
    if-nez p0, :normal
    new-instance v2, Ljava/lang/NumberFormatException;
    invoke-direct {v2}, Ljava/lang/NumberFormatException;-><init>()V
    throw v2
    :normal
    const/4 v0, 0x7
    :body_end
    goto :after
    :handler
    move-exception v2
    const/16 v1, 0x33
    invoke-static {v1}, Lfixture/TailCompletion;->mark(I)V
    const/16 v0, 0x9
    :handler_end
    goto :after
    :after
    const/16 v1, 0x34
    invoke-static {v1}, Lfixture/TailCompletion;->mark(I)V
    return v0
    .catch Ljava/lang/NumberFormatException; {:body .. :body_end} :handler
.end method

.method public static breakScope(I)I
    .locals 2
    sparse-switch p0, :cases
    const/16 v0, 0x3d
    invoke-static {v0}, Lfixture/TailCompletion;->mark(I)V
    goto :after
    :first
    const/16 v0, 0x3c
    invoke-static {v0}, Lfixture/TailCompletion;->mark(I)V
    goto :after
    :after
    const/16 v0, 0x3e
    invoke-static {v0}, Lfixture/TailCompletion;->mark(I)V
    const/4 v1, 0x7
    return v1
    :cases
    .sparse-switch
        0x0 -> :first
    .end sparse-switch
.end method

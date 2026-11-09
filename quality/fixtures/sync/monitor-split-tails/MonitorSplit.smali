.class public final Lfixture/MonitorSplit;
.super Ljava/lang/Object;

.field public static trace:I

.method public static mark(I)V
    .locals 1
    sget v0, Lfixture/MonitorSplit;->trace:I
    mul-int/lit8 v0, v0, 0x1f
    add-int/2addr v0, p0
    sput v0, Lfixture/MonitorSplit;->trace:I
    return-void
.end method

.method public static read(II)I
    .locals 3
    add-int/lit8 v0, p0, 0xa
    invoke-static {v0}, Lfixture/MonitorSplit;->mark(I)V
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
    const/4 v0, 0x5
    return v0
.end method

.method public static cleanup(II)V
    .locals 3
    add-int/lit8 v0, p0, 0x1e
    invoke-static {v0}, Lfixture/MonitorSplit;->mark(I)V
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

.method public static finish(III)I
    .locals 3
    add-int/lit8 v0, p0, 0x28
    invoke-static {v0}, Lfixture/MonitorSplit;->mark(I)V
    const/4 v0, 0x7
    if-ne p2, v0, :choose
    new-instance v1, Ljava/lang/IllegalStateException;
    const-string v2, "tail"
    invoke-direct {v1, v2}, Ljava/lang/IllegalStateException;-><init>(Ljava/lang/String;)V
    throw v1
    :choose
    if-eqz p0, :first
    const/4 v0, 0x1
    if-eq p0, v0, :second
    add-int/lit8 v0, p1, -0x3
    return v0
    :first
    add-int/lit8 v0, p1, 0x1
    return v0
    :second
    mul-int/lit8 v0, p1, 0x2
    return v0
.end method

.method public static transfer(Ljava/lang/Object;II)I
    .locals 4
    monitor-enter p0
    :body_start
    const/4 v3, 0x5
    invoke-static {v3}, Lfixture/MonitorSplit;->mark(I)V
    if-eqz p1, :alias
    add-int/lit8 v0, p2, 0x1
    :body_end
    monitor-exit p0
    goto :join
    :alias
    move v0, p2
    :alias_end
    monitor-exit p0
    goto :join
    :join
    add-int v1, v0, p2
    return v1
    :handler
    move-exception v2
    monitor-exit p0
    throw v2
    .catchall {:body_start .. :body_end} :handler
    .catchall {:alias .. :alias_end} :handler
.end method

.method public static split(Ljava/lang/Object;Ljava/lang/Object;II)I
    .locals 7
    const/4 v6, 0x0
    monitor-enter p0
    :inner_acquire
    monitor-enter p1
    :inner_acquired
    :read_start
    invoke-static {v6, p3}, Lfixture/MonitorSplit;->read(II)I
    move-result v0
    :read_end
    goto :normal_cleanup
    :typed
    move-exception v2
    const/16 v1, 0x14
    invoke-static {v1}, Lfixture/MonitorSplit;->mark(I)V
    const/16 v0, 0x64
    :typed_end
    goto :normal_cleanup
    :normal_cleanup
    invoke-static {v6, p3}, Lfixture/MonitorSplit;->cleanup(II)V
    if-eqz p2, :release_a
    const/4 v1, 0x1
    if-eq p2, v1, :release_b
    goto :release_c
    :held_end
    :release_a
    monitor-exit p1
    goto :tail_a
    :release_b
    monitor-exit p1
    goto :tail_b
    :release_c
    monitor-exit p1
    goto :tail_c
    :tail_a
    const/4 v1, 0x0
    invoke-static {v1, v0, p3}, Lfixture/MonitorSplit;->finish(III)I
    move-result v0
    goto :outer_release
    :tail_b
    const/4 v1, 0x1
    invoke-static {v1, v0, p3}, Lfixture/MonitorSplit;->finish(III)I
    move-result v0
    goto :outer_release
    :tail_c
    const/4 v1, 0x2
    invoke-static {v1, v0, p3}, Lfixture/MonitorSplit;->finish(III)I
    move-result v0
    :outer_body_end
    :outer_release
    monitor-exit p0
    return v0
    :cleanup_handler
    move-exception v3
    invoke-static {v6, p3}, Lfixture/MonitorSplit;->cleanup(II)V
    throw v3
    :cleanup_end
    :inner_handler
    move-exception v4
    monitor-exit p1
    throw v4
    :inner_handler_end
    :outer_handler
    move-exception v5
    monitor-exit p0
    throw v5
    .catch Ljava/lang/NumberFormatException; {:read_start .. :read_end} :typed
    .catch Ljava/lang/NullPointerException; {:read_start .. :read_end} :typed
    .catchall {:read_start .. :read_end} :cleanup_handler
    .catchall {:typed .. :typed_end} :cleanup_handler
    .catchall {:normal_cleanup .. :held_end} :inner_handler
    .catchall {:cleanup_handler .. :cleanup_end} :inner_handler
    .catchall {:inner_acquire .. :inner_acquired} :outer_handler
    .catchall {:release_a .. :outer_body_end} :outer_handler
    .catchall {:inner_handler .. :inner_handler_end} :outer_handler
.end method

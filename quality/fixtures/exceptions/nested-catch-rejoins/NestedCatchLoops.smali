.class public final Lfixture/NestedCatchLoops;
.super Ljava/lang/Object;

.method public static run(Ljava/lang/Object;II)I
    .locals 8
    const/4 v0, 0x0
    const/4 v2, 0x0
    :outer
    if-ge v0, p1, :done
    const/4 v1, 0x0
    :inner
    const/4 v6, 0x3
    if-ge v1, v6, :outer_update
    mul-int/lit8 v3, v0, 0x3
    add-int/2addr v3, v1
    monitor-enter p0
    :body
    invoke-static {v3, p2}, Lfixture/LoopCatchFinally;->read(II)I
    move-result v4
    :body_end
    goto :normal_cleanup
    :typed
    move-exception v5
    const/16 v6, 0x14
    invoke-static {v6}, Lfixture/LoopCatchFinally;->mark(I)V
    const/16 v4, 0x64
    :typed_end
    goto :normal_cleanup
    :normal_cleanup
    invoke-static {v3, p2}, Lfixture/LoopCatchFinally;->cleanup(II)V
    add-int/2addr v2, v4
    :before_exit
    monitor-exit p0
    :release_done
    add-int/lit8 v1, v1, 0x1
    goto :inner
    :outer_update
    add-int/lit8 v0, v0, 0x1
    goto :outer
    :done
    return v2
    :finally_handler
    move-exception v7
    invoke-static {v3, p2}, Lfixture/LoopCatchFinally;->cleanup(II)V
    throw v7
    :sync_handler
    move-exception v5
    monitor-exit p0
    throw v5
    .catch Ljava/lang/NumberFormatException; {:body .. :body_end} :typed
    .catch Ljava/lang/NullPointerException; {:body .. :body_end} :typed
    .catchall {:body .. :body_end} :finally_handler
    .catchall {:typed .. :typed_end} :finally_handler
    .catchall {:normal_cleanup .. :before_exit} :sync_handler
    .catchall {:finally_handler .. :sync_handler} :sync_handler
.end method

.method public static foreign(II)I
    .locals 5
    const/4 v0, 0x0
    const/4 v1, 0x0
    const/4 v3, 0x0
    :before
    invoke-static {v3, p1}, Lfixture/LoopCatchFinally;->read(II)I
    move-result v4
    :before_end
    :loop
    if-ge v0, p0, :done
    :body
    invoke-static {v0, p1}, Lfixture/LoopCatchFinally;->read(II)I
    move-result v2
    :body_end
    :join
    add-int/2addr v1, v2
    add-int/lit8 v0, v0, 0x1
    goto :loop
    :done
    return v1
    :shared
    move-exception v4
    const/16 v3, 0x14
    invoke-static {v3}, Lfixture/LoopCatchFinally;->mark(I)V
    const/16 v2, 0x64
    goto :join
    .catch Ljava/lang/NumberFormatException; {:before .. :before_end} :shared
    .catch Ljava/lang/NullPointerException; {:before .. :before_end} :shared
    .catch Ljava/lang/NumberFormatException; {:body .. :body_end} :shared
    .catch Ljava/lang/NullPointerException; {:body .. :body_end} :shared
.end method

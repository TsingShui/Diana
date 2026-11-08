.class public final Lfixture/SyncFinallyReturn;
.super Ljava/lang/Object;

.method public static native open(I)I
.end method

.method public static native read(I)I
.end method

.method public static native close(I)V
.end method

.method public static native record(I)V
.end method

# The finally close is inside the monitor scope. The early return and the normal
# path share that close, then the early path returns while the normal path records
# one more effect before the release. A second catch-all covers the whole remaining
# synchronized region, including the body handler, so both a failing close and the
# body rethrow release the monitor before leaving.
.method public static syncFinally(Ljava/lang/Object;I)I
    .locals 5
    const/4 v4, 0x0
    invoke-static {v4}, Lfixture/SyncFinallyReturn;->open(I)I
    move-result v0
    monitor-enter p0
    :body
    invoke-static {v0}, Lfixture/SyncFinallyReturn;->read(I)I
    move-result v1
    :body_end
    :normal_close
    invoke-static {v0}, Lfixture/SyncFinallyReturn;->close(I)V
    const/4 v4, 0x1
    if-eq p1, v4, :early_exit
    const/4 v4, 0x0
    invoke-static {v4}, Lfixture/SyncFinallyReturn;->record(I)V
    :early_exit
    monitor-exit p0
    return v1
    :body_handler
    move-exception v2
    :cleanup
    invoke-static {v0}, Lfixture/SyncFinallyReturn;->close(I)V
    :cleanup_end
    throw v2
    :sync_handler
    move-exception v3
    monitor-exit p0
    throw v3
    .catchall {:body .. :body_end} :body_handler
    .catchall {:normal_close .. :sync_handler} :sync_handler
.end method

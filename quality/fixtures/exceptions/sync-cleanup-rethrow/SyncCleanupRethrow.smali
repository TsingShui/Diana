.class public final Lfixture/SyncCleanupRethrow;
.super Ljava/lang/Object;

.method public static native open(I)I
.end method

.method public static native read(I)I
.end method

.method public static native close(I)V
.end method

# The inner body is protected by the resource catch, which closes and suppresses
# while the monitor is still held. The catch handler itself is protected by the
# monitor catch-all, so every path out of the catch releases the monitor first.
# The shared normal close sits after monitor-exit and is not protected by either
# handler.
.method public static guarded(Ljava/lang/Object;I)I
    .locals 5
    const/4 v4, 0x0
    invoke-static {v4}, Lfixture/SyncCleanupRethrow;->open(I)I
    move-result v0
    const/4 v1, 0x0
    monitor-enter p0
    :sync_start
    :body
    invoke-static {v0}, Lfixture/SyncCleanupRethrow;->read(I)I
    move-result v1
    const/4 v4, 0x1
    if-ne p1, v4, :body_end
    new-instance v4, Ljava/lang/IllegalStateException;
    const-string v3, "body"
    invoke-direct {v4, v3}, Ljava/lang/IllegalStateException;-><init>(Ljava/lang/String;)V
    throw v4
    :body_end
    monitor-exit p0
    :sync_end
    invoke-static {v0}, Lfixture/SyncCleanupRethrow;->close(I)V
    return v1
    :catch_handler
    move-exception v2
    :close_cleanup
    invoke-static {v0}, Lfixture/SyncCleanupRethrow;->close(I)V
    :close_cleanup_end
    goto :rethrow_primary
    :suppressed_handler
    move-exception v3
    invoke-virtual {v2, v3}, Ljava/lang/Throwable;->addSuppressed(Ljava/lang/Throwable;)V
    :rethrow_primary
    throw v2
    :sync_handler
    move-exception v4
    monitor-exit p0
    throw v4
    .catchall {:body .. :body_end} :catch_handler
    .catchall {:close_cleanup .. :close_cleanup_end} :suppressed_handler
    .catchall {:close_cleanup_end .. :sync_handler} :sync_handler
.end method

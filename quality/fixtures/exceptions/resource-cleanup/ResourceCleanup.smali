.class public final Lfixture/ResourceCleanup;
.super Ljava/lang/Object;

.method public static native open(I)I
.end method

.method public static native read(I)I
.end method

.method public static native close(I)V
.end method

.method public static loop(I)I
    .locals 6
    const/4 v0, 0x0
    const/4 v1, 0x0
    :loop
    if-ge v0, p0, :done
    invoke-static {v0}, Lfixture/ResourceCleanup;->open(I)I
    move-result v2
    :body
    invoke-static {v2}, Lfixture/ResourceCleanup;->read(I)I
    move-result v3
    add-int/2addr v1, v3
    :body_end
    invoke-static {v2}, Lfixture/ResourceCleanup;->close(I)V
    add-int/lit8 v0, v0, 0x1
    goto :loop
    :handler
    move-exception v4
    :cleanup
    invoke-static {v2}, Lfixture/ResourceCleanup;->close(I)V
    :cleanup_end
    goto :rethrow
    :suppressed
    move-exception v5
    invoke-virtual {v4, v5}, Ljava/lang/Throwable;->addSuppressed(Ljava/lang/Throwable;)V
    :rethrow
    throw v4
    :done
    return v1
    .catchall {:body .. :body_end} :handler
    .catchall {:cleanup .. :cleanup_end} :suppressed
.end method

.method public static nested(I)I
    .locals 6
    const/4 v0, 0x1
    invoke-static {v0}, Lfixture/ResourceCleanup;->open(I)I
    move-result v0
    :outer
    const/4 v1, 0x2
    invoke-static {v1}, Lfixture/ResourceCleanup;->open(I)I
    move-result v1
    :body
    invoke-static {v0}, Lfixture/ResourceCleanup;->read(I)I
    move-result v2
    invoke-static {v1}, Lfixture/ResourceCleanup;->read(I)I
    move-result v3
    add-int/2addr v2, v3
    add-int/2addr v2, p0
    :body_end
    invoke-static {v1}, Lfixture/ResourceCleanup;->close(I)V
    :outer_end
    invoke-static {v0}, Lfixture/ResourceCleanup;->close(I)V
    return v2
    :inner_handler
    move-exception v4
    :inner_cleanup
    invoke-static {v1}, Lfixture/ResourceCleanup;->close(I)V
    :inner_cleanup_end
    goto :inner_rethrow
    :inner_suppressed
    move-exception v5
    invoke-virtual {v4, v5}, Ljava/lang/Throwable;->addSuppressed(Ljava/lang/Throwable;)V
    :inner_rethrow
    throw v4
    :outer_handler
    move-exception v4
    :outer_cleanup
    invoke-static {v0}, Lfixture/ResourceCleanup;->close(I)V
    :outer_cleanup_end
    goto :outer_rethrow
    :outer_suppressed
    move-exception v5
    invoke-virtual {v4, v5}, Ljava/lang/Throwable;->addSuppressed(Ljava/lang/Throwable;)V
    :outer_rethrow
    throw v4
    .catchall {:outer .. :body} :outer_handler
    .catchall {:body .. :body_end} :inner_handler
    .catchall {:body_end .. :outer_end} :outer_handler
    .catchall {:inner_cleanup .. :inner_cleanup_end} :inner_suppressed
    .catchall {:inner_cleanup_end .. :outer_handler} :outer_handler
    .catchall {:outer_cleanup .. :outer_cleanup_end} :outer_suppressed
.end method

# The same capture receives a read after the loop as well as reads inside it.
.method public static sharedHandler(I)I
    .locals 6
    const/4 v0, 0x0
    invoke-static {v0}, Lfixture/ResourceCleanup;->open(I)I
    move-result v2
    const/4 v1, 0x0
    :loop
    if-ge v0, p0, :after
    :body
    invoke-static {v2}, Lfixture/ResourceCleanup;->read(I)I
    move-result v3
    add-int/2addr v1, v3
    :body_end
    add-int/lit8 v0, v0, 0x1
    goto :loop
    :after
    invoke-static {v2}, Lfixture/ResourceCleanup;->read(I)I
    move-result v3
    :after_end
    add-int/2addr v1, v3
    invoke-static {v2}, Lfixture/ResourceCleanup;->close(I)V
    return v1
    :handler
    move-exception v4
    :cleanup
    invoke-static {v2}, Lfixture/ResourceCleanup;->close(I)V
    :cleanup_end
    goto :rethrow
    :suppressed
    move-exception v5
    invoke-virtual {v4, v5}, Ljava/lang/Throwable;->addSuppressed(Ljava/lang/Throwable;)V
    :rethrow
    throw v4
    .catchall {:body .. :body_end} :handler
    .catchall {:after .. :after_end} :handler
    .catchall {:cleanup .. :cleanup_end} :suppressed
.end method

.class public final Lfixture/ResponseClose;
.super Ljava/lang/Object;

.method public static native acquire(I)Lfixture/ViewHandle;
.end method
.method public static native log(Ljava/lang/Throwable;)V
.end method

.method public static response(I)I
    .registers 7
    const/4 v0, 0
    const/4 v1, -1
    :try_start
    invoke-static {p0}, Lfixture/ResponseClose;->acquire(I)Lfixture/ViewHandle;
    move-result-object v0
    invoke-virtual {v0}, Lfixture/ViewHandle;->ready()Z
    move-result v3
    if-eqz v3, :try_end
    invoke-virtual {v0}, Lfixture/ViewHandle;->read()I
    move-result v1
    :try_end
    if-eqz v0, :return_result
    invoke-virtual {v0}, Lfixture/ViewHandle;->close()V
    :return_result
    return v1
    :interrupted
    move-exception v2
    :interrupt_start
    throw v2
    :interrupt_end
    :io
    move-exception v2
    :log_start
    invoke-static {v2}, Lfixture/ResponseClose;->log(Ljava/lang/Throwable;)V
    :log_end
    if-eqz v0, :return_empty
    invoke-virtual {v0}, Lfixture/ViewHandle;->close()V
    :return_empty
    const/4 v1, -1
    return v1
    :cleanup_exception
    move-exception v2
    if-eqz v0, :rethrow
    invoke-virtual {v0}, Lfixture/ViewHandle;->close()V
    :rethrow
    throw v2
    .catch Ljava/io/InterruptedIOException; {:try_start .. :try_end} :interrupted
    .catch Ljava/io/IOException; {:try_start .. :try_end} :io
    .catchall {:try_start .. :try_end} :cleanup_exception
    .catchall {:interrupt_start .. :interrupt_end} :cleanup_exception
    .catchall {:log_start .. :log_end} :cleanup_exception
.end method

.method public static resource(I)I
    .registers 6
    invoke-static {p0}, Lfixture/ResponseClose;->acquire(I)Lfixture/ViewHandle;
    move-result-object v0
    :body_start
    invoke-virtual {v0}, Lfixture/ViewHandle;->read()I
    move-result v1
    :body_end
    if-eqz v0, :return_value
    invoke-virtual {v0}, Lfixture/ViewHandle;->close()V
    :return_value
    return v1
    :primary
    move-exception v2
    if-eqz v0, :throw_primary
    :close_start
    invoke-virtual {v0}, Lfixture/ViewHandle;->close()V
    :close_end
    goto :throw_primary
    :secondary
    move-exception v3
    invoke-virtual {v2, v3}, Ljava/lang/Throwable;->addSuppressed(Ljava/lang/Throwable;)V
    :throw_primary
    throw v2
    .catchall {:body_start .. :body_end} :primary
    .catchall {:close_start .. :close_end} :secondary
.end method

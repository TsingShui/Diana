.class public final Lfixture/CleanupContexts;
.super Ljava/lang/Object;

.method public static native acquire(I)Lfixture/ContextHandle;
.end method
.method public static native log(Ljava/lang/Throwable;)V
.end method
.method public static native note(I)V
.end method

.method public static split(I)I
    .registers 7
    const/4 v0, 0
    :pre_start
    invoke-static {p0}, Lfixture/CleanupContexts;->acquire(I)Lfixture/ContextHandle;
    move-result-object v0
    :pre_end
    :post_start
    invoke-virtual {v0}, Lfixture/ContextHandle;->ready()Z
    move-result v1
    if-eqz v1, :normal_empty
    invoke-virtual {v0}, Lfixture/ContextHandle;->read()I
    move-result v2
    if-ltz v2, :normal_empty
    :post_end
    invoke-virtual {v0}, Lfixture/ContextHandle;->close()V
    return v2
    :normal_empty
    invoke-virtual {v0}, Lfixture/ContextHandle;->close()V
    :empty_return
    const/4 v2, -1
    return v2
    :post_all
    move-exception v5
    move-object v3, v0
    goto :shared_cleanup
    :post_io
    move-exception v4
    goto :shared_log
    :post_interrupted
    move-exception v4
    goto :shared_rethrow
    :pre_all
    move-exception v5
    move-object v3, v0
    goto :shared_cleanup
    :pre_io
    move-exception v4
    goto :shared_log
    :pre_interrupted
    move-exception v4
    goto :shared_rethrow
    :shared_log
    invoke-static {v4}, Lfixture/CleanupContexts;->log(Ljava/lang/Throwable;)V
    :shared_log_end
    if-eqz v0, :empty_return
    goto :normal_empty
    :shared_rethrow
    throw v4
    :shared_rethrow_end
    :shared_cleanup
    if-eqz v3, :pending_throw
    invoke-virtual {v3}, Lfixture/ContextHandle;->close()V
    :pending_throw
    throw v5
    .catch Ljava/io/InterruptedIOException; {:pre_start .. :pre_end} :pre_interrupted
    .catch Ljava/io/IOException; {:pre_start .. :pre_end} :pre_io
    .catchall {:pre_start .. :pre_end} :pre_all
    .catch Ljava/io/InterruptedIOException; {:post_start .. :post_end} :post_interrupted
    .catch Ljava/io/IOException; {:post_start .. :post_end} :post_io
    .catchall {:post_start .. :post_end} :post_all
    .catchall {:shared_log .. :shared_log_end} :post_all
    .catchall {:shared_rethrow .. :shared_rethrow_end} :pre_all
.end method

.method public static resource(I)I
    .registers 6
    :acquire_start
    invoke-static {p0}, Lfixture/CleanupContexts;->acquire(I)Lfixture/ContextHandle;
    move-result-object v0
    :acquire_end
    :body_start
    invoke-virtual {v0}, Lfixture/ContextHandle;->read()I
    move-result v1
    if-lez v1, :zero
    invoke-static {v1}, Lfixture/CleanupContexts;->note(I)V
    :body_end
    :normal_positive_start
    if-eqz v0, :positive_return
    invoke-virtual {v0}, Lfixture/ContextHandle;->close()V
    :positive_return
    return v1
    :normal_positive_end
    :zero
    if-eqz v0, :zero_return
    :normal_zero_start
    invoke-virtual {v0}, Lfixture/ContextHandle;->close()V
    :normal_zero_end
    :zero_return
    const/4 v1, 0
    return v1
    :primary
    move-exception v2
    if-eqz v0, :primary_throw
    :close_start
    invoke-virtual {v0}, Lfixture/ContextHandle;->close()V
    :close_end
    goto :primary_throw
    :secondary
    move-exception v3
    :suppress_start
    invoke-virtual {v2, v3}, Ljava/lang/Throwable;->addSuppressed(Ljava/lang/Throwable;)V
    :suppress_end
    :primary_throw
    throw v2
    :primary_throw_end
    :io
    move-exception v4
    invoke-static {v4}, Lfixture/CleanupContexts;->log(Ljava/lang/Throwable;)V
    const/4 v1, -7
    return v1
    .catch Ljava/io/IOException; {:acquire_start .. :acquire_end} :io
    .catchall {:body_start .. :body_end} :primary
    .catch Ljava/io/IOException; {:normal_positive_start .. :normal_positive_end} :io
    .catch Ljava/io/IOException; {:normal_zero_start .. :normal_zero_end} :io
    .catchall {:close_start .. :close_end} :secondary
    .catch Ljava/io/IOException; {:suppress_start .. :suppress_end} :io
    .catch Ljava/io/IOException; {:primary_throw .. :primary_throw_end} :io
.end method

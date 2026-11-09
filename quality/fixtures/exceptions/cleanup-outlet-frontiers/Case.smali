.class public final Lfixture/CleanupFrontiers;
.super Ljava/lang/Object;

.method public static native acquire(II)Lfixture/FrontierHandle;
.end method
.method public static native log(Ljava/lang/Throwable;)V
.end method
.method public static native probe()V
.end method

.method public static dual(I)I
    .registers 8
    const/4 v0, 0
    const/4 v1, 0
    const/4 v2, -1
    :body_start
    const/4 v3, 1
    invoke-static {p0, v3}, Lfixture/CleanupFrontiers;->acquire(II)Lfixture/FrontierHandle;
    move-result-object v0
    const/4 v3, 2
    invoke-static {p0, v3}, Lfixture/CleanupFrontiers;->acquire(II)Lfixture/FrontierHandle;
    move-result-object v1
    invoke-virtual {v0}, Lfixture/FrontierHandle;->ready()Z
    move-result v3
    if-eqz v3, :second
    invoke-virtual {v0}, Lfixture/FrontierHandle;->read()I
    move-result v2
    :second
    invoke-virtual {v1}, Lfixture/FrontierHandle;->ready()Z
    move-result v3
    if-eqz v3, :normal_close
    :inner_start
    invoke-virtual {v1}, Lfixture/FrontierHandle;->read()I
    move-result v3
    :inner_end
    add-int/2addr v2, v3
    goto :normal_close
    :runtime
    move-exception v4
    invoke-static {v4}, Lfixture/CleanupFrontiers;->log(Ljava/lang/Throwable;)V
    add-int/lit8 v2, v2, 1
    :normal_close
    if-eqz v1, :normal_first
    invoke-virtual {v1}, Lfixture/FrontierHandle;->close()V
    :normal_first
    if-eqz v0, :normal_return
    invoke-virtual {v0}, Lfixture/FrontierHandle;->close()V
    :normal_return
    return v2
    :interrupted
    move-exception v4
    :interrupt_start
    throw v4
    :interrupt_end
    :io
    move-exception v5
    :log_start
    invoke-static {v5}, Lfixture/CleanupFrontiers;->log(Ljava/lang/Throwable;)V
    :log_end
    if-eqz v1, :io_first
    invoke-virtual {v1}, Lfixture/FrontierHandle;->close()V
    :io_first
    if-eqz v0, :io_return
    invoke-virtual {v0}, Lfixture/FrontierHandle;->close()V
    :io_return
    const/4 v2, -2
    return v2
    :all
    move-exception v6
    if-eqz v1, :all_first
    invoke-virtual {v1}, Lfixture/FrontierHandle;->close()V
    :all_first
    if-eqz v0, :all_throw
    invoke-virtual {v0}, Lfixture/FrontierHandle;->close()V
    :all_throw
    throw v6

    .catch Ljava/io/InterruptedIOException; {:body_start .. :inner_start} :interrupted
    .catch Ljava/io/IOException; {:body_start .. :inner_start} :io
    .catchall {:body_start .. :inner_start} :all
    .catch Ljava/lang/RuntimeException; {:inner_start .. :inner_end} :runtime
    .catch Ljava/io/InterruptedIOException; {:inner_start .. :inner_end} :interrupted
    .catch Ljava/io/IOException; {:inner_start .. :inner_end} :io
    .catchall {:inner_start .. :inner_end} :all
    .catch Ljava/io/InterruptedIOException; {:inner_end .. :normal_close} :interrupted
    .catch Ljava/io/IOException; {:inner_end .. :normal_close} :io
    .catchall {:inner_end .. :normal_close} :all
    .catchall {:interrupt_start .. :interrupt_end} :all
    .catchall {:log_start .. :log_end} :all
.end method

.method public static gap(I)I
    .registers 6
    const/4 v0, 0
    const/4 v1, 1
    :acquire_start
    invoke-static {p0, v1}, Lfixture/CleanupFrontiers;->acquire(II)Lfixture/FrontierHandle;
    move-result-object v0
    :acquire_end
    invoke-static {}, Lfixture/CleanupFrontiers;->probe()V
    :read_start
    invoke-virtual {v0}, Lfixture/FrontierHandle;->read()I
    move-result v1
    :read_end
    if-eqz v0, :normal_return
    invoke-virtual {v0}, Lfixture/FrontierHandle;->close()V
    :normal_return
    return v1
    :io
    move-exception v2
    :log_start
    invoke-static {v2}, Lfixture/CleanupFrontiers;->log(Ljava/lang/Throwable;)V
    :log_end
    if-eqz v0, :io_return
    invoke-virtual {v0}, Lfixture/FrontierHandle;->close()V
    :io_return
    const/4 v1, -2
    return v1
    :all
    move-exception v3
    if-eqz v0, :all_throw
    invoke-virtual {v0}, Lfixture/FrontierHandle;->close()V
    :all_throw
    throw v3
    .catch Ljava/io/IOException; {:acquire_start .. :acquire_end} :io
    .catchall {:acquire_start .. :acquire_end} :all
    .catch Ljava/io/IOException; {:read_start .. :read_end} :io
    .catchall {:read_start .. :read_end} :all
    .catchall {:log_start .. :log_end} :all
.end method

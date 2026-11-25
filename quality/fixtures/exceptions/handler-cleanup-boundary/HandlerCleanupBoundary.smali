.class public final Lfixture/HandlerCleanupBoundary;
.super Ljava/lang/Object;

.method public static native acquire(I)I
.end method
.method public static native probe()V
.end method
.method public static native read(I)I
.end method
.method public static native log(Ljava/lang/Throwable;)V
.end method
.method public static native audit(I)V
.end method
.method public static native close(I)V
.end method

.method public static run(I)I
    .locals 4
    invoke-static {p0}, Lfixture/HandlerCleanupBoundary;->acquire(I)I
    move-result v0
    invoke-static {}, Lfixture/HandlerCleanupBoundary;->probe()V
    :read
    invoke-static {v0}, Lfixture/HandlerCleanupBoundary;->read(I)I
    move-result v1
    :read_end
    if-eqz v0, :success
    invoke-static {v0}, Lfixture/HandlerCleanupBoundary;->close(I)V
    :success
    return v1
    :io
    move-exception v2
    :log
    invoke-static {v2}, Lfixture/HandlerCleanupBoundary;->log(Ljava/lang/Throwable;)V
    :logged
    invoke-static {v0}, Lfixture/HandlerCleanupBoundary;->audit(I)V
    if-eqz v0, :recover
    invoke-static {v0}, Lfixture/HandlerCleanupBoundary;->close(I)V
    :recover
    const/4 v1, -2
    return v1
    :all
    move-exception v3
    if-eqz v0, :rethrow
    invoke-static {v0}, Lfixture/HandlerCleanupBoundary;->close(I)V
    :rethrow
    throw v3
    .catch Ljava/io/IOException; {:read .. :read_end} :io
    .catchall {:read .. :read_end} :all
    .catchall {:log .. :logged} :all
.end method

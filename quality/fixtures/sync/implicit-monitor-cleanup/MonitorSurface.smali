.class public final Lfixture/MonitorSurface;
.super Ljava/lang/Object;
.field public runningAsyncCalls:Ljava/util/ArrayDeque;
.field public runningSyncCalls:Ljava/util/ArrayDeque;
.method public counts()I
    .locals 7
    monitor-enter p0
    :body
    iget-object v0, p0, Lfixture/MonitorSurface;->runningAsyncCalls:Ljava/util/ArrayDeque;
    invoke-virtual {v0}, Ljava/util/ArrayDeque;->size()I
    move-result v2
    iget-object v1, p0, Lfixture/MonitorSurface;->runningSyncCalls:Ljava/util/ArrayDeque;
    invoke-virtual {v1}, Ljava/util/ArrayDeque;->size()I
    move-result v4
    :body_end
    add-int v5, v2, v4
    monitor-exit p0
    return v5
    :handler
    move-exception v6
    monitor-exit p0
    throw v6
    .catchall {:body .. :body_end} :handler
.end method
.method public static native read(I)I
.end method
.method public static native note(Ljava/lang/Throwable;)V
.end method
.method public static caught(I)I
    .locals 2
    :body
    invoke-static {p0}, Lfixture/MonitorSurface;->read(I)I
    move-result v0
    :body_end
    return v0
    :handler
    move-exception v1
    invoke-static {v1}, Lfixture/MonitorSurface;->note(Ljava/lang/Throwable;)V
    const/4 v0, -0x1
    return v0
    .catchall {:body .. :body_end} :handler
.end method

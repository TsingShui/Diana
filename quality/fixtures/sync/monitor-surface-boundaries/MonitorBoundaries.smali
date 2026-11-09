.class public final Lfixture/MonitorBoundaries;
.super Ljava/lang/Object;
.method public static native read(II)I
.end method
.method public static native note(Ljava/lang/Throwable;)V
.end method
.method public static nested(Ljava/lang/Object;Ljava/lang/Object;I)I
    .locals 8
    const/4 v7, 0x0
    monitor-enter p0
    :outer_start
    invoke-static {v7, p2}, Lfixture/MonitorBoundaries;->read(II)I
    move-result v0
    monitor-enter p1
    :inner_start
    const/4 v7, 0x1
    invoke-static {v7, p2}, Lfixture/MonitorBoundaries;->read(II)I
    move-result v1
    :inner_end
    monitor-exit p1
    const/4 v7, 0x2
    invoke-static {v7, p2}, Lfixture/MonitorBoundaries;->read(II)I
    move-result v2
    add-int v3, v0, v1
    add-int/2addr v3, v2
    :outer_end
    monitor-exit p0
    return v3
    :inner_handler
    move-exception v5
    monitor-exit p1
    throw v5
    :inner_handler_end
    :outer_handler
    move-exception v6
    monitor-exit p0
    throw v6
    .catchall {:outer_start .. :inner_start} :outer_handler
    .catchall {:inner_start .. :inner_end} :inner_handler
    .catchall {:inner_end .. :outer_end} :outer_handler
    .catchall {:inner_handler .. :inner_handler_end} :outer_handler
.end method
.method public static innerCatch(Ljava/lang/Object;I)I
    .locals 4
    const/4 v3, 0x0
    monitor-enter p0
    :body
    invoke-static {v3, p1}, Lfixture/MonitorBoundaries;->read(II)I
    move-result v0
    :body_end
    goto :normal
    :typed_handler
    move-exception v1
    const/16 v0, 0xa
    :normal
    monitor-exit p0
    return v0
    :cleanup_handler
    move-exception v2
    monitor-exit p0
    throw v2
    .catch Ljava/lang/IllegalStateException; {:body .. :body_end} :typed_handler
    .catchall {:body .. :body_end} :cleanup_handler
.end method
.method public static extra(Ljava/lang/Object;I)I
    .locals 3
    const/4 v2, 0x0
    monitor-enter p0
    :body
    invoke-static {v2, p1}, Lfixture/MonitorBoundaries;->read(II)I
    move-result v0
    :body_end
    monitor-exit p0
    return v0
    :handler
    move-exception v1
    invoke-static {v1}, Lfixture/MonitorBoundaries;->note(Ljava/lang/Throwable;)V
    monitor-exit p0
    throw v1
    .catchall {:body .. :body_end} :handler
.end method
.method public static otherException(Ljava/lang/Object;Ljava/lang/Throwable;I)I
    .locals 3
    const/4 v2, 0x0
    monitor-enter p0
    :body
    invoke-static {v2, p2}, Lfixture/MonitorBoundaries;->read(II)I
    move-result v0
    :body_end
    monitor-exit p0
    return v0
    :handler
    move-exception v1
    monitor-exit p0
    throw p1
    .catchall {:body .. :body_end} :handler
.end method
.method public static twoReturns(Ljava/lang/Object;I)I
    .locals 3
    const/4 v2, 0x0
    monitor-enter p0
    :body
    invoke-static {v2, p1}, Lfixture/MonitorBoundaries;->read(II)I
    move-result v0
    :body_end
    if-eqz p1, :first
    add-int/lit8 v0, v0, 0x64
    monitor-exit p0
    return v0
    :first
    monitor-exit p0
    return v0
    :handler
    move-exception v1
    monitor-exit p0
    throw v1
    .catchall {:body .. :body_end} :handler
.end method
.method public static wrongLock(Ljava/lang/Object;Ljava/lang/Object;I)I
    .locals 3
    const/4 v2, 0x0
    monitor-enter p0
    :body
    invoke-static {v2, p2}, Lfixture/MonitorBoundaries;->read(II)I
    move-result v0
    :body_end
    monitor-exit p0
    return v0
    :handler
    move-exception v1
    monitor-exit p1
    throw v1
    .catchall {:body .. :body_end} :handler
.end method
.method public static escaped(I)Ljava/lang/Throwable;
    .locals 3
    const/4 v2, 0x0
    :body
    invoke-static {v2, p0}, Lfixture/MonitorBoundaries;->read(II)I
    move-result v1
    :body_end
    const/4 v0, 0x0
    goto :join
    :handler
    move-exception v0
    :join
    return-object v0
    .catchall {:body .. :body_end} :handler
.end method

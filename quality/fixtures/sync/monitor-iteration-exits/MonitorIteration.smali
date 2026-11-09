.class public final Lfixture/MonitorIteration;
.super Ljava/lang/Object;

.field public static trace:I

.method public static mark(I)V
    .locals 1
    sget v0, Lfixture/MonitorIteration;->trace:I
    mul-int/lit8 v0, v0, 0x1f
    add-int/2addr v0, p0
    sput v0, Lfixture/MonitorIteration;->trace:I
    return-void
.end method

.method public static read(II)I
    .locals 3
    add-int/lit8 v0, p0, 0xa
    invoke-static {v0}, Lfixture/MonitorIteration;->mark(I)V
    const/4 v0, 0x1
    if-ne p1, v0, :not_nfe
    new-instance v1, Ljava/lang/NumberFormatException;
    const-string v2, "row"
    invoke-direct {v1, v2}, Ljava/lang/NumberFormatException;-><init>(Ljava/lang/String;)V
    throw v1
    :not_nfe
    const/4 v0, 0x2
    if-ne p1, v0, :not_npe
    new-instance v1, Ljava/lang/NullPointerException;
    const-string v2, "row"
    invoke-direct {v1, v2}, Ljava/lang/NullPointerException;-><init>(Ljava/lang/String;)V
    throw v1
    :not_npe
    const/4 v0, 0x3
    if-ne p1, v0, :not_ise
    new-instance v1, Ljava/lang/IllegalStateException;
    const-string v2, "row"
    invoke-direct {v1, v2}, Ljava/lang/IllegalStateException;-><init>(Ljava/lang/String;)V
    throw v1
    :not_ise
    const/4 v0, 0x4
    if-eq p1, v0, :error
    const/4 v0, 0x6
    if-ne p1, v0, :normal
    :error
    new-instance v1, Ljava/lang/AssertionError;
    const-string v2, "row"
    invoke-direct {v1, v2}, Ljava/lang/AssertionError;-><init>(Ljava/lang/Object;)V
    throw v1
    :normal
    add-int/lit8 v0, p0, 0x1
    return v0
.end method

.method public static cleanup(II)V
    .locals 3
    add-int/lit8 v0, p0, 0x1e
    invoke-static {v0}, Lfixture/MonitorIteration;->mark(I)V
    const/4 v0, 0x5
    if-eq p1, v0, :fail
    const/4 v0, 0x6
    if-ne p1, v0, :done
    :fail
    new-instance v1, Ljava/lang/IllegalArgumentException;
    const-string v2, "close"
    invoke-direct {v1, v2}, Ljava/lang/IllegalArgumentException;-><init>(Ljava/lang/String;)V
    throw v1
    :done
    return-void
.end method

.method public static heldCleanup(Ljava/lang/Object;II)I
    .locals 8
    const/4 v0, 0x0
    const/4 v1, 0x0
    :header
    if-ge v0, p1, :done
    monitor-enter p0
    :read_start
    invoke-static {v0, p2}, Lfixture/MonitorIteration;->read(II)I
    move-result v2
    :read_end
    goto :normal_cleanup
    :typed
    move-exception v3
    const/16 v4, 0x14
    invoke-static {v4}, Lfixture/MonitorIteration;->mark(I)V
    const/16 v2, 0x64
    :typed_end
    goto :normal_cleanup
    :broad
    move-exception v3
    const/16 v4, 0x15
    invoke-static {v4}, Lfixture/MonitorIteration;->mark(I)V
    const/4 v2, -0x2
    :broad_end
    goto :normal_cleanup
    :normal_cleanup
    invoke-static {v0, p2}, Lfixture/MonitorIteration;->cleanup(II)V
    const/4 v4, 0x2
    if-eq v2, v4, :unlock_continue
    add-int/2addr v1, v2
    :body_end
    :unlock_normal
    monitor-exit p0
    goto :step
    :unlock_continue
    monitor-exit p0
    goto :step
    :step
    add-int/lit8 v0, v0, 0x1
    goto :header
    :done
    return v1
    :finally_handler
    move-exception v5
    invoke-static {v0, p2}, Lfixture/MonitorIteration;->cleanup(II)V
    throw v5
    :finally_end
    :monitor_handler
    move-exception v6
    monitor-exit p0
    throw v6
    .catch Ljava/lang/NumberFormatException; {:read_start .. :read_end} :typed
    .catch Ljava/lang/NullPointerException; {:read_start .. :read_end} :typed
    .catch Ljava/lang/RuntimeException; {:read_start .. :read_end} :broad
    .catchall {:read_start .. :read_end} :finally_handler
    .catchall {:typed .. :typed_end} :finally_handler
    .catchall {:broad .. :broad_end} :finally_handler
    .catchall {:normal_cleanup .. :body_end} :monitor_handler
    .catchall {:finally_handler .. :finally_end} :monitor_handler
.end method

.method public static walk(Ljava/lang/Object;II)I
    .locals 8
    const/4 v0, 0x0
    const/4 v1, 0x0
    :header
    if-ge v0, p1, :done
    monitor-enter p0
    :read_start
    invoke-static {v0, p2}, Lfixture/MonitorIteration;->read(II)I
    move-result v2
    :read_end
    goto :normal_cleanup
    :typed
    move-exception v3
    const/16 v4, 0x14
    invoke-static {v4}, Lfixture/MonitorIteration;->mark(I)V
    const/16 v2, 0x64
    :typed_end
    goto :normal_cleanup
    :broad
    move-exception v3
    const/16 v4, 0x15
    invoke-static {v4}, Lfixture/MonitorIteration;->mark(I)V
    const/4 v2, -0x2
    :broad_end
    goto :normal_cleanup
    :normal_cleanup
    invoke-static {v0, p2}, Lfixture/MonitorIteration;->cleanup(II)V
    if-ltz v2, :unlock_return
    const/4 v4, 0x2
    if-eq v2, v4, :unlock_continue
    add-int/2addr v1, v2
    const/16 v4, 0xc8
    if-ge v1, v4, :unlock_break
    :body_end
    :unlock_normal
    monitor-exit p0
    goto :step
    :unlock_continue
    monitor-exit p0
    goto :step
    :unlock_break
    monitor-exit p0
    goto :done
    :unlock_return
    monitor-exit p0
    return v2
    :step
    add-int/lit8 v0, v0, 0x1
    goto :header
    :done
    return v1
    :finally_handler
    move-exception v5
    invoke-static {v0, p2}, Lfixture/MonitorIteration;->cleanup(II)V
    throw v5
    :finally_end
    :monitor_handler
    move-exception v6
    monitor-exit p0
    throw v6
    .catch Ljava/lang/NumberFormatException; {:read_start .. :read_end} :typed
    .catch Ljava/lang/NullPointerException; {:read_start .. :read_end} :typed
    .catch Ljava/lang/RuntimeException; {:read_start .. :read_end} :broad
    .catchall {:read_start .. :read_end} :finally_handler
    .catchall {:typed .. :typed_end} :finally_handler
    .catchall {:broad .. :broad_end} :finally_handler
    .catchall {:normal_cleanup .. :body_end} :monitor_handler
    .catchall {:finally_handler .. :finally_end} :monitor_handler
.end method

.method public static bypass(Ljava/lang/Object;II)I
    .locals 5
    const/4 v0, 0x0
    const/4 v1, 0x0
    :header
    if-ge v0, p1, :done
    monitor-enter p0
    :body
    add-int/lit8 v2, v0, 0x32
    invoke-static {v2}, Lfixture/MonitorIteration;->mark(I)V
    if-eqz p2, :advance_release
    add-int/lit8 v1, v1, 0x1
    const/4 v3, 0x2
    if-eq v1, v3, :return_release
    :body_end
    goto :header_release
    :advance_release
    monitor-exit p0
    goto :step
    :header_release
    monitor-exit p0
    goto :header
    :return_release
    monitor-exit p0
    return v1
    :handler
    move-exception v4
    monitor-exit p0
    throw v4
    :step
    add-int/lit8 v0, v0, 0x1
    goto :header
    :done
    return v1
    .catchall {:body .. :body_end} :handler
.end method

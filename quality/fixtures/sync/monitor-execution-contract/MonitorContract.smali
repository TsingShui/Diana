.class public final Lfixture/MonitorContract;
.super Ljava/lang/Object;

.method public constructor <init>()V
    .locals 0
    invoke-direct {p0}, Ljava/lang/Object;-><init>()V
    return-void
.end method

.method public declared-synchronized physical(Z)Z
    .locals 2
    monitor-enter p0
    :inside
    invoke-static {p0, p1}, Lfixture/MonitorHooks;->probe(Ljava/lang/Object;Z)Z
    move-result v0
    :inside_end
    monitor-exit p0
    return v0

    :handler
    move-exception v1
    monitor-exit p0
    throw v1
    .catchall {:inside .. :inside_end} :handler
.end method

.method public declared-synchronized metadataOnly()Z
    .locals 1
    invoke-static {p0}, Ljava/lang/Thread;->holdsLock(Ljava/lang/Object;)Z
    move-result v0
    return v0
.end method

.method public nested(Z)Z
    .locals 3
    monitor-enter p0
    :outer
    monitor-enter p0
    :inner
    invoke-static {p0, p1}, Lfixture/MonitorHooks;->probe(Ljava/lang/Object;Z)Z
    move-result v0
    :inner_end
    monitor-exit p0
    :outer_end
    monitor-exit p0
    return v0

    :inner_handler
    move-exception v1
    monitor-exit p0
    throw v1
    :inner_handler_end
    :outer_handler
    move-exception v2
    monitor-exit p0
    throw v2
    .catchall {:outer .. :inner} :outer_handler
    .catchall {:inner .. :inner_end} :inner_handler
    .catchall {:inner_end .. :outer_end} :outer_handler
    .catchall {:inner_handler .. :inner_handler_end} :outer_handler
.end method

.method public nestedNamed(Ljava/lang/Object;Z)V
    .locals 3
    monitor-enter p1
    :outer
    monitor-enter p1
    :inner
    invoke-static {p1, p2}, Lfixture/MonitorHooks;->probeVoid(Ljava/lang/Object;Z)V
    :inner_end
    monitor-exit p1
    :outer_end
    monitor-exit p1
    return-void

    :inner_handler
    move-exception v1
    monitor-exit p1
    throw v1
    :inner_handler_end
    :outer_handler
    move-exception v2
    monitor-exit p1
    throw v2
    .catchall {:outer .. :inner} :outer_handler
    .catchall {:inner .. :inner_end} :inner_handler
    .catchall {:inner_end .. :outer_end} :outer_handler
    .catchall {:inner_handler .. :inner_handler_end} :outer_handler
.end method

.method public synchronized native nativeSync()V
.end method

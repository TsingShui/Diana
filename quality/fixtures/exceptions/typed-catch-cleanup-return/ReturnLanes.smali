.class public final Lfixture/ReturnLanes;
.super Ljava/lang/Object;
.field public static trace:I

.method public static lookup(Ljava/lang/Object;Ljava/lang/RuntimeException;Ljava/lang/RuntimeException;Ljava/lang/RuntimeException;)Ljava/lang/Object;
    .registers 12
    const-class v0, Lfixture/ReturnLanes;
    monitor-enter v0
    const/4 v1, 0x0
    const/4 v2, 0x1
    const/4 v3, 0x3
    const/4 v4, 0x4
    const/4 v5, 0x5
    :open_start
    invoke-static {v2}, Lfixture/ReturnLanes;->stamp(I)V
    :open_end
    :inner_start
    if-eqz p0, :empty
    invoke-static {p0}, Lfixture/ReturnLanes;->capture(Ljava/lang/Object;)Ljava/lang/Object;
    move-result-object v6
    invoke-static {v3, p1}, Lfixture/ReturnLanes;->markThenThrow(ILjava/lang/RuntimeException;)V
    :inner_end
    :resource_normal_start
    invoke-static {v4, p2}, Lfixture/ReturnLanes;->markThenThrow(ILjava/lang/RuntimeException;)V
    :resource_normal_end
    :owner_normal_start
    invoke-static {v5, p3}, Lfixture/ReturnLanes;->markThenThrow(ILjava/lang/RuntimeException;)V
    :owner_normal_end
    monitor-exit v0
    return-object v6

    :empty
    :resource_empty_start
    invoke-static {v4, p2}, Lfixture/ReturnLanes;->markThenThrow(ILjava/lang/RuntimeException;)V
    :resource_empty_end
    :owner_empty_start
    invoke-static {v5, p3}, Lfixture/ReturnLanes;->markThenThrow(ILjava/lang/RuntimeException;)V
    :owner_empty_end
    monitor-exit v0
    return-object v1

    :recover_cursor
    move-exception v7
    :resource_throw_start
    invoke-static {v4, p2}, Lfixture/ReturnLanes;->markThenThrow(ILjava/lang/RuntimeException;)V
    :resource_throw_end
    goto :rethrow
    :suppress
    move-exception v6
    :suppress_start
    invoke-virtual {v7, v6}, Ljava/lang/Throwable;->addSuppressed(Ljava/lang/Throwable;)V
    :rethrow
    throw v7
    :rethrow_end

    :owner_unwind
    move-exception v7
    :unwind_owner_start
    invoke-static {v5, p3}, Lfixture/ReturnLanes;->markThenThrow(ILjava/lang/RuntimeException;)V
    throw v7
    :unwind_owner_end

    :caught
    :caught_owner_start
    invoke-static {v5, p3}, Lfixture/ReturnLanes;->markThenThrow(ILjava/lang/RuntimeException;)V
    :caught_owner_end
    monitor-exit v0
    return-object v1

    :unlock
    move-exception v7
    monitor-exit v0
    throw v7

    .catchall {:open_start .. :open_end} :unlock
    .catchall {:inner_start .. :inner_end} :recover_cursor
    .catch Ljava/lang/Exception; {:resource_normal_start .. :resource_normal_end} :caught
    .catchall {:resource_normal_start .. :resource_normal_end} :owner_unwind
    .catchall {:owner_normal_start .. :owner_normal_end} :unlock
    .catch Ljava/lang/Exception; {:resource_empty_start .. :resource_empty_end} :caught
    .catchall {:resource_empty_start .. :resource_empty_end} :owner_unwind
    .catchall {:owner_empty_start .. :owner_empty_end} :unlock
    .catchall {:resource_throw_start .. :resource_throw_end} :suppress
    .catch Ljava/lang/Exception; {:suppress_start .. :rethrow_end} :caught
    .catchall {:suppress_start .. :rethrow_end} :owner_unwind
    .catchall {:caught_owner_start .. :caught_owner_end} :unlock
    .catchall {:unwind_owner_start .. :unwind_owner_end} :unlock
.end method

.method private static capture(Ljava/lang/Object;)Ljava/lang/Object;
    .registers 2
    const/4 v0, 0x2
    invoke-static {v0}, Lfixture/ReturnLanes;->stamp(I)V
    return-object p0
.end method

.method private static markThenThrow(ILjava/lang/RuntimeException;)V
    .registers 2
    invoke-static {p0}, Lfixture/ReturnLanes;->stamp(I)V
    if-eqz p1, :return
    throw p1
:return
    return-void
.end method

.method private static stamp(I)V
    .registers 2
    sget v0, Lfixture/ReturnLanes;->trace:I
    mul-int/lit8 v0, v0, 0xa
    add-int/2addr v0, p0
    sput v0, Lfixture/ReturnLanes;->trace:I
    return-void
.end method

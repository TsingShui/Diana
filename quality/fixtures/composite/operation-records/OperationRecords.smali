.class public final Lfixture/OperationRecords;
.super Ljava/lang/Object;

.method static native first(I)I
.end method
.method static native later()I
.end method
.method static native gate(II)V
.end method

.method public static protectedNew(I)I
    .registers 5
    const/4 v0, 0x0
    :start
    invoke-static {p0}, Lfixture/OperationRecords;->first(I)I
    move-result v0
    new-instance v1, Lfixture/RecordBox;
    invoke-static {}, Lfixture/OperationRecords;->later()I
    move-result v2
    invoke-direct {v1, v0, v2}, Lfixture/RecordBox;-><init>(II)V
    add-int/lit8 v0, v0, 0x1
    :end
    return v0
    :handler
    move-exception v3
    return v0
    .catch Ljava/lang/RuntimeException; {:start .. :end} :handler
.end method

.method public static locked(Ljava/lang/Object;I)I
    .registers 5
    monitor-enter p0
    :start
    invoke-static {p1}, Lfixture/OperationRecords;->first(I)I
    move-result v0
    if-ltz v0, :negative
    invoke-static {}, Lfixture/OperationRecords;->later()I
    move-result v1
    goto :release
    :negative
    move v1, v0
    :end
    :release
    monitor-exit p0
    return v1
    :handler
    move-exception v2
    monitor-exit p0
    throw v2
    .catchall {:start .. :end} :handler
.end method

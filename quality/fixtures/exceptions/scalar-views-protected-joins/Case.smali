.class public final Lfixture/ProtectedViews;
.super Ljava/lang/Object;

.method public static native mark(I)V
.end method

.method public static unit(I)C
    .registers 2
    add-int/lit8 v0, p0, 10
    invoke-static {v0}, Lfixture/ProtectedViews;->mark(I)V
    add-int/lit8 v0, p0, 65
    int-to-char v0, v0
    return v0
.end method

.method public static record(C)I
    .registers 2
    add-int/lit16 v0, p0, 1000
    invoke-static {v0}, Lfixture/ProtectedViews;->mark(I)V
    return v0
.end method

.method public static record(I)I
    .registers 2
    add-int/lit16 v0, p0, 2000
    invoke-static {v0}, Lfixture/ProtectedViews;->mark(I)V
    return v0
.end method

.method public static route([CIZ)I
    .registers 10
    const/4 v0, 0
    const/4 v1, 0
    :head
    if-ge v0, p1, :done
    if-eqz p2, :from_call
    aget-char v2, p0, v0
    goto :body
    :from_call
    invoke-static {v0}, Lfixture/ProtectedViews;->unit(I)C
    move-result v2
    :body
    :body_start
    invoke-static {v2}, Lfixture/ProtectedViews;->record(C)I
    move-result v3
    add-int/2addr v1, v3
    invoke-static {v2}, Lfixture/ProtectedViews;->record(I)I
    move-result v3
    add-int/2addr v1, v3
    :body_end
    add-int/lit8 v4, v0, 70
    invoke-static {v4}, Lfixture/ProtectedViews;->mark(I)V
    goto :advance
    :recover
    move-exception v5
    :handler_start
    add-int/lit8 v4, v0, 40
    invoke-static {v4}, Lfixture/ProtectedViews;->mark(I)V
    :handler_end
    add-int/lit8 v4, v0, 70
    invoke-static {v4}, Lfixture/ProtectedViews;->mark(I)V
    goto :advance
    :finish_exception
    move-exception v5
    add-int/lit8 v4, v0, 70
    invoke-static {v4}, Lfixture/ProtectedViews;->mark(I)V
    throw v5
    :advance
    add-int/lit8 v0, v0, 1
    goto :head
    :done
    return v1
    .catch Ljava/lang/IllegalStateException; {:body_start .. :body_end} :recover
    .catchall {:body_start .. :body_end} :finish_exception
    .catchall {:handler_start .. :handler_end} :finish_exception
.end method

.method public static composed(I)I
    .registers 2
    invoke-static {p0}, Lfixture/ProtectedViews;->unit(I)C
    move-result v0
    invoke-static {v0}, Lfixture/ProtectedViews;->record(C)I
    move-result v0
    return v0
.end method

.method public static unproved(I)I
    .registers 4
    invoke-static {p0}, Lfixture/ProtectedViews;->record(C)I
    move-result v0
    invoke-static {p0}, Lfixture/ProtectedViews;->record(I)I
    move-result v1
    add-int/2addr v0, v1
    return v0
.end method

.method public static raw([CZ)I
    .registers 4
    if-eqz p1, :second
    const/4 v0, 0
    aget-char v1, p0, v0
    goto :join
    :second
    const/4 v0, 1
    aget-char v1, p0, v0
    :join
    return v1
.end method

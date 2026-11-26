.class public final Lfixture/SiblingExit;
.super Ljava/lang/Object;

.method public static run(II)I
    .locals 5
    const/4 v0, 0
    const/4 v1, 0
    :outer_start
    :outer
    if-ge v0, p0, :done
    invoke-static {v0, p1}, Lfixture/ScopeHooks;->select(II)I
    move-result v2
    packed-switch v2, :table
    goto :done
    :skip
    invoke-static {v0}, Lfixture/ScopeHooks;->skip(I)V
    goto :inc
    :normal
    invoke-static {v0}, Lfixture/ScopeHooks;->start(I)I
    move-result v3
    :inner
    if-lez v3, :read
    invoke-static {v3}, Lfixture/ScopeHooks;->step(I)V
    add-int/lit8 v3, v3, -1
    goto :inner
    :read
    :typed_start
    invoke-static {v0}, Lfixture/ScopeHooks;->read(I)I
    move-result v4
    :typed_end
    goto :sum
    :typed_catch
    move-exception v4
    invoke-static {v0}, Lfixture/ScopeHooks;->recover(I)I
    move-result v4
    :sum
    add-int/2addr v1, v4
    :inc
    add-int/lit8 v0, v0, 1
    goto :outer
    :outer_end
    .catch Ljava/lang/IllegalArgumentException; {:typed_start .. :typed_end} :typed_catch
    .catchall {:outer_start .. :outer_end} :outer_handler
    :done
    invoke-static {p1}, Lfixture/ScopeHooks;->close(I)V
    return v1
    :outer_handler
    move-exception v4
    invoke-static {p1}, Lfixture/ScopeHooks;->close(I)V
    throw v4
    :table
    .packed-switch 0x0
    :skip
    :normal
    :normal
    .end packed-switch
.end method

.method public static sweep(III)I
    .locals 6
    const/4 v0, 0
    const/4 v1, 0
    packed-switch p2, :prefaces
    goto :rows
    :skip
    invoke-static {p1}, Lfixture/ScopeHooks;->skip(I)V
    goto :rows
    :warm
    invoke-static {p1}, Lfixture/ScopeHooks;->start(I)I
    move-result v4
    :warm_header
    if-lez v4, :rows
    invoke-static {v4}, Lfixture/ScopeHooks;->step(I)V
    add-int/lit8 v4, v4, -1
    goto :warm_header
    :rows
    if-ge v0, p0, :normal
    :read_start
    invoke-static {v0}, Lfixture/ScopeHooks;->read(I)I
    move-result v2
    :read_end
    goto :join
    :recover
    move-exception v5
    invoke-static {v0}, Lfixture/ScopeHooks;->recover(I)I
    move-result v2
    :recover_end
    :join
    add-int/2addr v1, v2
    add-int/lit8 v0, v0, 1
    goto :rows
    :normal
    if-eqz p1, :normal_zero
    invoke-static {p0}, Lfixture/ScopeHooks;->step(I)V
    invoke-static {p1}, Lfixture/ScopeHooks;->close(I)V
    goto :done
    :normal_zero
    invoke-static {p0}, Lfixture/ScopeHooks;->skip(I)V
    :done
    return v1
    :handler
    move-exception v5
    if-eqz p1, :except_zero
    invoke-static {p0}, Lfixture/ScopeHooks;->step(I)V
    invoke-static {p1}, Lfixture/ScopeHooks;->close(I)V
    goto :rethrow
    :except_zero
    invoke-static {p0}, Lfixture/ScopeHooks;->skip(I)V
    :rethrow
    throw v5
    .catch Ljava/lang/NumberFormatException; {:read_start .. :read_end} :recover
    .catch Ljava/lang/NullPointerException; {:read_start .. :read_end} :recover
    .catchall {:read_start .. :read_end} :handler
    .catchall {:recover .. :recover_end} :handler
    :prefaces
    .packed-switch 0
    :warm
    :warm
    :skip
    .end packed-switch
.end method

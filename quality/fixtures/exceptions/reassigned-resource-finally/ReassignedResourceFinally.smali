.class public final Lfixture/ReassignedResourceFinally;
.super Ljava/lang/Object;

.method public static native read(I)I
.end method

.method public static native open(I)I
.end method

.method public static native close(I)V
.end method

# The acquisition argument is read before open. Inside the protected region the
# current handle is closed and the slot is reassigned to a new acquisition; the
# acquired flag records whether the finally owns a live handle. The finally is
# duplicated on the normal path and in the catch-all handler, and its conditional
# close is outside the protected range in both copies.
.method public static rotate(I)I
    .locals 8
    const/4 v3, 0x0
    const/4 v5, 0x0
    invoke-static {v5}, Lfixture/ReassignedResourceFinally;->read(I)I
    move-result v0
    invoke-static {v0}, Lfixture/ReassignedResourceFinally;->open(I)I
    move-result v1
    const/4 v2, 0x1
    :try_start
    invoke-static {v1}, Lfixture/ReassignedResourceFinally;->read(I)I
    move-result v4
    move v3, v4
    const/4 v5, 0x1
    if-ne p0, v5, :normal_return
    const/4 v2, 0x0
    invoke-static {v1}, Lfixture/ReassignedResourceFinally;->close(I)V
    add-int/lit8 v5, v0, 0x1
    invoke-static {v5}, Lfixture/ReassignedResourceFinally;->open(I)I
    move-result v6
    move v1, v6
    const/4 v2, 0x1
    invoke-static {v1}, Lfixture/ReassignedResourceFinally;->read(I)I
    move-result v4
    add-int/2addr v3, v4
    :normal_return
    :try_end
    if-eqz v2, :skip_normal_close
    invoke-static {v1}, Lfixture/ReassignedResourceFinally;->close(I)V
    :skip_normal_close
    return v3
    :handler
    move-exception v6
    :cleanup
    if-eqz v2, :cleanup_done
    invoke-static {v1}, Lfixture/ReassignedResourceFinally;->close(I)V
    :cleanup_done
    throw v6
    .catchall {:try_start .. :try_end} :handler
.end method

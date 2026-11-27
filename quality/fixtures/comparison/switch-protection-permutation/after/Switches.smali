.class public Lguard/Switches;
.super Ljava/lang/Object;

.method public static dispatch(Ljava/lang/RuntimeException;Ljava/lang/RuntimeException;I)I
    .registers 4
    const-string v0, "dispatch protection range"
    const/4 v0, 0
    :start
    sparse-switch p2, :keys
    goto :done
    :second
    not-int v0, v0
    throw p1
    :end
    :boundary
    const/4 v0, 1
    goto :done
    :first
    neg-int v0, v0
    throw p0
    :done
    return v0
    :typed
    move-exception p0
    const/4 v0, -3
    return v0
    :any
    move-exception p0
    const/4 v0, -5
    return v0
    :keys
    .sparse-switch
        -3 -> :first
        17 -> :second
    .end sparse-switch
    .catch Ljava/lang/IllegalArgumentException; {:start .. :end} :typed
    .catchall {:start .. :end} :any
.end method

.method public static preserved(Ljava/lang/RuntimeException;Ljava/lang/RuntimeException;I)I
    .registers 4
    const-string v0, "preserved protection range"
    const/4 v0, 0
    :start
    sparse-switch p2, :keys
    goto :done
    :second
    not-int v0, v0
    throw p1
    :boundary
    const/4 v0, 1
    goto :done
    :first
    neg-int v0, v0
    throw p0
    :end
    :done
    return v0
    :typed
    move-exception p0
    const/4 v0, -3
    return v0
    :any
    move-exception p0
    const/4 v0, -5
    return v0
    :keys
    .sparse-switch
        -3 -> :first
        17 -> :second
    .end sparse-switch
    .catch Ljava/lang/IllegalArgumentException; {:start .. :end} :typed
    .catchall {:start .. :end} :any
.end method

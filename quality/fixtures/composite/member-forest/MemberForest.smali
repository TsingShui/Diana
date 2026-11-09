.class public final Lfixture/MemberForest;
.super Ljava/lang/Object;

.method static native tick(I)I
.end method

.method public static interleaved(I)I
    .registers 2
    if-eqz p0, :right
    const/4 v0, 0x1
    goto :left_tail
    :right
    const/4 v0, 0x2
    goto :right_tail
    :left_tail
    add-int/lit8 v0, v0, 0x3
    goto :done
    :right_tail
    add-int/lit8 v0, v0, 0x5
    :done
    return v0
.end method

.method public static nested(II)I
    .registers 5
    move v0, p1
    const/4 v1, 0x4
    if-ltz p0, :early
    :repeat
    if-ge v0, v1, :done
    :try_start
    packed-switch p0, :cases
    add-int/lit8 v0, v0, 0x1
    goto :joined
    :small
    invoke-static {v0}, Lfixture/MemberForest;->tick(I)I
    move-result v0
    goto :joined
    :two
    add-int/lit8 v0, v0, 0x2
    :try_end
    :joined
    goto :repeat
    :caught
    move-exception v2
    add-int/lit8 v0, v0, 0x1
    goto :repeat
    :done
    return v0
    :early
    const/4 v0, -0x1
    return v0
    .catch Ljava/lang/RuntimeException; {:try_start .. :try_end} :caught
    :cases
    .packed-switch 0x0
        :small
        :small
        :two
    .end packed-switch
.end method

.method public static locked(Ljava/lang/Object;I)I
    .registers 5
    move v0, p1
    const/4 v1, 0x3
    monitor-enter p0
    :try_start
    :repeat
    if-ge v0, v1, :release
    invoke-static {v0}, Lfixture/MemberForest;->tick(I)I
    move-result v0
    goto :repeat
    :release
    monitor-exit p0
    :try_end
    return v0
    :caught
    move-exception v2
    monitor-exit p0
    throw v2
    .catchall {:try_start .. :try_end} :caught
.end method

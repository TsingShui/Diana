.class public final Lfixture/SequentialCleanupOrigins;
.super Ljava/lang/Object;

.method public static native read(I)I
.end method

.method public static native close(I)V
.end method

.method public static readBoth(II)I
    .locals 4
    move v0, p0
    :first
    invoke-static {v0}, Lfixture/SequentialCleanupOrigins;->read(I)I
    move-result v1
    :first_end
    invoke-static {v0}, Lfixture/SequentialCleanupOrigins;->close(I)V
    move v0, p1
    :second
    invoke-static {v0}, Lfixture/SequentialCleanupOrigins;->read(I)I
    move-result v2
    :second_end
    invoke-static {v0}, Lfixture/SequentialCleanupOrigins;->close(I)V
    add-int v3, v1, v2
    return v3

    :first_failure
    move-exception v2
    invoke-static {p0}, Lfixture/SequentialCleanupOrigins;->close(I)V
    throw v2
    :second_failure
    move-exception v2
    invoke-static {p0}, Lfixture/SequentialCleanupOrigins;->close(I)V
    throw v2
    .catchall {:first .. :first_end} :first_failure
    .catchall {:second .. :second_end} :second_failure
.end method

.method public static closeBoth(II)I
    .locals 4
    move v0, p0
    :first
    invoke-static {v0}, Lfixture/SequentialCleanupOrigins;->read(I)I
    move-result v1
    :first_end
    invoke-static {v0}, Lfixture/SequentialCleanupOrigins;->close(I)V
    move v0, p1
    :second
    invoke-static {v0}, Lfixture/SequentialCleanupOrigins;->read(I)I
    move-result v2
    :second_end
    invoke-static {v0}, Lfixture/SequentialCleanupOrigins;->close(I)V
    add-int v3, v1, v2
    return v3

    :first_failure
    move-exception v2
    invoke-static {p0}, Lfixture/SequentialCleanupOrigins;->close(I)V
    throw v2
    :second_failure
    move-exception v2
    invoke-static {p1}, Lfixture/SequentialCleanupOrigins;->close(I)V
    throw v2
    .catchall {:first .. :first_end} :first_failure
    .catchall {:second .. :second_end} :second_failure
.end method

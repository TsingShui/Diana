.class public final Lfixture/OrderedCleanupArguments;
.super Ljava/lang/Object;

.method public static native read(I)I
.end method

.method public static native mark(I)V
.end method

.method public static native close(II)V
.end method

.method public static run(II)I
    .locals 4
    move v0, p0
    :first
    invoke-static {v0}, Lfixture/OrderedCleanupArguments;->read(I)I
    move-result v1
    :first_end
    invoke-static {v0, p1}, Lfixture/OrderedCleanupArguments;->close(II)V

    move v0, p1
    :second
    invoke-static {v0}, Lfixture/OrderedCleanupArguments;->read(I)I
    move-result v2
    :second_end
    invoke-static {v1}, Lfixture/OrderedCleanupArguments;->mark(I)V
    invoke-static {v0, p0}, Lfixture/OrderedCleanupArguments;->close(II)V
    add-int v3, v1, v2
    return v3

    :first_failure
    move-exception v2
    invoke-static {p0, p1}, Lfixture/OrderedCleanupArguments;->close(II)V
    throw v2
    :second_failure
    move-exception v2
    invoke-static {v1}, Lfixture/OrderedCleanupArguments;->mark(I)V
    invoke-static {p0, p1}, Lfixture/OrderedCleanupArguments;->close(II)V
    throw v2
    .catchall {:first .. :first_end} :first_failure
    .catchall {:second .. :second_end} :second_failure
.end method

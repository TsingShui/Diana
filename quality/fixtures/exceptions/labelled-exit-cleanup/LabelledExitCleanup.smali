.class public final Lfixture/LabelledExitCleanup;
.super Ljava/lang/Object;

.method public static native open(I)I
.end method

.method public static native read(I)I
.end method

.method public static native close(I)V
.end method

# The resource lives in the outer loop body. A labelled continue, labelled break
# and return each close the handle before leaving. The exceptional path closes,
# suppresses a close failure on the pending primary and rethrows. Only the native
# body calls are protected; every normal close is outside the catchall range.
.method public static scan(II)I
    .locals 8
    const/4 v0, 0x0
    const/4 v1, 0x0
    :outer_test
    if-ge v1, p0, :done
    invoke-static {v1}, Lfixture/LabelledExitCleanup;->open(I)I
    move-result v3
    const/4 v2, 0x0
    :body
    :inner_test
    const/4 v5, 0x4
    if-ge v2, v5, :inner_done
    invoke-static {v3}, Lfixture/LabelledExitCleanup;->read(I)I
    move-result v4
    if-ne v4, p1, :not_continue
    :end_continue_test
    invoke-static {v3}, Lfixture/LabelledExitCleanup;->close(I)V
    add-int/lit8 v1, v1, 0x1
    goto :outer_test
    :not_continue
    if-gez v4, :not_break
    :end_break_test
    invoke-static {v3}, Lfixture/LabelledExitCleanup;->close(I)V
    goto :done
    :not_break
    add-int/2addr v0, v4
    const/16 v5, 0x64
    if-le v0, v5, :not_return
    :end_return_test
    invoke-static {v3}, Lfixture/LabelledExitCleanup;->close(I)V
    return v0
    :not_return
    add-int/lit8 v2, v2, 0x1
    goto :inner_test
    :inner_done
    invoke-static {v3}, Lfixture/LabelledExitCleanup;->close(I)V
    add-int/lit8 v1, v1, 0x1
    goto :outer_test
    :done
    return v0
    :handler
    move-exception v6
    :cleanup
    invoke-static {v3}, Lfixture/LabelledExitCleanup;->close(I)V
    :cleanup_end
    goto :rethrow
    :suppressed
    move-exception v7
    invoke-virtual {v6, v7}, Ljava/lang/Throwable;->addSuppressed(Ljava/lang/Throwable;)V
    :rethrow
    throw v6
    .catchall {:body .. :end_continue_test} :handler
    .catchall {:not_continue .. :end_break_test} :handler
    .catchall {:not_break .. :end_return_test} :handler
    .catchall {:not_return .. :inner_done} :handler
    .catchall {:cleanup .. :cleanup_end} :suppressed
.end method

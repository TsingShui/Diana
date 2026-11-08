.class public final Lfixture/ThreeResourceLateFailure;
.super Ljava/lang/Object;

.method public static native open(I)I
.end method

.method public static native read(I)I
.end method

.method public static native close(I)V
.end method

# Nested cleanup for three resources. The outer/middle handler ranges exclude the
# inner protected body and catch the inner rethrow instead, matching the nested
# try-with-resources protocol. Normal close of a later resource stays inside the
# enclosing protection.
.method public static openThree(I)I
    .locals 8
    const/4 v6, 0x0
    const/4 v7, 0x0
    invoke-static {v7}, Lfixture/ThreeResourceLateFailure;->open(I)I
    move-result v0
    :outer
    const/4 v7, 0x1
    invoke-static {v7}, Lfixture/ThreeResourceLateFailure;->open(I)I
    move-result v1
    :middle
    const/4 v7, 0x2
    invoke-static {v7}, Lfixture/ThreeResourceLateFailure;->open(I)I
    move-result v2
    :body
    invoke-static {v0}, Lfixture/ThreeResourceLateFailure;->read(I)I
    move-result v3
    invoke-static {v1}, Lfixture/ThreeResourceLateFailure;->read(I)I
    move-result v4
    invoke-static {v2}, Lfixture/ThreeResourceLateFailure;->read(I)I
    move-result v5
    add-int/2addr v3, v4
    add-int/2addr v3, v5
    move v6, v3
    :body_end
    invoke-static {v2}, Lfixture/ThreeResourceLateFailure;->close(I)V
    :middle_end
    invoke-static {v1}, Lfixture/ThreeResourceLateFailure;->close(I)V
    :outer_end
    invoke-static {v0}, Lfixture/ThreeResourceLateFailure;->close(I)V
    return v6
    :inner_handler
    move-exception v3
    :inner_cleanup
    invoke-static {v2}, Lfixture/ThreeResourceLateFailure;->close(I)V
    :inner_cleanup_end
    goto :inner_rethrow
    :inner_suppressed
    move-exception v4
    invoke-virtual {v3, v4}, Ljava/lang/Throwable;->addSuppressed(Ljava/lang/Throwable;)V
    :inner_rethrow
    throw v3
    :middle_handler
    move-exception v3
    :middle_cleanup
    invoke-static {v1}, Lfixture/ThreeResourceLateFailure;->close(I)V
    :middle_cleanup_end
    goto :middle_rethrow
    :middle_suppressed
    move-exception v4
    invoke-virtual {v3, v4}, Ljava/lang/Throwable;->addSuppressed(Ljava/lang/Throwable;)V
    :middle_rethrow
    throw v3
    :outer_handler
    move-exception v3
    :outer_cleanup
    invoke-static {v0}, Lfixture/ThreeResourceLateFailure;->close(I)V
    :outer_cleanup_end
    goto :outer_rethrow
    :outer_suppressed
    move-exception v4
    invoke-virtual {v3, v4}, Ljava/lang/Throwable;->addSuppressed(Ljava/lang/Throwable;)V
    :outer_rethrow
    throw v3
    .catchall {:body .. :body_end} :inner_handler
    .catchall {:inner_cleanup .. :inner_cleanup_end} :inner_suppressed
    .catchall {:inner_cleanup_end .. :middle_handler} :middle_handler
    .catchall {:middle .. :body} :middle_handler
    .catchall {:body_end .. :middle_end} :middle_handler
    .catchall {:middle_cleanup .. :middle_cleanup_end} :middle_suppressed
    .catchall {:middle_cleanup_end .. :outer_handler} :outer_handler
    .catchall {:outer .. :middle} :outer_handler
    .catchall {:middle_end .. :outer_end} :outer_handler
    .catchall {:outer_cleanup .. :outer_cleanup_end} :outer_suppressed
.end method

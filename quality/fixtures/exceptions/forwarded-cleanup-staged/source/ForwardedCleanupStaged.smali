.class public final Lfixture/ForwardedCleanupStaged;
.super Ljava/lang/Object;

.method private static native start(I)V
.end method

.method public native cleanup(IJLjava/lang/String;Ljava/lang/Throwable;)V
.end method

.method private static native outward(Ljava/lang/Throwable;)V
.end method

.method public static run(Lfixture/ForwardedCleanupStaged;Lfixture/ForwardedCleanupStaged;Lfixture/ForwardedCleanupStaged;I)V
    .locals 7

    if-eqz p3, :left_start
    const/4 v6, 0x1
    if-eq p3, v6, :right_start

    const/16 v1, 0x21
    :direct_call
    invoke-static {v1}, Lfixture/ForwardedCleanupStaged;->start(I)V
    :direct_end
    .catchall {:direct_call .. :direct_end} :direct_capture
    return-void

    :left_start
    const/16 v1, 0xb
    :left_call
    invoke-static {v1}, Lfixture/ForwardedCleanupStaged;->start(I)V
    :left_end
    .catchall {:left_call .. :left_end} :left_capture
    return-void

    :right_start
    const/16 v1, 0x16
    :right_call
    invoke-static {v1}, Lfixture/ForwardedCleanupStaged;->start(I)V
    :right_end
    .catchall {:right_call .. :right_end} :right_capture
    return-void

    :left_capture
    move-exception v6
    move-object v0, p0
    const/16 v1, 0xb
    const-wide v2, 0x100000001L
    goto :transfer_pending

    :right_capture
    move-exception v5
    move-object v0, p1
    const/16 v1, 0x16
    const-wide v2, 0x200000002L
    goto :set_forwarded_tag

    :transfer_pending
    move-object v5, v6

    :set_forwarded_tag
    const-string v4, "forwarded"
    goto :shared_cleanup

    :direct_capture
    move-exception v5
    move-object v0, p2
    const/16 v1, 0x21
    const-wide v2, 0x300000003L
    const-string v4, "direct"

    :shared_cleanup
    invoke-virtual/range {v0 .. v5}, Lfixture/ForwardedCleanupStaged;->cleanup(IJLjava/lang/String;Ljava/lang/Throwable;)V
    throw v5
    :cleanup_end
    .catchall {:shared_cleanup .. :cleanup_end} :outward_capture

    :outward_capture
    move-exception v5
    invoke-static {v5}, Lfixture/ForwardedCleanupStaged;->outward(Ljava/lang/Throwable;)V
    return-void
.end method

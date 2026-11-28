.class public final Lfixture/ForwardedCleanup;
.super Ljava/lang/Object;

.method private static native start(I)V
.end method

.method public native cleanup(IJLjava/lang/String;Ljava/lang/Throwable;)V
.end method

.method private static native outward(Ljava/lang/Throwable;)V
.end method

.method public static run(Lfixture/ForwardedCleanup;Lfixture/ForwardedCleanup;Z)V
    .locals 6

    if-nez p2, :right_start

    const/16 v1, 0xb
    :left_start
    invoke-static {v1}, Lfixture/ForwardedCleanup;->start(I)V
    :left_end
    .catchall {:left_start .. :left_end} :left_capture
    return-void

    :right_start
    const/16 v1, 0x16
    :right_call
    invoke-static {v1}, Lfixture/ForwardedCleanup;->start(I)V
    :right_end
    .catchall {:right_call .. :right_end} :right_capture
    return-void

    :left_capture
    move-exception v5
    move-object v0, p0
    const/16 v1, 0xb
    const-wide v2, 0x100000001L
    const-string v4, "amber"
    goto :shared_cleanup

    :right_capture
    move-exception v5
    move-object v0, p1
    const/16 v1, 0x16
    const-wide v2, 0x200000002L
    const-string v4, "blue"

    :shared_cleanup
    invoke-virtual/range {v0 .. v5}, Lfixture/ForwardedCleanup;->cleanup(IJLjava/lang/String;Ljava/lang/Throwable;)V
    throw v5
    :shared_cleanup_end
    .catchall {:shared_cleanup .. :shared_cleanup_end} :outward_capture

    :outward_capture
    move-exception v5
    invoke-static {v5}, Lfixture/ForwardedCleanup;->outward(Ljava/lang/Throwable;)V
    return-void
.end method

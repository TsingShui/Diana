.class public final Lfixture/FinallyOutwardState;
.super Ljava/lang/Object;

.method private static native stage(I)V
.end method

.method private static native outward(ILjava/lang/Throwable;)V
.end method

.method public native cleanup(I)V
.end method

.method public static run(Lfixture/FinallyOutwardState;II)V
    .locals 7

    const/4 v5, 0x3
    const/4 v0, 0x1
    :call_one
    invoke-static {v0}, Lfixture/FinallyOutwardState;->stage(I)V
    :one_end

    if-nez p2, :other_timestamp
    const/4 v5, 0x1
    goto :after_timestamp
    :other_timestamp
    const/4 v5, 0x2
    :after_timestamp

    const/4 v0, 0x2
    :call_two
    invoke-static {v0}, Lfixture/FinallyOutwardState;->stage(I)V
    :two_end
    const/4 v0, 0x3
    :call_three
    invoke-static {v0}, Lfixture/FinallyOutwardState;->stage(I)V
    :three_end

    :normal_cleanup
    invoke-virtual {p0, p1}, Lfixture/FinallyOutwardState;->cleanup(I)V
    :normal_cleanup_end
    return-void

    :capture_one
    move-exception v1
    move-object v4, v1
    goto :exception_cleanup

    :capture_two
    move-exception v2
    move-object v4, v2
    goto :exception_cleanup

    :capture_three
    move-exception v3
    move-object v4, v3

    :exception_cleanup
    invoke-virtual {p0, p1}, Lfixture/FinallyOutwardState;->cleanup(I)V
    throw v4
    :exception_cleanup_end

    :outer_capture
    move-exception v6
    invoke-static {v5, v6}, Lfixture/FinallyOutwardState;->outward(ILjava/lang/Throwable;)V
    return-void

    .catchall {:call_one .. :one_end} :capture_one
    .catchall {:call_two .. :two_end} :capture_two
    .catchall {:call_three .. :three_end} :capture_three
    .catchall {:normal_cleanup .. :normal_cleanup_end} :outer_capture
    .catchall {:exception_cleanup .. :exception_cleanup_end} :outer_capture
.end method

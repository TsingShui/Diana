.class public final Lfixture/BackoffTwoWide;
.super Ljava/lang/Object;

.method private static native initialClock()J
.end method

.method private static native schedule()V
.end method

.method private static native save(J)V
.end method

.method private static native waitFor()V
.end method

.method private static native interruption(Ljava/lang/InterruptedException;)V
.end method

.method private static native refreshClock()J
.end method

.method private static native reacquire()J
.end method

.method private static native cancel()V
.end method

.method private static native complete(JJ)V
.end method

.method private static native diagnostic(JJLjava/lang/Throwable;)V
.end method

.method public static run()V
    .locals 12

    invoke-static {}, Lfixture/BackoffTwoWide;->initialClock()J
    move-result-wide v0
    const-wide v10, 0x300000068L

    :early_start
    invoke-static {}, Lfixture/BackoffTwoWide;->schedule()V
    :early_end

    :save_start
    move-wide v2, v0
    invoke-static {v2, v3}, Lfixture/BackoffTwoWide;->save(J)V
    :save_end
    const-wide v10, 0x4000000b4L

    :wait_start
    invoke-static {}, Lfixture/BackoffTwoWide;->waitFor()V
    :wait_end
    goto :refresh_normal

    :typed_capture
    move-exception v7
    :interruption_start
    invoke-static {v7}, Lfixture/BackoffTwoWide;->interruption(Ljava/lang/InterruptedException;)V
    :interruption_end

    :refresh_normal
    invoke-static {}, Lfixture/BackoffTwoWide;->refreshClock()J
    move-result-wide v0
    invoke-static {}, Lfixture/BackoffTwoWide;->reacquire()J
    move-result-wide v10
    :refresh_normal_end
    goto :normal_cancel

    :work_failure
    move-exception v8
    :refresh_fault
    invoke-static {}, Lfixture/BackoffTwoWide;->refreshClock()J
    move-result-wide v0
    invoke-static {}, Lfixture/BackoffTwoWide;->reacquire()J
    move-result-wide v10
    throw v8
    :refresh_fault_end

    :normal_cancel
    invoke-static {}, Lfixture/BackoffTwoWide;->cancel()V
    invoke-static {v0, v1, v10, v11}, Lfixture/BackoffTwoWide;->complete(JJ)V
    :normal_cancel_end
    return-void

    :capture_early
    move-exception v5
    move-object v4, v5
    goto :fault_cancel

    :capture_fresh_normal
    move-exception v6
    move-object v4, v6
    goto :fault_cancel

    :capture_fresh_fault
    move-exception v9
    move-object v4, v9

    :fault_cancel
    invoke-static {}, Lfixture/BackoffTwoWide;->cancel()V
    throw v4
    :fault_cancel_end

    :outer_capture
    move-exception v2
    invoke-static {v0, v1, v10, v11, v2}, Lfixture/BackoffTwoWide;->diagnostic(JJLjava/lang/Throwable;)V
    throw v2

    .catchall {:early_start .. :early_end} :capture_early
    .catchall {:save_start .. :save_end} :work_failure
    .catch Ljava/lang/InterruptedException; {:wait_start .. :wait_end} :typed_capture
    .catchall {:wait_start .. :wait_end} :work_failure
    .catchall {:interruption_start .. :interruption_end} :work_failure
    .catchall {:refresh_normal .. :refresh_normal_end} :capture_fresh_normal
    .catchall {:refresh_fault .. :refresh_fault_end} :capture_fresh_fault
    .catchall {:normal_cancel .. :normal_cancel_end} :outer_capture
    .catchall {:fault_cancel .. :fault_cancel_end} :outer_capture
.end method

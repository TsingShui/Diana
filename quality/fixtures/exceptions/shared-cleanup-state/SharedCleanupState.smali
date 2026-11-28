.class public final Lfixture/SharedCleanupState;
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

.method private static native reacquire()V
.end method

.method private static native cancel()V
.end method

.method private static native diagnostic(JLjava/lang/Throwable;)V
.end method

.method public static run()V
    .locals 10

    invoke-static {}, Lfixture/SharedCleanupState;->initialClock()J
    move-result-wide v0

    :early_start
    invoke-static {}, Lfixture/SharedCleanupState;->schedule()V
    :early_end

    :save_start
    move-wide v2, v0
    invoke-static {v2, v3}, Lfixture/SharedCleanupState;->save(J)V
    :save_end

    :wait_start
    invoke-static {}, Lfixture/SharedCleanupState;->waitFor()V
    :wait_end
    goto :refresh_normal

    :typed_capture
    move-exception v7
    :interruption_start
    invoke-static {v7}, Lfixture/SharedCleanupState;->interruption(Ljava/lang/InterruptedException;)V
    :interruption_end

    :refresh_normal
    invoke-static {}, Lfixture/SharedCleanupState;->refreshClock()J
    move-result-wide v0
    invoke-static {}, Lfixture/SharedCleanupState;->reacquire()V
    :refresh_normal_end
    goto :normal_cancel

    :work_failure
    move-exception v8
    :refresh_fault
    invoke-static {}, Lfixture/SharedCleanupState;->refreshClock()J
    move-result-wide v0
    invoke-static {}, Lfixture/SharedCleanupState;->reacquire()V
    throw v8
    :refresh_fault_end

    :normal_cancel
    invoke-static {}, Lfixture/SharedCleanupState;->cancel()V
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
    invoke-static {}, Lfixture/SharedCleanupState;->cancel()V
    throw v4
    :fault_cancel_end

    :outer_capture
    move-exception v2
    invoke-static {v0, v1, v2}, Lfixture/SharedCleanupState;->diagnostic(JLjava/lang/Throwable;)V
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

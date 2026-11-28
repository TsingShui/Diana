.class public final Lfixture/FinallyTyped;
.super Ljava/lang/Object;

.method private static native stage(I)V
.end method

.method private static native onTyped(Ljava/lang/IllegalArgumentException;)V
.end method

.method public native cleanup(I)V
.end method

.method public static run(Lfixture/FinallyTyped;I)V
    .locals 7

    const/4 v0, 0x1
    :call_one
    invoke-static {v0}, Lfixture/FinallyTyped;->stage(I)V
    :one_end
    const/4 v0, 0x2
    :call_two
    invoke-static {v0}, Lfixture/FinallyTyped;->stage(I)V
    :two_end
    goto :after_typed

    :typed_capture
    move-exception v5
    :typed_body
    invoke-static {v5}, Lfixture/FinallyTyped;->onTyped(Ljava/lang/IllegalArgumentException;)V
    :typed_end

    :after_typed
    const/4 v0, 0x3
    :call_three
    invoke-static {v0}, Lfixture/FinallyTyped;->stage(I)V
    :three_end
    invoke-virtual {p0, p1}, Lfixture/FinallyTyped;->cleanup(I)V
    return-void

    :capture_one
    move-exception v1
    move-object v4, v1
    goto :cleanup

    :capture_two
    move-exception v2
    move-object v4, v2
    goto :cleanup

    :capture_three
    move-exception v3
    move-object v4, v3
    goto :cleanup

    :capture_handler
    move-exception v6
    move-object v4, v6

    :cleanup
    invoke-virtual {p0, p1}, Lfixture/FinallyTyped;->cleanup(I)V
    throw v4

    .catchall {:call_one .. :one_end} :capture_one
    .catch Ljava/lang/IllegalArgumentException; {:call_two .. :two_end} :typed_capture
    .catchall {:call_two .. :two_end} :capture_two
    .catchall {:typed_body .. :typed_end} :capture_handler
    .catchall {:call_three .. :three_end} :capture_three
.end method

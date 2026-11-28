.class public final Lfixture/FinallyFamily;
.super Ljava/lang/Object;

.method private static native stage(I)V
.end method

.method public native cleanup(I)V
.end method

.method public static run(Lfixture/FinallyFamily;I)V
    .locals 5

    const/4 v0, 0x1
    :call_one
    invoke-static {v0}, Lfixture/FinallyFamily;->stage(I)V
    :one_end
    const/4 v0, 0x2
    :call_two
    invoke-static {v0}, Lfixture/FinallyFamily;->stage(I)V
    :two_end
    const/4 v0, 0x3
    :call_three
    invoke-static {v0}, Lfixture/FinallyFamily;->stage(I)V
    :three_end
    invoke-virtual {p0, p1}, Lfixture/FinallyFamily;->cleanup(I)V
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

    :cleanup
    invoke-virtual {p0, p1}, Lfixture/FinallyFamily;->cleanup(I)V
    throw v4

    .catchall {:call_one .. :one_end} :capture_one
    .catchall {:call_two .. :two_end} :capture_two
    .catchall {:call_three .. :three_end} :capture_three
.end method

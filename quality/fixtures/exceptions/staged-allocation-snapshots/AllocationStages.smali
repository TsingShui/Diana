.class public final Lfixture/AllocationStages;
.super Ljava/lang/Object;

.method private static native prepare()I
.end method

.method private static native argument(I)I
.end method

.method static native initialized()V
.end method

.method static native construct(III)V
.end method

.method private static native publish(Lfixture/AllocationItem;)V
.end method

.method private static native caught(ILjava/lang/Throwable;)V
.end method

.method public static run()V
    .locals 5
    const/16 v0, 10

    :protected_start
    invoke-static {}, Lfixture/AllocationStages;->prepare()I
    move-result v1
    const/16 v0, 15

    new-instance v2, Lfixture/AllocationItem;
    const/16 v0, 20
    invoke-static {v0}, Lfixture/AllocationStages;->argument(I)I
    move-result v3
    const/16 v0, 30
    invoke-direct {v2, v1, v3, v0}, Lfixture/AllocationItem;-><init>(III)V
    invoke-static {v2}, Lfixture/AllocationStages;->publish(Lfixture/AllocationItem;)V
    :protected_end
    return-void

    :capture
    move-exception v4
    invoke-static {v0, v4}, Lfixture/AllocationStages;->caught(ILjava/lang/Throwable;)V
    return-void

    .catchall {:protected_start .. :protected_end} :capture
.end method

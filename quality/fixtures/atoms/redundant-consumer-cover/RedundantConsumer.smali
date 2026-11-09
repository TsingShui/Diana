.class public final Lfixture/RedundantConsumer;
.super Ljava/lang/Object;
.method public static native made()V
.end method
.method public static native after()V
.end method
.method public static immediate()Lfixture/Container;
    .registers 2
    new-instance v0, Lfixture/Container;
    invoke-direct {v0}, Lfixture/Container;-><init>()V
    move-object v1, v0
    check-cast v1, Lfixture/Container;
    return-object v1
.end method
.method public static barrier()Lfixture/Container;
    .registers 2
    new-instance v0, Lfixture/Container;
    invoke-direct {v0}, Lfixture/Container;-><init>()V
    move-object v1, v0
    check-cast v1, Lfixture/Container;
    invoke-static {}, Lfixture/RedundantConsumer;->after()V
    return-object v1
.end method

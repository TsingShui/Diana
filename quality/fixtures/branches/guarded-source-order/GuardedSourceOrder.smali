.class public final Lfixture/GuardedSourceOrder;
.super Ljava/lang/Object;


# annotations
.annotation system Ldalvik/annotation/MemberClasses;
    value = {
        Lfixture/GuardedSourceOrder$Selection;,
        Lfixture/GuardedSourceOrder$Effects;
    }
.end annotation


# direct methods
.method public constructor <init>()V
    .locals 0

    invoke-direct {p0}, Ljava/lang/Object;-><init>()V

    return-void
.end method

.method public static collect(Ljava/lang/Object;I)Ljava/lang/Object;
    .locals 4

    const/4 v0, 0x0

    :try_start_1
    instance-of v1, p0, Lfixture/GuardedSourceOrder$Selection;

    if-eqz v1, :cond_29

    move-object v1, p0

    check-cast v1, Lfixture/GuardedSourceOrder$Selection;

    invoke-virtual {v1}, Lfixture/GuardedSourceOrder$Selection;->preview()Z

    move-result v2

    if-eqz v2, :cond_13

    invoke-static {v1}, Lfixture/GuardedSourceOrder$Effects;->prepare(Lfixture/GuardedSourceOrder$Selection;)Ljava/lang/Object;

    move-result-object v2

    goto :goto_14

    :cond_13
    move-object v2, v0

    :goto_14
    invoke-virtual {v1}, Lfixture/GuardedSourceOrder$Selection;->path()Ljava/lang/String;

    move-result-object v3

    invoke-static {v3}, Lfixture/GuardedSourceOrder$Effects;->empty(Ljava/lang/String;)Z

    move-result v3

    if-nez v3, :cond_27

    invoke-virtual {v1}, Lfixture/GuardedSourceOrder$Selection;->path()Ljava/lang/String;

    move-result-object v1

    invoke-static {v1, p1}, Lfixture/GuardedSourceOrder$Effects;->confirm(Ljava/lang/String;I)Ljava/lang/Object;

    move-result-object p1

    goto :goto_2b

    :cond_27
    move-object p1, v0

    goto :goto_2b

    :cond_29
    move-object p1, v0

    move-object v2, p1

    :goto_2b
    invoke-static {p0}, Lfixture/GuardedSourceOrder$Effects;->finish(Ljava/lang/Object;)Ljava/lang/Object;

    move-result-object p0

    invoke-static {v2, p0}, Lfixture/GuardedSourceOrder$Effects;->combine(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;

    move-result-object p0

    invoke-static {p0, p1}, Lfixture/GuardedSourceOrder$Effects;->combine(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;

    move-result-object p0
    :try_end_37
    .catch Ljava/io/InterruptedIOException; {:try_start_1 .. :try_end_37} :catch_38

    return-object p0

    :catch_38
    invoke-static {}, Lfixture/GuardedSourceOrder$Effects;->stopped()V

    return-object v0
.end method

.method public static collectAfterOpening(Ljava/lang/Object;I)Ljava/lang/Object;
    .locals 4

    const/4 v0, 0x0

    :try_start_1
    instance-of v1, p0, Lfixture/GuardedSourceOrder$Selection;

    if-eqz v1, :cond_29

    move-object v1, p0

    check-cast v1, Lfixture/GuardedSourceOrder$Selection;

    invoke-virtual {v1}, Lfixture/GuardedSourceOrder$Selection;->preview()Z

    move-result v2

    if-eqz v2, :cond_13

    invoke-static {v1, p1}, Lfixture/GuardedSourceOrder$Effects;->open(Lfixture/GuardedSourceOrder$Selection;I)Ljava/lang/Object;

    move-result-object v2

    goto :goto_14

    :cond_13
    move-object v2, v0

    :goto_14
    invoke-virtual {v1}, Lfixture/GuardedSourceOrder$Selection;->path()Ljava/lang/String;

    move-result-object v3

    invoke-static {v3}, Lfixture/GuardedSourceOrder$Effects;->empty(Ljava/lang/String;)Z

    move-result v3

    if-nez v3, :cond_27

    invoke-virtual {v1}, Lfixture/GuardedSourceOrder$Selection;->path()Ljava/lang/String;

    move-result-object v1

    invoke-static {v1, p1}, Lfixture/GuardedSourceOrder$Effects;->confirm(Ljava/lang/String;I)Ljava/lang/Object;

    move-result-object p1

    goto :goto_2b

    :cond_27
    move-object p1, v0

    goto :goto_2b

    :cond_29
    move-object p1, v0

    move-object v2, p1

    :goto_2b
    invoke-static {p0}, Lfixture/GuardedSourceOrder$Effects;->finish(Ljava/lang/Object;)Ljava/lang/Object;

    move-result-object p0

    invoke-static {v2, p0}, Lfixture/GuardedSourceOrder$Effects;->combine(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;

    move-result-object p0

    invoke-static {p0, p1}, Lfixture/GuardedSourceOrder$Effects;->combine(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;

    move-result-object p0
    :try_end_37
    .catch Ljava/io/InterruptedIOException; {:try_start_1 .. :try_end_37} :catch_38

    return-object p0

    :catch_38
    invoke-static {}, Lfixture/GuardedSourceOrder$Effects;->stopped()V

    return-object v0
.end method

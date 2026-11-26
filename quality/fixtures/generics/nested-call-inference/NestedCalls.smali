.class public final Lfixture/NestedCalls;
.super Ljava/lang/Object;

.method public static id(Ljava/lang/Object;)Ljava/lang/Object;
    .registers 1
    .annotation system Ldalvik/annotation/Signature;
        value = {"<T:Ljava/lang/Object;>(TT;)TT;"}
    .end annotation
    return-object p0
.end method

.method public static bounded(Ljava/util/List;)Ljava/util/List;
    .registers 1
    .annotation system Ldalvik/annotation/Signature;
        value = {"<T::Ljava/util/List<Ljava/lang/String;>;>(TT;)TT;"}
    .end annotation
    return-object p0
.end method

.method public static choose(ZLjava/util/List;Ljava/util/List;)Ljava/util/List;
    .registers 4
    .annotation system Ldalvik/annotation/Signature;
        value = {"(ZLjava/util/List<Ljava/lang/String;>;Ljava/util/List;)Ljava/util/List<*>;"}
    .end annotation
    if-eqz p0, :raw
    invoke-static {p1}, Lfixture/NestedCalls;->bounded(Ljava/util/List;)Ljava/util/List;
    move-result-object v0
    goto :join
    :raw
    invoke-static {p2}, Lfixture/NestedCalls;->bounded(Ljava/util/List;)Ljava/util/List;
    move-result-object v0
    :join
    invoke-static {v0}, Lfixture/NestedCalls;->id(Ljava/lang/Object;)Ljava/lang/Object;
    move-result-object v0
    check-cast v0, Ljava/util/List;
    return-object v0
.end method

.method public static chain14(Ljava/lang/String;)Ljava/lang/String;
    .registers 2
    invoke-static {p0}, Lfixture/NestedCalls;->id(Ljava/lang/Object;)Ljava/lang/Object;
    move-result-object v0
    invoke-static {v0}, Lfixture/NestedCalls;->id(Ljava/lang/Object;)Ljava/lang/Object;
    move-result-object v0
    invoke-static {v0}, Lfixture/NestedCalls;->id(Ljava/lang/Object;)Ljava/lang/Object;
    move-result-object v0
    invoke-static {v0}, Lfixture/NestedCalls;->id(Ljava/lang/Object;)Ljava/lang/Object;
    move-result-object v0
    invoke-static {v0}, Lfixture/NestedCalls;->id(Ljava/lang/Object;)Ljava/lang/Object;
    move-result-object v0
    invoke-static {v0}, Lfixture/NestedCalls;->id(Ljava/lang/Object;)Ljava/lang/Object;
    move-result-object v0
    invoke-static {v0}, Lfixture/NestedCalls;->id(Ljava/lang/Object;)Ljava/lang/Object;
    move-result-object v0
    invoke-static {v0}, Lfixture/NestedCalls;->id(Ljava/lang/Object;)Ljava/lang/Object;
    move-result-object v0
    invoke-static {v0}, Lfixture/NestedCalls;->id(Ljava/lang/Object;)Ljava/lang/Object;
    move-result-object v0
    invoke-static {v0}, Lfixture/NestedCalls;->id(Ljava/lang/Object;)Ljava/lang/Object;
    move-result-object v0
    invoke-static {v0}, Lfixture/NestedCalls;->id(Ljava/lang/Object;)Ljava/lang/Object;
    move-result-object v0
    invoke-static {v0}, Lfixture/NestedCalls;->id(Ljava/lang/Object;)Ljava/lang/Object;
    move-result-object v0
    invoke-static {v0}, Lfixture/NestedCalls;->id(Ljava/lang/Object;)Ljava/lang/Object;
    move-result-object v0
    invoke-static {v0}, Lfixture/NestedCalls;->id(Ljava/lang/Object;)Ljava/lang/Object;
    move-result-object v0
    check-cast v0, Ljava/lang/String;
    return-object v0
.end method

.method public static closedStep(Ljava/lang/String;Ljava/lang/Object;)Ljava/lang/String;
    .registers 2
    .annotation system Ldalvik/annotation/Signature;
        value = {"<T:Ljava/lang/Object;>(Ljava/lang/String;TT;)Ljava/lang/String;"}
    .end annotation
    return-object p0
.end method

.method public static closedChain(Ljava/lang/String;Ljava/lang/Object;)Ljava/lang/String;
    .registers 3
    invoke-static {p0, p1}, Lfixture/NestedCalls;->closedStep(Ljava/lang/String;Ljava/lang/Object;)Ljava/lang/String;
    move-result-object v0
    invoke-static {v0, p1}, Lfixture/NestedCalls;->closedStep(Ljava/lang/String;Ljava/lang/Object;)Ljava/lang/String;
    move-result-object v0
    invoke-static {v0, p1}, Lfixture/NestedCalls;->closedStep(Ljava/lang/String;Ljava/lang/Object;)Ljava/lang/String;
    move-result-object v0
    invoke-static {v0, p1}, Lfixture/NestedCalls;->closedStep(Ljava/lang/String;Ljava/lang/Object;)Ljava/lang/String;
    move-result-object v0
    invoke-static {v0, p1}, Lfixture/NestedCalls;->closedStep(Ljava/lang/String;Ljava/lang/Object;)Ljava/lang/String;
    move-result-object v0
    invoke-static {v0, p1}, Lfixture/NestedCalls;->closedStep(Ljava/lang/String;Ljava/lang/Object;)Ljava/lang/String;
    move-result-object v0
    invoke-static {v0, p1}, Lfixture/NestedCalls;->closedStep(Ljava/lang/String;Ljava/lang/Object;)Ljava/lang/String;
    move-result-object v0
    invoke-static {v0, p1}, Lfixture/NestedCalls;->closedStep(Ljava/lang/String;Ljava/lang/Object;)Ljava/lang/String;
    move-result-object v0
    invoke-static {v0, p1}, Lfixture/NestedCalls;->closedStep(Ljava/lang/String;Ljava/lang/Object;)Ljava/lang/String;
    move-result-object v0
    invoke-static {v0, p1}, Lfixture/NestedCalls;->closedStep(Ljava/lang/String;Ljava/lang/Object;)Ljava/lang/String;
    move-result-object v0
    invoke-static {v0, p1}, Lfixture/NestedCalls;->closedStep(Ljava/lang/String;Ljava/lang/Object;)Ljava/lang/String;
    move-result-object v0
    return-object v0
.end method

.method public static keepAfter(Ljava/util/List;Ljava/lang/Object;)Ljava/lang/Object;
    .registers 2
    .annotation system Ldalvik/annotation/Signature;
        value = {"<T:Ljava/lang/Object;>(Ljava/util/List<Ljava/lang/String;>;TT;)TT;"}
    .end annotation
    return-object p1
.end method

.method public static routed(ZLjava/util/List;Ljava/util/List;)Ljava/util/List;
    .registers 5
    .annotation system Ldalvik/annotation/Signature;
        value = {"(ZLjava/util/List<Ljava/lang/String;>;Ljava/util/List;)Ljava/util/List<Ljava/lang/String;>;"}
    .end annotation
    if-eqz p0, :from_raw
    invoke-static {p1}, Lfixture/NestedCalls;->bounded(Ljava/util/List;)Ljava/util/List;
    move-result-object v0
    goto :join
    :from_raw
    invoke-static {p2}, Lfixture/NestedCalls;->bounded(Ljava/util/List;)Ljava/util/List;
    move-result-object v0
    :join
    invoke-static {p1}, Lfixture/NestedCalls;->id(Ljava/lang/Object;)Ljava/lang/Object;
    move-result-object v1
    invoke-static {v0, v1}, Lfixture/NestedCalls;->keepAfter(Ljava/util/List;Ljava/lang/Object;)Ljava/lang/Object;
    move-result-object v0
    check-cast v0, Ljava/util/List;
    return-object v0
.end method

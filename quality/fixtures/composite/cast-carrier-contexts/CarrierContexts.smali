.class public Lfixture/CarrierContexts;
.super Ljava/lang/Object;
.method static native make()Ljava/lang/Object;
.end method
.method static triplet(Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;)I
    .registers 4
    check-cast p1, [I
    if-ne p0, p1, :different
    if-ne p1, p2, :different
    const/4 v0, 0x1
    return v0
  :different
    const/4 v0, 0x0
    return v0
.end method
.method static caught(Ljava/lang/Object;Ljava/lang/Object;)I
    .registers 4
  :start
    check-cast p1, [I
    if-ne p0, p1, :different
    const/4 v0, 0x1
    return v0
  :different
    const/4 v0, 0x0
    return v0
  :end
    .catch Ljava/lang/ClassCastException; {:start .. :end} :handler
  :handler
    move-exception v1
    const/4 v0, -0x1
    return v0
.end method
.method static acting(Ljava/lang/Object;)I
    .registers 3
    check-cast p0, [I
    invoke-static {}, Lfixture/CarrierContexts;->make()Ljava/lang/Object;
    move-result-object v0
    if-ne v0, p0, :different
    const/4 v1, 0x1
    return v1
  :different
    const/4 v1, 0x0
    return v1
.end method

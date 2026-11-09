.class public Lfixture/CarrierPreparation;
.super Ljava/lang/Object;
.method static ordered(Ljava/lang/Object;I)I
    .registers 3
    if-nez p1, :hit
    const-string v0, "x"
    check-cast p0, [I
    invoke-virtual {p0, v0}, Ljava/lang/Object;->equals(Ljava/lang/Object;)Z
    move-result v0
    if-nez v0, :hit
    const/4 v0, 0x0
    return v0
  :hit
    const/4 v0, 0x1
    return v0
.end method
.method static castFirst(Ljava/lang/Object;I)I
    .registers 3
    if-nez p1, :hit
    check-cast p0, [I
    const-string v0, "x"
    invoke-virtual {p0, v0}, Ljava/lang/Object;->equals(Ljava/lang/Object;)Z
    move-result v0
    if-nez v0, :hit
    const/4 v0, 0x0
    return v0
  :hit
    const/4 v0, 0x1
    return v0
.end method
.method static native produce()Ljava/lang/Object;
.end method
.method static native consume(Ljava/lang/Object;)I
.end method
.method static produced()I
    .registers 1
    invoke-static {}, Lfixture/CarrierPreparation;->produce()Ljava/lang/Object;
    move-result-object v0
    check-cast v0, [I
    invoke-static {v0}, Lfixture/CarrierPreparation;->consume(Ljava/lang/Object;)I
    move-result v0
    return v0
.end method

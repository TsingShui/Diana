.class public Lfixture/CarrierOrder;
.super Ljava/lang/Object;
.method static native make()Ljava/lang/Object;
.end method
.method static native sink(ILjava/lang/Object;)V
.end method
.method static right(Ljava/lang/Object;Ljava/lang/Object;)I
    .registers 3
    check-cast p1, [I
    if-ne p0, p1, :different
    const/4 v0, 0x1
    return v0
  :different
    const/4 v0, 0x0
    return v0
.end method
.method static left(Ljava/lang/Object;Ljava/lang/Object;)I
    .registers 3
    check-cast p0, [I
    if-ne p0, p1, :different
    const/4 v0, 0x1
    return v0
  :different
    const/4 v0, 0x0
    return v0
.end method
.method static guarded(Ljava/lang/Object;Ljava/lang/Object;I)I
    .registers 4
    check-cast p1, [I
    if-eqz p2, :compare
    const/16 v0, 0x63
    return v0
  :compare
    if-ne p0, p1, :different
    const/4 v0, 0x1
    return v0
  :different
    const/4 v0, 0x0
    return v0
.end method
.method static afterEffect(Ljava/lang/Object;)Ljava/lang/Object;
    .registers 2
    check-cast p0, [I
    invoke-static {}, Lfixture/CarrierOrder;->make()Ljava/lang/Object;
    move-result-object v0
    return-object p0
.end method
.method static arrayLength(Ljava/lang/Object;)I
    .registers 2
    check-cast p0, [I
    array-length v0, p0
    return v0
.end method
.method static argument(Ljava/lang/Object;)V
    .registers 2
    check-cast p0, [I
    const/4 v0, 0x0
    invoke-static {v0, p0}, Lfixture/CarrierOrder;->sink(ILjava/lang/Object;)V
    return-void
.end method

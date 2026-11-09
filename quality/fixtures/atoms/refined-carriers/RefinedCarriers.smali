.class public final Lfixture/RefinedCarriers;
.super Ljava/lang/Object;

.field public static trace:I

.method public static mark(I)V
    .locals 1
    sget v0, Lfixture/RefinedCarriers;->trace:I
    mul-int/lit8 v0, v0, 0x1f
    add-int/2addr v0, p0
    sput v0, Lfixture/RefinedCarriers;->trace:I
    return-void
.end method

.method public static identity(Ljava/lang/Object;)Ljava/lang/Object;
    .locals 1
    const/16 v0, 0xa
    invoke-static {v0}, Lfixture/RefinedCarriers;->mark(I)V
    return-object p0
.end method

.method public static tick(II)V
    .locals 2
    invoke-static {p0}, Lfixture/RefinedCarriers;->mark(I)V
    const/4 v0, 0x1
    if-ne p1, v0, :done
    new-instance v0, Ljava/lang/IllegalStateException;
    const-string v1, "tick"
    invoke-direct {v0, v1}, Ljava/lang/IllegalStateException;-><init>(Ljava/lang/String;)V
    throw v0
    :done
    return-void
.end method

.method public static declared(Ljava/lang/Object;)I
    .locals 2
    invoke-static {p0}, Lfixture/RefinedCarriers;->identity(Ljava/lang/Object;)Ljava/lang/Object;
    move-result-object v0
    check-cast v0, Ljava/lang/String;
    invoke-virtual {v0}, Ljava/lang/String;->length()I
    move-result v1
    return v1
.end method

.method public static checkedEarly(Ljava/lang/Object;I)I
    .locals 2
    check-cast p0, Ljava/lang/String;
    const/4 v0, 0x1
    invoke-static {v0, p1}, Lfixture/RefinedCarriers;->tick(II)V
    if-ltz p1, :zero
    invoke-virtual {p0}, Ljava/lang/String;->length()I
    move-result v1
    return v1
    :zero
    const/4 v1, 0x0
    return v1
.end method

.method public static bytes()[B
    .locals 1
    const/4 v0, 0x3
    new-array v0, v0, [B
    fill-array-data v0, :values
    return-object v0
    :values
    .array-data 1
        0x1t
        0x2t
        0x3t
    .end array-data
.end method

.method public static guarded(Ljava/lang/Object;)[B
    .locals 1
    const/4 v0, 0x3
    new-array v0, v0, [B
    check-cast p0, Ljava/lang/String;
    fill-array-data v0, :values
    return-object v0
    :values
    .array-data 1
        0x1t
        0x2t
        0x3t
    .end array-data
.end method

.class public final Lfixture/ReadPreparedDefaults;
.super Ljava/lang/Object;

.method public static encodedZero()I
    .locals 1
    const/4 v0, 0
    return v0
.end method

.method public static preparedNull()Ljava/lang/String;
    .locals 1
    sget-object v0, Lfixture/PreparedDefaults;->D_PREPARED_NULL:Ljava/lang/String;
    return-object v0
.end method

.method public static preparedZero()I
    .locals 1
    sget v0, Lfixture/PreparedDefaults;->C_PREPARED_ZERO:I
    return v0
.end method

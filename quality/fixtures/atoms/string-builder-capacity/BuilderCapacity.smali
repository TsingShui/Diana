.class public final Lfixture/BuilderCapacity;
.super Ljava/lang/Object;

.method private static native produce(II)Ljava/lang/String;
.end method

.method private static native caught(Ljava/lang/String;Ljava/lang/Throwable;)Ljava/lang/String;
.end method

.method public static builder(II)Ljava/lang/String;
    .locals 4
    new-instance v0, Ljava/lang/StringBuilder;
    invoke-direct {v0, p0}, Ljava/lang/StringBuilder;-><init>(I)V
    const/4 v2, 1
    invoke-static {v2, p1}, Lfixture/BuilderCapacity;->produce(II)Ljava/lang/String;
    move-result-object v1
    invoke-virtual {v0, v1}, Ljava/lang/StringBuilder;->append(Ljava/lang/String;)Ljava/lang/StringBuilder;
    move-result-object v0
    const/4 v2, 2
    invoke-static {v2, p1}, Lfixture/BuilderCapacity;->produce(II)Ljava/lang/String;
    move-result-object v1
    invoke-virtual {v0, v1}, Ljava/lang/StringBuilder;->append(Ljava/lang/String;)Ljava/lang/StringBuilder;
    move-result-object v0
    invoke-virtual {v0}, Ljava/lang/StringBuilder;->toString()Ljava/lang/String;
    move-result-object v1
    return-object v1
.end method

.method public static buffer(II)Ljava/lang/String;
    .locals 4
    new-instance v0, Ljava/lang/StringBuffer;
    invoke-direct {v0, p0}, Ljava/lang/StringBuffer;-><init>(I)V
    const/4 v2, 1
    invoke-static {v2, p1}, Lfixture/BuilderCapacity;->produce(II)Ljava/lang/String;
    move-result-object v1
    invoke-virtual {v0, v1}, Ljava/lang/StringBuffer;->append(Ljava/lang/String;)Ljava/lang/StringBuffer;
    move-result-object v0
    const/4 v2, 2
    invoke-static {v2, p1}, Lfixture/BuilderCapacity;->produce(II)Ljava/lang/String;
    move-result-object v1
    invoke-virtual {v0, v1}, Ljava/lang/StringBuffer;->append(Ljava/lang/String;)Ljava/lang/StringBuffer;
    move-result-object v0
    invoke-virtual {v0}, Ljava/lang/StringBuffer;->toString()Ljava/lang/String;
    move-result-object v1
    return-object v1
.end method

.method public static protectedBuilder(II)Ljava/lang/String;
    .locals 4
    :try_start
    new-instance v0, Ljava/lang/StringBuilder;
    invoke-direct {v0, p0}, Ljava/lang/StringBuilder;-><init>(I)V
    const/4 v2, 1
    invoke-static {v2, p1}, Lfixture/BuilderCapacity;->produce(II)Ljava/lang/String;
    move-result-object v1
    invoke-virtual {v0, v1}, Ljava/lang/StringBuilder;->append(Ljava/lang/String;)Ljava/lang/StringBuilder;
    move-result-object v0
    const/4 v2, 2
    invoke-static {v2, p1}, Lfixture/BuilderCapacity;->produce(II)Ljava/lang/String;
    move-result-object v1
    invoke-virtual {v0, v1}, Ljava/lang/StringBuilder;->append(Ljava/lang/String;)Ljava/lang/StringBuilder;
    move-result-object v0
    invoke-virtual {v0}, Ljava/lang/StringBuilder;->toString()Ljava/lang/String;
    move-result-object v1
    :try_end
    return-object v1

    :catch_capacity
    move-exception v3
    const-string v2, "capacity"
    invoke-static {v2, v3}, Lfixture/BuilderCapacity;->caught(Ljava/lang/String;Ljava/lang/Throwable;)Ljava/lang/String;
    move-result-object v1
    return-object v1

    :catch_producer
    move-exception v3
    const-string v2, "producer"
    invoke-static {v2, v3}, Lfixture/BuilderCapacity;->caught(Ljava/lang/String;Ljava/lang/Throwable;)Ljava/lang/String;
    move-result-object v1
    return-object v1

    .catch Ljava/lang/NegativeArraySizeException; {:try_start .. :try_end} :catch_capacity
    .catch Ljava/lang/IllegalStateException; {:try_start .. :try_end} :catch_producer
.end method

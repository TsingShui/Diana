.class public final Lfixture/BuilderConversion;
.super Ljava/lang/Object;

.method private static native produce(II)Ljava/lang/Object;
.end method

.method private static native caught(Ljava/lang/String;Ljava/lang/Throwable;)Ljava/lang/String;
.end method

.method public static precomputed(I)Ljava/lang/String;
    .locals 5
    const/4 v3, 1
    invoke-static {v3, p0}, Lfixture/BuilderConversion;->produce(II)Ljava/lang/Object;
    move-result-object v1
    const/4 v3, 2
    invoke-static {v3, p0}, Lfixture/BuilderConversion;->produce(II)Ljava/lang/Object;
    move-result-object v2
    new-instance v0, Ljava/lang/StringBuilder;
    invoke-direct {v0}, Ljava/lang/StringBuilder;-><init>()V
    invoke-virtual {v0, v1}, Ljava/lang/StringBuilder;->append(Ljava/lang/Object;)Ljava/lang/StringBuilder;
    move-result-object v0
    invoke-virtual {v0, v2}, Ljava/lang/StringBuilder;->append(Ljava/lang/Object;)Ljava/lang/StringBuilder;
    move-result-object v0
    invoke-virtual {v0}, Ljava/lang/StringBuilder;->toString()Ljava/lang/String;
    move-result-object v0
    return-object v0
.end method

.method public static protectedBuffer(I)Ljava/lang/String;
    .locals 5
    :try_start
    const/4 v3, 1
    invoke-static {v3, p0}, Lfixture/BuilderConversion;->produce(II)Ljava/lang/Object;
    move-result-object v1
    const/4 v3, 2
    invoke-static {v3, p0}, Lfixture/BuilderConversion;->produce(II)Ljava/lang/Object;
    move-result-object v2
    new-instance v0, Ljava/lang/StringBuffer;
    invoke-direct {v0}, Ljava/lang/StringBuffer;-><init>()V
    invoke-virtual {v0, v1}, Ljava/lang/StringBuffer;->append(Ljava/lang/Object;)Ljava/lang/StringBuffer;
    move-result-object v0
    invoke-virtual {v0, v2}, Ljava/lang/StringBuffer;->append(Ljava/lang/Object;)Ljava/lang/StringBuffer;
    move-result-object v0
    invoke-virtual {v0}, Ljava/lang/StringBuffer;->toString()Ljava/lang/String;
    move-result-object v0
    :try_end
    return-object v0

    :catch_producer
    move-exception v4
    const-string v3, "producer"
    invoke-static {v3, v4}, Lfixture/BuilderConversion;->caught(Ljava/lang/String;Ljava/lang/Throwable;)Ljava/lang/String;
    move-result-object v0
    return-object v0

    :catch_conversion
    move-exception v4
    const-string v3, "conversion"
    invoke-static {v3, v4}, Lfixture/BuilderConversion;->caught(Ljava/lang/String;Ljava/lang/Throwable;)Ljava/lang/String;
    move-result-object v0
    return-object v0

    .catch Ljava/lang/IllegalArgumentException; {:try_start .. :try_end} :catch_producer
    .catch Ljava/lang/IllegalStateException; {:try_start .. :try_end} :catch_conversion
.end method

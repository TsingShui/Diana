.class public final Lfixture/ConstructorArguments;
.super Ljava/lang/Object;
.method public static native getSystem()Lfixture/Resources;
.end method
.method public static native note(Ljava/lang/Throwable;)V
.end method
.method public static point()Lfixture/Point;
    .locals 5
    new-instance v0, Lfixture/Point;
    invoke-static {}, Lfixture/ConstructorArguments;->getSystem()Lfixture/Resources;
    move-result-object v1
    invoke-virtual {v1}, Lfixture/Resources;->getDisplayMetrics()Lfixture/Metrics;
    move-result-object v2
    iget v3, v2, Lfixture/Metrics;->widthPixels:I
    invoke-static {}, Lfixture/ConstructorArguments;->getSystem()Lfixture/Resources;
    move-result-object v1
    invoke-virtual {v1}, Lfixture/Resources;->getDisplayMetrics()Lfixture/Metrics;
    move-result-object v2
    iget v4, v2, Lfixture/Metrics;->heightPixels:I
    invoke-direct {v0, v3, v4}, Lfixture/Point;-><init>(II)V
    return-object v0
.end method
.method public static caughtPoint()Lfixture/Point;
    .locals 5
    :body
    new-instance v0, Lfixture/Point;
    invoke-static {}, Lfixture/ConstructorArguments;->getSystem()Lfixture/Resources;
    move-result-object v1
    invoke-virtual {v1}, Lfixture/Resources;->getDisplayMetrics()Lfixture/Metrics;
    move-result-object v2
    iget v3, v2, Lfixture/Metrics;->widthPixels:I
    invoke-static {}, Lfixture/ConstructorArguments;->getSystem()Lfixture/Resources;
    move-result-object v1
    invoke-virtual {v1}, Lfixture/Resources;->getDisplayMetrics()Lfixture/Metrics;
    move-result-object v2
    iget v4, v2, Lfixture/Metrics;->heightPixels:I
    invoke-direct {v0, v3, v4}, Lfixture/Point;-><init>(II)V
    :body_end
    return-object v0
    :handler
    move-exception v0
    invoke-static {v0}, Lfixture/ConstructorArguments;->note(Ljava/lang/Throwable;)V
    const/4 v0, 0x0
    return-object v0
    .catchall {:body .. :body_end} :handler
.end method
.method public static bodyError(I)V
    .locals 2
    :body
    if-eqz p0, :done
    new-instance v0, Ljava/lang/IllegalStateException;
    const-string v1, "body"
    invoke-direct {v0, v1}, Ljava/lang/IllegalStateException;-><init>(Ljava/lang/String;)V
    throw v0
    :body_end
    :done
    return-void
    :handler
    move-exception v0
    invoke-static {v0}, Lfixture/ConstructorArguments;->note(Ljava/lang/Throwable;)V
    return-void
    .catchall {:body .. :body_end} :handler
.end method

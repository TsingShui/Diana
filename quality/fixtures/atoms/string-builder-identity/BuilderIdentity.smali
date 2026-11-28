.class public final Lfixture/BuilderIdentity;
.super Ljava/lang/Object;

.field public static final SUFFIX:Ljava/lang/String; = "fixed"

.method private static native observe(Ljava/lang/String;)V
.end method

.method public static literalBuilder()V
    .locals 2
    new-instance v0, Ljava/lang/StringBuilder;
    invoke-direct {v0}, Ljava/lang/StringBuilder;-><init>()V
    const-string v1, "prefix"
    invoke-virtual {v0, v1}, Ljava/lang/StringBuilder;->append(Ljava/lang/String;)Ljava/lang/StringBuilder;
    move-result-object v0
    const-string v1, "suffix"
    invoke-virtual {v0, v1}, Ljava/lang/StringBuilder;->append(Ljava/lang/String;)Ljava/lang/StringBuilder;
    move-result-object v0
    invoke-virtual {v0}, Ljava/lang/StringBuilder;->toString()Ljava/lang/String;
    move-result-object v1
    invoke-static {v1}, Lfixture/BuilderIdentity;->observe(Ljava/lang/String;)V
    return-void
.end method

.method public static singleObjectLiteral()V
    .locals 2
    new-instance v0, Ljava/lang/StringBuilder;
    invoke-direct {v0}, Ljava/lang/StringBuilder;-><init>()V
    const-string v1, "payload"
    invoke-virtual {v0, v1}, Ljava/lang/StringBuilder;->append(Ljava/lang/Object;)Ljava/lang/StringBuilder;
    move-result-object v0
    invoke-virtual {v0}, Ljava/lang/StringBuilder;->toString()Ljava/lang/String;
    move-result-object v1
    invoke-static {v1}, Lfixture/BuilderIdentity;->observe(Ljava/lang/String;)V
    return-void
.end method

.method public static primitiveBuffer()V
    .locals 2
    new-instance v0, Ljava/lang/StringBuffer;
    invoke-direct {v0}, Ljava/lang/StringBuffer;-><init>()V
    const/16 v1, 37
    invoke-virtual {v0, v1}, Ljava/lang/StringBuffer;->append(I)Ljava/lang/StringBuffer;
    move-result-object v0
    const/16 v1, 58
    invoke-virtual {v0, v1}, Ljava/lang/StringBuffer;->append(C)Ljava/lang/StringBuffer;
    move-result-object v0
    const/16 v1, 1
    invoke-virtual {v0, v1}, Ljava/lang/StringBuffer;->append(Z)Ljava/lang/StringBuffer;
    move-result-object v0
    invoke-virtual {v0}, Ljava/lang/StringBuffer;->toString()Ljava/lang/String;
    move-result-object v1
    invoke-static {v1}, Lfixture/BuilderIdentity;->observe(Ljava/lang/String;)V
    return-void
.end method

.method public static dynamicSeed(I)V
    .locals 2
    new-instance v0, Ljava/lang/StringBuilder;
    const-string v1, "value="
    invoke-direct {v0, v1}, Ljava/lang/StringBuilder;-><init>(Ljava/lang/String;)V
    invoke-virtual {v0, p0}, Ljava/lang/StringBuilder;->append(I)Ljava/lang/StringBuilder;
    move-result-object v0
    invoke-virtual {v0}, Ljava/lang/StringBuilder;->toString()Ljava/lang/String;
    move-result-object v1
    invoke-static {v1}, Lfixture/BuilderIdentity;->observe(Ljava/lang/String;)V
    return-void
.end method


.method public static arithmeticBuffer()V
    .locals 3
    const/16 v1, 127
    add-int/lit8 v1, v1, 2
    int-to-byte v1, v1
    new-instance v0, Ljava/lang/StringBuffer;
    const-string v2, "number="
    invoke-direct {v0, v2}, Ljava/lang/StringBuffer;-><init>(Ljava/lang/String;)V
    invoke-virtual {v0, v1}, Ljava/lang/StringBuffer;->append(I)Ljava/lang/StringBuffer;
    move-result-object v0
    invoke-virtual {v0}, Ljava/lang/StringBuffer;->toString()Ljava/lang/String;
    move-result-object v1
    invoke-static {v1}, Lfixture/BuilderIdentity;->observe(Ljava/lang/String;)V
    return-void
.end method

.method public static staticString()V
    .locals 2
    new-instance v0, Ljava/lang/StringBuilder;
    const-string v1, "field="
    invoke-direct {v0, v1}, Ljava/lang/StringBuilder;-><init>(Ljava/lang/String;)V
    sget-object v1, Lfixture/BuilderIdentity;->SUFFIX:Ljava/lang/String;
    invoke-virtual {v0, v1}, Ljava/lang/StringBuilder;->append(Ljava/lang/String;)Ljava/lang/StringBuilder;
    move-result-object v0
    invoke-virtual {v0}, Ljava/lang/StringBuilder;->toString()Ljava/lang/String;
    move-result-object v1
    invoke-static {v1}, Lfixture/BuilderIdentity;->observe(Ljava/lang/String;)V
    return-void
.end method

.method public static branchedString(Z)V
    .locals 3
    if-eqz p0, :left
    const-string v1, "right"
    goto :common

    :left
    const-string v1, "left"

    :common
    new-instance v0, Ljava/lang/StringBuilder;
    const-string v2, "route="
    invoke-direct {v0, v2}, Ljava/lang/StringBuilder;-><init>(Ljava/lang/String;)V
    invoke-virtual {v0, v1}, Ljava/lang/StringBuilder;->append(Ljava/lang/String;)Ljava/lang/StringBuilder;
    move-result-object v0
    invoke-virtual {v0}, Ljava/lang/StringBuilder;->toString()Ljava/lang/String;
    move-result-object v1
    invoke-static {v1}, Lfixture/BuilderIdentity;->observe(Ljava/lang/String;)V
    return-void
.end method

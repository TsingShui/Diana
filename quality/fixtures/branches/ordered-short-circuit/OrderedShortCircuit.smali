.class public final Lfixture/OrderedShortCircuit;
.super Ljava/lang/Object;

.method public static run(Ljava/lang/String;)Z
    .locals 1
    :loop
    if-eqz p0, :exhausted

    invoke-static {p0}, Lfixture/OrderedShortCircuit;->acceptLeft(Ljava/lang/String;)Z
    move-result v0
    if-nez v0, :left_true

    invoke-static {p0}, Lfixture/OrderedShortCircuit;->acceptRight(Ljava/lang/String;)Z
    move-result v0
    if-nez v0, :right_true

    invoke-static {p0}, Lfixture/OrderedShortCircuit;->next(Ljava/lang/String;)Ljava/lang/String;
    move-result-object p0
    goto :loop

    :left_true
    const/4 v0, 0x1
    return v0

    :right_true
    const/4 v0, 0x1
    return v0

    :exhausted
    const/4 v0, 0
    return v0
.end method

.method private static native acceptLeft(Ljava/lang/String;)Z
.end method

.method private static native acceptRight(Ljava/lang/String;)Z
.end method

.method private static native next(Ljava/lang/String;)Ljava/lang/String;
.end method

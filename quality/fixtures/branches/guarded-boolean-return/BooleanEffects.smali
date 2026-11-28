.class public final Lfixture/BooleanEffects;
.super Ljava/lang/Object;
.field public static trace:I
.field public static flags:I
.field public static throwAt:I
.field public static failure:Ljava/lang/IllegalStateException;

.method private static observe(I)Z
    .locals 2
    sget v0, Lfixture/BooleanEffects;->trace:I
    mul-int/lit8 v0, v0, 0xa
    add-int/2addr v0, p0
    sput v0, Lfixture/BooleanEffects;->trace:I
    sget v0, Lfixture/BooleanEffects;->throwAt:I
    if-ne p0, v0, :value
    sget-object v0, Lfixture/BooleanEffects;->failure:Ljava/lang/IllegalStateException;
    throw v0
:value
    sget v0, Lfixture/BooleanEffects;->flags:I
    add-int/lit8 v1, p0, -0x1
    ushr-int/2addr v0, v1
    and-int/lit8 v0, v0, 0x1
    return v0
.end method

.method public static first()Z
    .locals 1
    const/4 v0, 0x1
    invoke-static {v0}, Lfixture/BooleanEffects;->observe(I)Z
    move-result v0
    return v0
.end method

.method public static second()Z
    .locals 1
    const/4 v0, 0x2
    invoke-static {v0}, Lfixture/BooleanEffects;->observe(I)Z
    move-result v0
    return v0
.end method

.method public static third()Z
    .locals 1
    const/4 v0, 0x3
    invoke-static {v0}, Lfixture/BooleanEffects;->observe(I)Z
    move-result v0
    return v0
.end method

.method public static fourth()Z
    .locals 1
    const/4 v0, 0x4
    invoke-static {v0}, Lfixture/BooleanEffects;->observe(I)Z
    move-result v0
    return v0
.end method

.method public static mark()V
    .locals 1
    const/4 v0, 0x5
    invoke-static {v0}, Lfixture/BooleanEffects;->observe(I)Z
    return-void
.end method

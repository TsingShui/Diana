.class public final Lfixture/SwitchFallthrough;
.super Ljava/lang/Object;

.field public static state:I
.field public static observed:F

.method public static mark(I)V
    .locals 1
    sget v0, Lfixture/SwitchFallthrough;->state:I
    mul-int/lit8 v0, v0, 0xa
    add-int/2addr v0, p0
    sput v0, Lfixture/SwitchFallthrough;->state:I
    return-void
.end method

.method public static observeFloat(F)V
    .locals 0
    sput p0, Lfixture/SwitchFallthrough;->observed:F
    return-void
.end method

.method public static mixedBits(I)I
    .locals 1
    const v0, 0x3f800000
    invoke-static {v0}, Lfixture/SwitchFallthrough;->observeFloat(F)V
    packed-switch p0, :table
    const/4 v0, -0x1
    goto :done
    :ten
    add-int/lit8 v0, v0, 0x1
    :eleven
    add-int/lit8 v0, v0, 0x2
    :done
    return v0
    :table
    .packed-switch 0xa
        :ten
        :eleven
    .end packed-switch
.end method

.method public static shared(I)I
    .locals 1
    packed-switch p0, :table
    const/4 v0, -0x1
    return v0
    :shared
    const/4 v0, 0x1
    invoke-static {v0}, Lfixture/SwitchFallthrough;->mark(I)V
    const/16 v0, 0xb
    return v0
    :third
    const/4 v0, 0x3
    invoke-static {v0}, Lfixture/SwitchFallthrough;->mark(I)V
    const/16 v0, 0x21
    return v0
    :table
    .packed-switch 0x1
        :shared
        :shared
        :third
    .end packed-switch
.end method

.method public static chain(I)I
    .locals 2
    const/4 v0, 0x0
    packed-switch p0, :table
    const/4 v0, -0x1
    goto :done
    :ten
    const/4 v1, 0x1
    invoke-static {v1}, Lfixture/SwitchFallthrough;->mark(I)V
    add-int/lit8 v0, v0, 0x1
    :eleven
    const/4 v1, 0x2
    invoke-static {v1}, Lfixture/SwitchFallthrough;->mark(I)V
    add-int/lit8 v0, v0, 0x2
    goto :done
    :twelve
    const/4 v0, 0x3
    invoke-static {v0}, Lfixture/SwitchFallthrough;->mark(I)V
    :done
    return v0
    :table
    .packed-switch 0xa
        :ten
        :eleven
        :twelve
    .end packed-switch
.end method

.method public static parameterSeed(II)I
    .locals 0
    packed-switch p0, :table
    const/4 p1, -0x1
    goto :done
    :ten
    add-int/lit8 p1, p1, 0x1
    :eleven
    add-int/lit8 p1, p1, 0x2
    :done
    return p1
    :table
    .packed-switch 0xa
        :ten
        :eleven
    .end packed-switch
.end method

.method public static emptyDefault(I)I
    .locals 2
    const/4 v0, 0x0
    packed-switch p0, :table
    :done
    return v0
    :one
    const/4 v0, 0x3
    const/4 v1, 0x1
    invoke-static {v1}, Lfixture/SwitchFallthrough;->mark(I)V
    goto :done
    :two
    const/4 v0, 0x5
    const/4 v1, 0x2
    invoke-static {v1}, Lfixture/SwitchFallthrough;->mark(I)V
    goto :done
    :table
    .packed-switch 0x1
        :one
        :two
    .end packed-switch
.end method

.method public static conditional(II)I
    .locals 1
    packed-switch p0, :table
    const/16 v0, 0x9
    invoke-static {v0}, Lfixture/SwitchFallthrough;->mark(I)V
    goto :done
    :one
    const/4 v0, 0x1
    invoke-static {v0}, Lfixture/SwitchFallthrough;->mark(I)V
    if-eqz p1, :done
    :two
    const/4 v0, 0x2
    invoke-static {v0}, Lfixture/SwitchFallthrough;->mark(I)V
    :done
    sget v0, Lfixture/SwitchFallthrough;->state:I
    return v0
    :table
    .packed-switch 0x1
        :one
        :two
    .end packed-switch
.end method

.method public static sharedTail(I)I
    .locals 1
    packed-switch p0, :table
    const/16 v0, 0x9
    invoke-static {v0}, Lfixture/SwitchFallthrough;->mark(I)V
    goto :done
    :one
    const/4 v0, 0x1
    invoke-static {v0}, Lfixture/SwitchFallthrough;->mark(I)V
    goto :three
    :two
    const/4 v0, 0x2
    invoke-static {v0}, Lfixture/SwitchFallthrough;->mark(I)V
    :three
    const/4 v0, 0x3
    invoke-static {v0}, Lfixture/SwitchFallthrough;->mark(I)V
    :done
    sget v0, Lfixture/SwitchFallthrough;->state:I
    return v0
    :table
    .packed-switch 0x1
        :one
        :two
        :three
    .end packed-switch
.end method

.method public static wide(I)J
    .locals 4
    const-wide/16 v0, 0x0
    packed-switch p0, :table
    const-wide/16 v0, -0x1
    goto :done
    :ten
    const-wide/16 v2, 0x1
    add-long/2addr v0, v2
    :eleven
    const-wide/16 v2, 0x2
    add-long/2addr v0, v2
    :done
    return-wide v0
    :table
    .packed-switch 0xa
        :ten
        :eleven
    .end packed-switch
.end method

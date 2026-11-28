.class public final Lorigin/PrefixOwner;
.super Ljava/lang/Object;

.field public static value:I

.method static constructor <clinit>()V
    .locals 1
    const/4 v0, 0x7
    sput v0, Loutside/PrefixOwner;->value:I
    return-void
.end method

.method public static observed()I
    .locals 1
    sget v0, Lorigin/PrefixOwner;->value:I
    return v0
.end method

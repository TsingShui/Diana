.class public Lfixture/BrokenInitializer;
.super Ljava/lang/Object;

.field public static leaked:Ljava/lang/Object;

.method static constructor <clinit>()V
    .registers 1
    new-instance v0, Ljava/lang/Object;
    sput-object v0, Lfixture/BrokenInitializer;->leaked:Ljava/lang/Object;
    return-void
.end method

.method public static sibling()I
    .registers 1
    const/16 v0, 11
    return v0
.end method

.class public interface abstract Lfixture/RuntimeFace;
.super Ljava/lang/Object;

.field public static final FIRST:Ljava/lang/String;
.field public static final SECOND:Ljava/lang/String;

.method static constructor <clinit>()V
    .locals 1
    const-string v0, "first"
    sput-object v0, Lfixture/RuntimeFace;->FIRST:Ljava/lang/String;
    invoke-static {}, Lfixture/InterfaceEffects;->second()Ljava/lang/String;
    move-result-object v0
    sput-object v0, Lfixture/RuntimeFace;->SECOND:Ljava/lang/String;
    return-void
.end method

.method public static readFirst()Ljava/lang/String;
    .locals 1
    sget-object v0, Lfixture/RuntimeFace;->FIRST:Ljava/lang/String;
    return-object v0
.end method

.method public static readSecond()Ljava/lang/String;
    .locals 1
    sget-object v0, Lfixture/RuntimeFace;->SECOND:Ljava/lang/String;
    return-object v0
.end method

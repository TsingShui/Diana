.class public final Lfixture/DexProvider;
.super Ljava/lang/Object;

.field public static final COMPILE_TEXT:Ljava/lang/String; = "compiled"
.field public static final COMPILE_NUMBER:I = 42
.field public static final RUNTIME_TEXT:Ljava/lang/String;
.field public static final RUNTIME_NUMBER:I

.method static constructor <clinit>()V
    .locals 2

    const-string v0, "runtime"
    sput-object v0, Lfixture/DexProvider;->RUNTIME_TEXT:Ljava/lang/String;

    const/16 v1, 7
    sput v1, Lfixture/DexProvider;->RUNTIME_NUMBER:I

    sget-object v0, Lfixture/DexProvider;->RUNTIME_TEXT:Ljava/lang/String;
    sget v1, Lfixture/DexProvider;->RUNTIME_NUMBER:I
    invoke-static {v0, v1}, Lfixture/DexInitEvents;->begin(Ljava/lang/String;I)V
    invoke-static {}, Lfixture/DexInitEvents;->complete()V
    return-void
.end method

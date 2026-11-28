.class public final Lfixture/GuardedSourceOrder$Selection;
.super Ljava/lang/Object;


# annotations
.annotation system Ldalvik/annotation/EnclosingClass;
    value = Lfixture/GuardedSourceOrder;
.end annotation

.annotation system Ldalvik/annotation/InnerClass;
    accessFlags = 0x19
    name = "Selection"
.end annotation


# instance fields
.field public final preview:Z

.field public remote:Z


# direct methods
.method public constructor <init>(ZZ)V
    .locals 0

    invoke-direct {p0}, Ljava/lang/Object;-><init>()V

    iput-boolean p1, p0, Lfixture/GuardedSourceOrder$Selection;->preview:Z

    iput-boolean p2, p0, Lfixture/GuardedSourceOrder$Selection;->remote:Z

    return-void
.end method


# virtual methods
.method public path()Ljava/lang/String;
    .locals 0

    iget-boolean p0, p0, Lfixture/GuardedSourceOrder$Selection;->remote:Z

    if-eqz p0, :cond_7

    const-string p0, "remote"

    return-object p0

    :cond_7
    const-string p0, ""

    return-object p0
.end method

.method public preview()Z
    .locals 0

    iget-boolean p0, p0, Lfixture/GuardedSourceOrder$Selection;->preview:Z

    return p0
.end method

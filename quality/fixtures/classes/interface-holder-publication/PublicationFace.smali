.class public interface abstract Lfixture/PublicationFace;
.super Ljava/lang/Object;

.field public static final FIRST:Ljava/lang/Object;
.field public static final SECOND:Ljava/lang/Object;

.method static constructor <clinit>()V
    .locals 1
    new-instance v0, Ljava/lang/Object;
    invoke-direct {v0}, Ljava/lang/Object;-><init>()V
    sput-object v0, Lfixture/PublicationFace;->FIRST:Ljava/lang/Object;
    invoke-static {}, Lfixture/PublicationFace;->observe()V
    sput-object v0, Lfixture/PublicationFace;->SECOND:Ljava/lang/Object;
    return-void
.end method

.method public static observe()V
    .locals 1
    sget-object v0, Lfixture/PublicationFace;->FIRST:Ljava/lang/Object;
    sput-object v0, Lfixture/HolderObservation;->seen:Ljava/lang/Object;
    return-void
.end method

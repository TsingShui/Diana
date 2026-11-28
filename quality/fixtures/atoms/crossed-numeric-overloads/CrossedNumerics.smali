.class public final Lfixture/CrossedNumerics;
.super Ljava/lang/Object;

.method private static native record(Ljava/lang/String;II)V
.end method

.method public constructor <init>(CC)V
    .locals 1
    invoke-direct {p0}, Ljava/lang/Object;-><init>()V
    const-string v0, "constructor:CC"
    invoke-static {v0, p1, p2}, Lfixture/CrossedNumerics;->record(Ljava/lang/String;II)V
    return-void
.end method

.method public constructor <init>(CI)V
    .locals 1
    invoke-direct {p0}, Ljava/lang/Object;-><init>()V
    const-string v0, "constructor:CI"
    invoke-static {v0, p1, p2}, Lfixture/CrossedNumerics;->record(Ljava/lang/String;II)V
    return-void
.end method

.method public constructor <init>(IC)V
    .locals 1
    invoke-direct {p0}, Ljava/lang/Object;-><init>()V
    const-string v0, "constructor:IC"
    invoke-static {v0, p1, p2}, Lfixture/CrossedNumerics;->record(Ljava/lang/String;II)V
    return-void
.end method

.method public constructor <init>(II)V
    .locals 1
    invoke-direct {p0}, Ljava/lang/Object;-><init>()V
    const-string v0, "constructor:II"
    invoke-static {v0, p1, p2}, Lfixture/CrossedNumerics;->record(Ljava/lang/String;II)V
    return-void
.end method

.method public mix(CC)V
    .locals 1
    const-string v0, "char:CC"
    invoke-static {v0, p1, p2}, Lfixture/CrossedNumerics;->record(Ljava/lang/String;II)V
    return-void
.end method

.method public mix(CI)V
    .locals 1
    const-string v0, "char:CI"
    invoke-static {v0, p1, p2}, Lfixture/CrossedNumerics;->record(Ljava/lang/String;II)V
    return-void
.end method

.method public mix(IC)V
    .locals 1
    const-string v0, "char:IC"
    invoke-static {v0, p1, p2}, Lfixture/CrossedNumerics;->record(Ljava/lang/String;II)V
    return-void
.end method

.method public mix(II)V
    .locals 1
    const-string v0, "char:II"
    invoke-static {v0, p1, p2}, Lfixture/CrossedNumerics;->record(Ljava/lang/String;II)V
    return-void
.end method

.method public octet(BB)V
    .locals 1
    const-string v0, "byte:BB"
    invoke-static {v0, p1, p2}, Lfixture/CrossedNumerics;->record(Ljava/lang/String;II)V
    return-void
.end method

.method public octet(BI)V
    .locals 1
    const-string v0, "byte:BI"
    invoke-static {v0, p1, p2}, Lfixture/CrossedNumerics;->record(Ljava/lang/String;II)V
    return-void
.end method

.method public octet(IB)V
    .locals 1
    const-string v0, "byte:IB"
    invoke-static {v0, p1, p2}, Lfixture/CrossedNumerics;->record(Ljava/lang/String;II)V
    return-void
.end method

.method public octet(II)V
    .locals 1
    const-string v0, "byte:II"
    invoke-static {v0, p1, p2}, Lfixture/CrossedNumerics;->record(Ljava/lang/String;II)V
    return-void
.end method

.method public static run(CB)V
    .locals 2
    new-instance v0, Lfixture/CrossedNumerics;
    invoke-direct {v0, p0, p0}, Lfixture/CrossedNumerics;-><init>(CI)V
    invoke-virtual {v0, p0, p0}, Lfixture/CrossedNumerics;->mix(CI)V
    invoke-virtual {v0, p0, p0}, Lfixture/CrossedNumerics;->mix(IC)V
    invoke-virtual {v0, p1, p1}, Lfixture/CrossedNumerics;->octet(BI)V
    invoke-virtual {v0, p1, p1}, Lfixture/CrossedNumerics;->octet(IB)V
    new-instance v1, Lfixture/CrossedNumerics;
    invoke-direct {v1, p0, p0}, Lfixture/CrossedNumerics;-><init>(IC)V
    return-void
.end method

.method public static control(CBI)V
    .locals 1
    new-instance v0, Lfixture/CrossedNumerics;
    invoke-direct {v0, p0, p0}, Lfixture/CrossedNumerics;-><init>(CC)V
    invoke-virtual {v0, p0, p0}, Lfixture/CrossedNumerics;->mix(CC)V
    invoke-virtual {v0, p1, p1}, Lfixture/CrossedNumerics;->octet(BB)V
    invoke-virtual {v0, p2, p2}, Lfixture/CrossedNumerics;->mix(II)V
    invoke-virtual {v0, p2, p2}, Lfixture/CrossedNumerics;->octet(II)V
    return-void
.end method

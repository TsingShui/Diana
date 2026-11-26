.class public final Lfixture/WordOrder;
.super Ljava/lang/Object;

.method public static native take(JI)I
.end method
.method public static native take(DJI)I
.end method
.method public static native finish(II[I)I
.end method

.method public static run(DJILjava/lang/invoke/MethodHandle;)I
    .locals 5
    invoke-static {p2, p3, p4}, Lfixture/WordOrder;->take(JI)I
    move-result v0
    invoke-static/range {p0 .. p4}, Lfixture/WordOrder;->take(DJI)I
    move-result v1
    move-object v2, p5
    move-wide v3, p2
    invoke-polymorphic/range {v2 .. v4}, Ljava/lang/invoke/MethodHandle;->invokeExact([Ljava/lang/Object;)Ljava/lang/Object;, (J)I
    move-result v2
    filled-new-array/range {v0 .. v2}, [I
    move-result-object v3
    invoke-static {v0, v2, v3}, Lfixture/WordOrder;->finish(II[I)I
    move-result v0
    return v0
.end method

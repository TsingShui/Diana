.class public final Lfixture/OrRoutes;
.super Ljava/lang/Object;

.field public static trace:I

.method public static mark(I)V
    .locals 1
    sget v0, Lfixture/OrRoutes;->trace:I
    mul-int/lit8 v0, v0, 0x1f
    add-int/2addr v0, p0
    sput v0, Lfixture/OrRoutes;->trace:I
    return-void
.end method

.method public static probe(III)Z
    .locals 2
    add-int/lit8 v0, p0, 0xa
    invoke-static {v0}, Lfixture/OrRoutes;->mark(I)V
    if-eq p0, p2, :fault
    const/4 v0, 0x1
    shl-int/2addr v0, p0
    and-int/2addr v0, p1
    if-eqz v0, :no
    const/4 v0, 0x1
    return v0
    :no
    const/4 v0, 0x0
    return v0
    :fault
    new-instance v0, Ljava/lang/IllegalStateException;
    const-string v1, "probe"
    invoke-direct {v0, v1}, Ljava/lang/IllegalStateException;-><init>(Ljava/lang/String;)V
    throw v0
.end method

# Three effectful tests, a two-block selected route and a separate selected route.
.method public static ordered(II)I
    .locals 2
    const/4 v1, 0x0
    invoke-static {v1, p0, p1}, Lfixture/OrRoutes;->probe(III)Z
    move-result v0
    if-nez v0, :yes
    const/4 v1, 0x1
    invoke-static {v1, p0, p1}, Lfixture/OrRoutes;->probe(III)Z
    move-result v0
    if-eqz v0, :third
    :second_route
    goto :second_route_tail
    :second_route_tail
    goto :yes
    :third
    const/4 v1, 0x2
    invoke-static {v1, p0, p1}, Lfixture/OrRoutes;->probe(III)Z
    move-result v0
    if-eqz v0, :no
    :third_route
    goto :yes
    :no
    const/16 v1, 0x32
    invoke-static {v1}, Lfixture/OrRoutes;->mark(I)V
    const/16 v0, 0x9
    goto :join
    :yes
    const/16 v1, 0x28
    invoke-static {v1}, Lfixture/OrRoutes;->mark(I)V
    const/4 v0, 0x7
    :join
    const/16 v1, 0x3c
    invoke-static {v1}, Lfixture/OrRoutes;->mark(I)V
    return v0
.end method

# A selected path with an observable call must not become transparent.
.method public static effectful(I)I
    .locals 3
    const/4 v2, -0x1
    const/4 v1, 0x0
    invoke-static {v1, p0, v2}, Lfixture/OrRoutes;->probe(III)Z
    move-result v0
    if-nez v0, :yes
    const/4 v1, 0x1
    invoke-static {v1, p0, v2}, Lfixture/OrRoutes;->probe(III)Z
    move-result v0
    if-eqz v0, :no
    const/16 v1, 0x1e
    invoke-static {v1}, Lfixture/OrRoutes;->mark(I)V
    goto :yes
    :no
    const/16 v1, 0x32
    invoke-static {v1}, Lfixture/OrRoutes;->mark(I)V
    const/16 v0, 0x9
    return v0
    :yes
    const/16 v1, 0x28
    invoke-static {v1}, Lfixture/OrRoutes;->mark(I)V
    const/4 v0, 0x7
    return v0
.end method

# A later test prelude has work that cannot be spelled as its boolean operand.
.method public static blocked(I)I
    .locals 3
    const/4 v2, -0x1
    const/4 v1, 0x0
    invoke-static {v1, p0, v2}, Lfixture/OrRoutes;->probe(III)Z
    move-result v0
    if-nez v0, :yes
    const/16 v1, 0x19
    invoke-static {v1}, Lfixture/OrRoutes;->mark(I)V
    const/4 v1, 0x1
    invoke-static {v1, p0, v2}, Lfixture/OrRoutes;->probe(III)Z
    move-result v0
    if-eqz v0, :no
    goto :yes
    :no
    const/16 v1, 0x32
    invoke-static {v1}, Lfixture/OrRoutes;->mark(I)V
    const/16 v0, 0x9
    goto :join
    :yes
    const/16 v1, 0x28
    invoke-static {v1}, Lfixture/OrRoutes;->mark(I)V
    const/4 v0, 0x7
    :join
    return v0
.end method

# The empty goto carries the replacement value into an actual shared-result Phi.
.method public static copied(III)I
    .locals 1
    if-nez p0, :yes
    move p1, p2
    if-eqz p2, :no
    goto :yes
    :no
    const/4 v0, -0x1
    return v0
    :yes
    return p1
.end method

# The skip edge enters the selected route without selecting either probe edge.
.method public static foreign(II)I
    .locals 3
    if-nez p1, :route
    const/4 v2, -0x1
    const/4 v1, 0x0
    invoke-static {v1, p0, v2}, Lfixture/OrRoutes;->probe(III)Z
    move-result v0
    if-nez v0, :yes
    const/4 v1, 0x1
    invoke-static {v1, p0, v2}, Lfixture/OrRoutes;->probe(III)Z
    move-result v0
    if-eqz v0, :no
    :route
    goto :yes
    :no
    const/16 v1, 0x32
    invoke-static {v1}, Lfixture/OrRoutes;->mark(I)V
    const/16 v0, 0x9
    return v0
    :yes
    const/16 v1, 0x28
    invoke-static {v1}, Lfixture/OrRoutes;->mark(I)V
    const/4 v0, 0x7
    return v0
.end method

# A selected empty-goto cycle diverges; it does not reach the shared return.
.method public static cyclic(I)I
    .locals 3
    const/4 v2, -0x1
    const/4 v1, 0x0
    invoke-static {v1, p0, v2}, Lfixture/OrRoutes;->probe(III)Z
    move-result v0
    if-nez v0, :yes
    const/4 v1, 0x1
    invoke-static {v1, p0, v2}, Lfixture/OrRoutes;->probe(III)Z
    move-result v0
    if-eqz v0, :no
    :cycle_a
    goto :cycle_b
    :cycle_b
    goto :cycle_a
    :no
    const/16 v0, 0x9
    return v0
    :yes
    const/4 v0, 0x7
    return v0
.end method

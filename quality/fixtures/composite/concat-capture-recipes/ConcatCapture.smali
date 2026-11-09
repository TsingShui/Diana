.class public final Lfixture/ConcatCapture;
.super Ljava/lang/Object;
.field public static trace:I

.method public static mark(I)V
    .locals 1
    sget v0, Lfixture/ConcatCapture;->trace:I
    mul-int/lit8 v0, v0, 0x1f
    add-int/2addr v0, p0
    sput v0, Lfixture/ConcatCapture;->trace:I
    return-void
.end method

.method public static read(II)Ljava/lang/String;
    .locals 2
    add-int/lit8 v0, p0, 0xa
    invoke-static {v0}, Lfixture/ConcatCapture;->mark(I)V
    if-eq p0, p1, :fault
    if-eqz p0, :left
    const-string v0, "right"
    return-object v0
    :left
    const-string v0, "left"
    return-object v0
    :fault
    new-instance v0, Ljava/lang/IllegalStateException;
    const-string v1, "read"
    invoke-direct {v0, v1}, Ljava/lang/IllegalStateException;-><init>(Ljava/lang/String;)V
    throw v0
.end method

.method public static numeric(IICZJ)Ljava/lang/String;
    .locals 1
    invoke-custom/range {p0 .. p5}, numeric("mix", (IICZJ)Ljava/lang/String;)@Ljava/lang/invoke/StringConcatFactory;->makeConcat(Ljava/lang/invoke/MethodHandles$Lookup;Ljava/lang/String;Ljava/lang/invoke/MethodType;)Ljava/lang/invoke/CallSite;
    move-result-object v0
    return-object v0
.end method

.method public static recipe(Ljava/lang/String;IC)Ljava/lang/String;
    .locals 1
    invoke-custom {p0, p1, p2}, recipe("join", (Ljava/lang/String;IC)Ljava/lang/String;, "\u0002[\u0001:\u0001]\u0001\u0002", "tag", "\u0001")@Ljava/lang/invoke/StringConcatFactory;->makeConcatWithConstants(Ljava/lang/invoke/MethodHandles$Lookup;Ljava/lang/String;Ljava/lang/invoke/MethodType;Ljava/lang/String;[Ljava/lang/Object;)Ljava/lang/invoke/CallSite;
    move-result-object v0
    return-object v0
.end method

.method public static constants(Ljava/lang/String;)Ljava/lang/String;
    .locals 1
    invoke-custom {p0}, constants("join", (Ljava/lang/String;)Ljava/lang/String;, "\u0001\u0002\u0002\u0002", 7L, true, 'A')@Ljava/lang/invoke/StringConcatFactory;->makeConcatWithConstants(Ljava/lang/invoke/MethodHandles$Lookup;Ljava/lang/String;Ljava/lang/invoke/MethodType;Ljava/lang/String;[Ljava/lang/Object;)Ljava/lang/invoke/CallSite;
    move-result-object v0
    return-object v0
.end method

.method public static floating(FD)Ljava/lang/String;
    .locals 1
    invoke-custom/range {p0 .. p2}, floating("join", (FD)Ljava/lang/String;, "\u0001:\u0001")@Ljava/lang/invoke/StringConcatFactory;->makeConcatWithConstants(Ljava/lang/invoke/MethodHandles$Lookup;Ljava/lang/String;Ljava/lang/invoke/MethodType;Ljava/lang/String;[Ljava/lang/Object;)Ljava/lang/invoke/CallSite;
    move-result-object v0
    return-object v0
.end method

.method public static single(Ljava/lang/String;)Ljava/lang/String;
    .locals 1
    invoke-custom {p0}, single("join", (Ljava/lang/String;)Ljava/lang/String;)@Ljava/lang/invoke/StringConcatFactory;->makeConcat(Ljava/lang/invoke/MethodHandles$Lookup;Ljava/lang/String;Ljava/lang/invoke/MethodType;)Ljava/lang/invoke/CallSite;
    move-result-object v0
    return-object v0
.end method

.method public static moved(CZ)Ljava/lang/String;
    .locals 2
    move v0, p0
    move v1, p1
    invoke-custom {v0, v1}, moved("join", (CZ)Ljava/lang/String;)@Ljava/lang/invoke/StringConcatFactory;->makeConcat(Ljava/lang/invoke/MethodHandles$Lookup;Ljava/lang/String;Ljava/lang/invoke/MethodType;)Ljava/lang/invoke/CallSite;
    move-result-object v0
    return-object v0
.end method

.method public static charAsInt(C)Ljava/lang/String;
    .locals 1
    invoke-custom {p0}, charAsInt("join", (I)Ljava/lang/String;)@Ljava/lang/invoke/StringConcatFactory;->makeConcat(Ljava/lang/invoke/MethodHandles$Lookup;Ljava/lang/String;Ljava/lang/invoke/MethodType;)Ljava/lang/invoke/CallSite;
    move-result-object v0
    return-object v0
.end method

# The two helpers execute in 0/1 order, but concat consumes their values in 1/0 order.
.method public static effectful(I)Ljava/lang/String;
    .locals 3
    const/4 v0, 0x0
    invoke-static {v0, p0}, Lfixture/ConcatCapture;->read(II)Ljava/lang/String;
    move-result-object v0
    const/4 v1, 0x1
    invoke-static {v1, p0}, Lfixture/ConcatCapture;->read(II)Ljava/lang/String;
    move-result-object v1
    invoke-custom {v1, v0}, effectful("join", (Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;, "\u0001:\u0001")@Ljava/lang/invoke/StringConcatFactory;->makeConcatWithConstants(Ljava/lang/invoke/MethodHandles$Lookup;Ljava/lang/String;Ljava/lang/invoke/MethodType;Ljava/lang/String;[Ljava/lang/Object;)Ljava/lang/invoke/CallSite;
    move-result-object v2
    return-object v2
.end method

.method public static loop(II)Ljava/lang/String;
    .locals 4
    const/4 v0, 0x0
    const-string v1, ""
    :head
    if-ge v0, p0, :done
    :body
    invoke-static {v0, p1}, Lfixture/ConcatCapture;->read(II)Ljava/lang/String;
    move-result-object v2
    invoke-custom {v1, v0, v2}, loop("join", (Ljava/lang/String;ILjava/lang/String;)Ljava/lang/String;, "\u0001<\u0001:\u0001>")@Ljava/lang/invoke/StringConcatFactory;->makeConcatWithConstants(Ljava/lang/invoke/MethodHandles$Lookup;Ljava/lang/String;Ljava/lang/invoke/MethodType;Ljava/lang/String;[Ljava/lang/Object;)Ljava/lang/invoke/CallSite;
    move-result-object v1
    :body_end
    add-int/lit8 v0, v0, 0x1
    goto :head
    :handler
    move-exception v3
    const-string v1, "caught"
    goto :done
    :done
    return-object v1
    .catch Ljava/lang/IllegalStateException; {:body .. :body_end} :handler
.end method

.method public static object(Ljava/lang/Object;I)Ljava/lang/String;
    .locals 1
    invoke-custom {p0, p1}, object("join", (Ljava/lang/Object;I)Ljava/lang/String;)@Ljava/lang/invoke/StringConcatFactory;->makeConcat(Ljava/lang/invoke/MethodHandles$Lookup;Ljava/lang/String;Ljava/lang/invoke/MethodType;)Ljava/lang/invoke/CallSite;
    move-result-object v0
    return-object v0
.end method

.method public static nonfiniteLiteral()Ljava/lang/String;
    .locals 1
    const v0, 0x7f800000
    invoke-custom {v0}, infinity("join", (F)Ljava/lang/String;)@Ljava/lang/invoke/StringConcatFactory;->makeConcat(Ljava/lang/invoke/MethodHandles$Lookup;Ljava/lang/String;Ljava/lang/invoke/MethodType;)Ljava/lang/invoke/CallSite;
    move-result-object v0
    return-object v0
.end method

.method public static badRecipe(Ljava/lang/String;I)Ljava/lang/String;
    .locals 1
    invoke-custom {p0, p1}, badRecipe("join", (Ljava/lang/String;I)Ljava/lang/String;, "\u0001")@Ljava/lang/invoke/StringConcatFactory;->makeConcatWithConstants(Ljava/lang/invoke/MethodHandles$Lookup;Ljava/lang/String;Ljava/lang/invoke/MethodType;Ljava/lang/String;[Ljava/lang/Object;)Ljava/lang/invoke/CallSite;
    move-result-object v0
    return-object v0
.end method

.method public static badBootstrap(Ljava/lang/String;)Ljava/lang/String;
    .locals 1
    invoke-custom {p0}, badBootstrap("join", (Ljava/lang/String;)Ljava/lang/String;)@Ljava/lang/invoke/StringConcatFactory;->makeConcat(Ljava/lang/invoke/MethodHandles$Lookup;Ljava/lang/String;Ljava/lang/invoke/MethodType;)Ljava/lang/String;
    move-result-object v0
    return-object v0
.end method

.method public static unknownFactory(Ljava/lang/String;I)Ljava/lang/String;
    .locals 1
    invoke-custom {p0, p1}, unknown("join", (Ljava/lang/String;I)Ljava/lang/String;)@Lfixture/FactoryAlias;->makeConcat(Ljava/lang/invoke/MethodHandles$Lookup;Ljava/lang/String;Ljava/lang/invoke/MethodType;)Ljava/lang/invoke/CallSite;
    move-result-object v0
    return-object v0
.end method

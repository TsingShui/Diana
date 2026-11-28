.class public interface abstract annotation Lfixture/DexDefaults;
.super Ljava/lang/Object;
.implements Ljava/lang/annotation/Annotation;

.annotation system Ldalvik/annotation/AnnotationDefault;
    value = .subannotation Lfixture/DexDefaults;
        classChoice = [Ljava/lang/String;
        count = -0x21
        empty = {}
        enabled = true
        label = .subannotation Lfixture/DexLabel;
            rank = 0x4
            value = "west"
        .end subannotation
        labels = {
            .subannotation Lfixture/DexLabel;
                rank = -0x1
                value = "north"
            .end subannotation,
            .subannotation Lfixture/DexLabel;
                rank = 0x5
                value = "east"
            .end subannotation
        }
        name = "row\n\"blue\""
        policy = .enum Ljava/lang/annotation/RetentionPolicy;->RUNTIME:Ljava/lang/annotation/RetentionPolicy;
        scores = {0x3, -0x2, 0x3}
    .end subannotation
.end annotation

.method public abstract classChoice()Ljava/lang/Class;
.end method

.method public abstract count()I
.end method

.method public abstract empty()[I
.end method

.method public abstract enabled()Z
.end method

.method public abstract label()Lfixture/DexLabel;
.end method

.method public abstract labels()[Lfixture/DexLabel;
.end method

.method public abstract name()Ljava/lang/String;
.end method

.method public abstract policy()Ljava/lang/annotation/RetentionPolicy;
.end method

.method public abstract required()I
.end method

.method public abstract scores()[I
.end method

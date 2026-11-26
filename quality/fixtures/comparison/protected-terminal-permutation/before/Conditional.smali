.class public Lguard/Conditional;
.super Ljava/lang/Object;

.method public static run(Ljava/lang/RuntimeException;I)Ljava/lang/Object;
 .registers 3
 const-string v0, "audit-shared-conditional-anchor"
 const/4 v0, 0
 :start
 if-eqz p1, :hit
 goto :done
 :hit
 throw p0
 :end
 const/4 p1, 7
 goto :done
 :done
 return-object p0
 :handler
 move-exception p0
 const/4 p0, 0
 return-object p0
 .catch Ljava/lang/RuntimeException; {:start .. :end} :handler
.end method

package demo;
public class Probe {
 public static String rows() {
  var a=net.fabricmc.fabric.api.attachment.v1.AttachmentSyncPredicate.all(); var a2=net.fabricmc.fabric.api.attachment.v1.AttachmentSyncPredicate.all();
  var t=net.fabricmc.fabric.api.attachment.v1.AttachmentSyncPredicate.targetOnly(); var t2=net.fabricmc.fabric.api.attachment.v1.AttachmentSyncPredicate.targetOnly();
  var b=net.fabricmc.fabric.api.attachment.v1.AttachmentSyncPredicate.allButTarget(); var b2=net.fabricmc.fabric.api.attachment.v1.AttachmentSyncPredicate.allButTarget();
  return (a==a2)+"|"+(t==t2)+"|"+(b==b2);
 }
}

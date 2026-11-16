import net.minecraft.SharedConstants;
import net.minecraft.server.Bootstrap;
import net.minecraft.network.syncher.*;
import net.minecraft.world.entity.*;
public class Probe {
 static final EntityDataSerializer<Integer> S = new EntityDataSerializer<Integer>() { public net.minecraft.network.codec.StreamCodec<? super net.minecraft.network.RegistryFriendlyByteBuf,Integer> codec(){return null;} public Integer copy(Integer x){return x;} };
 static class Base implements SyncedDataHolder { public void onSyncedDataUpdated(EntityDataAccessor<?> a) {} public void onSyncedDataUpdated(java.util.List<SynchedEntityData.DataValue<?>> a) {} }
 static class Sub extends Base { }
 public static void main(String[] x) {
  SharedConstants.tryDetectVersion(); Bootstrap.bootStrap();
  for (MobCategory c : MobCategory.values()) System.out.println("cat="+c.name()+","+c.getName()+","+c.getDebugAbbreviation()+","+c.getMaxInstancesPerChunk()+","+c.isFriendly()+","+c.isPersistent()+","+c.getDespawnDistance()+","+c.getNoDespawnDistance()+","+c.toString());
  System.out.println("serial="+(EntityDataSerializers.BYTE==EntityDataSerializers.BYTE)+","+EntityDataSerializers.BYTE.equals(EntityDataSerializers.INT));
  System.out.println("base="+SynchedEntityData.defineId(Base.class,S).id());
  System.out.println("sub="+SynchedEntityData.defineId(Sub.class,S).id());
  System.out.println("base2="+SynchedEntityData.defineId(Base.class,S).id());
  class Sibling implements SyncedDataHolder { public void onSyncedDataUpdated(EntityDataAccessor<?> a) {} public void onSyncedDataUpdated(java.util.List<SynchedEntityData.DataValue<?>> a) {} }
  System.out.println("sibling="+SynchedEntityData.defineId(Sibling.class,S).id());
  try { for(int i=0;i<256;i++) SynchedEntityData.defineId(Overflow.class,S); SynchedEntityData.defineId(Overflow.class,S); } catch (IllegalArgumentException e) { System.out.println("overflow="+e.getMessage()); }
 }
 static class Overflow implements SyncedDataHolder { public void onSyncedDataUpdated(EntityDataAccessor<?> a) {} public void onSyncedDataUpdated(java.util.List<SynchedEntityData.DataValue<?>> a) {} }
}

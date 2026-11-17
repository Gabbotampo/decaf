package demo;
import net.minecraft.SharedConstants;
import net.minecraft.WorldVersion;
public final class Probe {
 public static String rows() {
  WorldVersion v=SharedConstants.getCurrentVersion();
  StringBuilder o=new StringBuilder();
  o.append("name|").append(v.name()).append('\n');
  o.append("id|").append(v.id()).append('\n');
  o.append("protocol|").append(v.protocolVersion()).append('\n');
  o.append("stable|").append(v.stable()).append('\n');
  o.append("data|").append(v.dataVersion().version()).append('|').append(v.dataVersion().series()).append('\n');
  o.append("pack-server|").append(v.packVersion(net.minecraft.server.packs.PackType.SERVER_DATA).major()).append('.').append(v.packVersion(net.minecraft.server.packs.PackType.SERVER_DATA).minor()).append('\n');
  o.append("pack-client|").append(v.packVersion(net.minecraft.server.packs.PackType.CLIENT_RESOURCES).major()).append('.').append(v.packVersion(net.minecraft.server.packs.PackType.CLIENT_RESOURCES).minor()).append('\n');
  o.append("same|").append(SharedConstants.getCurrentVersion() == v).append('\n');
  o.append("data-same|").append(v.dataVersion() == v.dataVersion()).append('\n');
  o.append("pack-same|").append(v.packVersion(net.minecraft.server.packs.PackType.SERVER_DATA) == v.packVersion(net.minecraft.server.packs.PackType.SERVER_DATA)).append('\n');
  o.append("values-server|").append(v.packVersion(net.minecraft.server.packs.PackType.values()[1]).major()).append('\n');
  o.append("values-client|").append(v.packVersion(net.minecraft.server.packs.PackType.values()[0]).major()).append('\n');
  
  return o.toString();
 }
}

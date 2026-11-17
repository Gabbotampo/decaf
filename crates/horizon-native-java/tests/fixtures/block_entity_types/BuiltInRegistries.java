package net.minecraft.core.registries;
public final class BuiltInRegistries {
  public static final Registry BLOCK_ENTITY_TYPE = new Registry();
  public static final class Registry { public Object getKey(Object value) { return value; } }
}

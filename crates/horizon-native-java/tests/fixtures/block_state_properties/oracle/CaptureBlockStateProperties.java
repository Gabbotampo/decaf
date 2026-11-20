package oracle;

import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.Comparator;
import java.util.List;

/** Captures BlockStateProperties from the real 26.3 server with JDK 25. */
public final class CaptureBlockStateProperties {
  public static void main(String[] args) throws Exception {
    net.minecraft.SharedConstants.tryDetectVersion();
    net.minecraft.server.Bootstrap.bootStrap();
    Class<?> owner = Class.forName("net.minecraft.world.level.block.state.properties.BlockStateProperties");
    Method getName = Class.forName("net.minecraft.world.level.block.state.properties.Property")
        .getMethod("getName");
    Class<?> propertyClass = Class.forName("net.minecraft.world.level.block.state.properties.Property");
    Method values = propertyClass.getMethod("getPossibleValues");
    // Property.getName(T): the value's serialized name, as block states and blockstate JSON spell it.
    Method valueName = propertyClass.getMethod("getName", Comparable.class);
    Field[] fields = owner.getFields();
    java.util.Arrays.sort(fields, Comparator.comparing(Field::getName));
    System.out.println("# field|name|type|values|declared field type (the getstatic descriptor's class)");
    for (Field field : fields) {
      if (!Modifier.isStatic(field.getModifiers()) || !field.getType().getName().contains("Property")) continue;
      Object property = field.get(null);
      String name = (String) getName.invoke(property);
      @SuppressWarnings("unchecked") List<Object> possible = (List<Object>) values.invoke(property);
      String type = property.getClass().getSimpleName().replace("Property", "").toLowerCase(java.util.Locale.ROOT);
      if (type.equals("bool")) type = "boolean";
      List<String> names = new java.util.ArrayList<>();
      for (Object value : possible) names.add((String) valueName.invoke(property, value));
      System.out.println(field.getName() + "|" + name + "|" + type + "|" + names + "|" +
          field.getType().getName().replace('.', '/'));
    }
  }
}

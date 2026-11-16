package demo;

import com.google.gson.*;
import com.google.gson.annotations.SerializedName;
import java.util.*;

/** Java 25 oracle for gson 2.14.0 reflective fromJson. */
final class Probe {
  static final class Nested { String label; }
  enum Mode { FIRST, @SerializedName("wire-second") SECOND }
  static final class Config {
    String name; int size; boolean enabled; List<Nested> items; Map<String,Integer> nums;
    int[] arr; Mode mode; @SerializedName(value="wire_name", alternate={"old_name"}) String alias;
    String ctorDefault = "field";
    Config() { ctorDefault = "constructor"; }
  }
  static final class UnsafeConfig {
    String value;
    private UnsafeConfig(String ignored) { value = "constructor"; }
  }
  abstract static class Abs { int x; }
  interface Intf {}
  record Rec(int x, String s) { public Rec { s = s + "|ctor"; } }
  static final class NestedToken extends com.google.gson.reflect.TypeToken<List<Nested>> {}
  static void show(String label, Object value) { System.out.println(label+"="+String.valueOf(value)); }
  public static void main(String[] args) {
    Gson gson = new Gson();
    Config config = gson.fromJson("{\"name\":123,\"size\":\"7\",\"enabled\":true,\"items\":[{\"label\":\"inner\"}],\"nums\":{\"n\":2},\"arr\":[3,4],\"mode\":\"wire-second\",\"old_name\":\"alias\",\"unknown\":9}", Config.class);
    show("config", config.name+"|"+config.size+"|"+config.enabled+"|"+config.items.get(0).label+"|"+config.nums.get("n")+"|"+config.arr[1]+"|"+config.mode+"|"+config.alias+"|"+config.ctorDefault);
    Config defaults = gson.fromJson("{\"unknown\":1}", Config.class);
    show("defaults", String.valueOf(defaults.name)+"|"+defaults.size+"|"+defaults.enabled+"|"+defaults.ctorDefault);
    Config booleans = gson.fromJson("{\"enabled\":true}", Config.class);
    show("booleanFieldBoxing", Boolean.valueOf(booleans.enabled).toString());
    UnsafeConfig unsafe = gson.fromJson("{\"value\":\"json\"}", UnsafeConfig.class);
    show("unsafe", unsafe.value);
    List<Nested> values = gson.fromJson("[{\"label\":\"token\"}]", new NestedToken());
    show("token", values.get(0).label);
    JsonElement element = JsonParser.parseString("{\"name\":\"element\"}");
    show("element", gson.fromJson(element, Config.class).name);
    try { gson.fromJson("{\"x\":1}", Abs.class); show("abstractCase", "object"); }
    catch (Throwable t) { show("abstractCase", t.getClass().getName()+"|"+t.getMessage()); }
    try { gson.fromJson("{}", Intf.class); show("interfaceCase", "object"); }
    catch (Throwable t) { show("interfaceCase", t.getClass().getName()+"|"+t.getMessage()); }
    Rec record = gson.fromJson("{\"x\":4,\"s\":\"ok\"}", Rec.class);
    show("recordCase", record.x()+"|"+record.s());
  }
}

public final class GsonFromJsonOracle {
  public static void main(String[] args) { Probe.main(args); }
}

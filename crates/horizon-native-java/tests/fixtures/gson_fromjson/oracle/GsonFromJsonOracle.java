import com.google.gson.*;
import com.google.gson.annotations.SerializedName;
import java.util.*;

/** Java 25 oracle for gson 2.14.0 reflective fromJson. */
public final class GsonFromJsonOracle {
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
  static void show(String label, Object value) { System.out.println(label+"="+String.valueOf(value)); }
  static void error(String label, Runnable body) {
    try { body.run(); show(label, "OK"); }
    catch (Throwable t) { show(label, t.getClass().getName()+"|"+t.getMessage()+"|cause="+String.valueOf(t.getCause())); }
  }
  public static void main(String[] args) {
    Gson gson = new Gson();
    Config config = gson.fromJson("{\"name\":123,\"size\":\"7\",\"enabled\":true,\"items\":[{\"label\":\"inner\"}],\"nums\":{\"n\":2},\"arr\":[3,4],\"mode\":\"wire-second\",\"old_name\":\"alias\",\"unknown\":9}", Config.class);
    show("config", config.name+"|"+config.size+"|"+config.enabled+"|"+config.items.get(0).label+"|"+config.nums.get("n")+"|"+config.arr[1]+"|"+config.mode+"|"+config.alias+"|"+config.ctorDefault);
    Config defaults = gson.fromJson("{\"unknown\":1}", Config.class);
    show("defaults", String.valueOf(defaults.name)+"|"+defaults.size+"|"+defaults.enabled+"|"+defaults.ctorDefault);
    UnsafeConfig unsafe = gson.fromJson("{\"value\":\"json\"}", UnsafeConfig.class);
    show("unsafe", unsafe.value);
    JsonPrimitive number = gson.fromJson("1.2300", JsonPrimitive.class);
    show("number", number.getAsNumber().getClass().getName()+"|"+number.getAsNumber()+"|"+number.getAsString());
    error("malformed", () -> gson.fromJson("{bad", Config.class));
    error("trailing", () -> gson.fromJson("{} {}", Config.class));
  }
}

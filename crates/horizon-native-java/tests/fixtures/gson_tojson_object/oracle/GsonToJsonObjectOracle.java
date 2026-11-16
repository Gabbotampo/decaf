import com.google.gson.*;
import java.util.*;

public final class GsonToJsonObjectOracle {
  static final class NestedConfig {
    String label; int amount;
    NestedConfig(String label, int amount) { this.label = label; this.amount = amount; }
  }
  static final class Config {
    String name; int count; boolean enabled; double ratio; String nullable;
    List<String> tags; Map<String,Integer> values; NestedConfig nested;
    Config() {
      name = "<cfg>"; count = -9; enabled = true; ratio = -98.0; nullable = null;
      tags = new ArrayList<>(); tags.add("<a>"); tags.add("b");
      values = new LinkedHashMap<>(); values.put("x", 96); values.put("y", 17);
      nested = new NestedConfig("n0", 47);
    }
  }
  public static void main(String[] args) {
    Config value = new Config();
    Gson compact = new Gson();
    Gson pretty = new GsonBuilder().setPrettyPrinting().create();
    Gson nulls = new GsonBuilder().setPrettyPrinting().serializeNulls().create();
    Gson safe = new GsonBuilder().setPrettyPrinting().serializeNulls().disableHtmlEscaping().create();
    String output = compact.toJson(value)
        + "|P=" + pretty.toJson(value).replace("\n", "\\n")
        + "|N=" + nulls.toJson(value).replace("\n", "\\n")
        + "|H=" + safe.toJson(value).replace("\n", "\\n");
    System.out.println(output);
  }
}

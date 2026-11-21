import com.google.gson.JsonElement;
import com.mojang.serialization.DataResult;
import com.mojang.serialization.JsonOps;
import net.minecraft.ChatFormatting;
import net.minecraft.SharedConstants;
import net.minecraft.network.chat.Component;
import net.minecraft.network.chat.ComponentSerialization;
import net.minecraft.server.Bootstrap;

/**
 * Captures Minecraft 26.3's real {@link ComponentSerialization#CODEC} + {@link JsonOps#INSTANCE}
 * JSON output on OpenJDK 25 for the cases exercised by {@code fixture_runes_item_tooltip.rs}.
 */
public final class TooltipComponentsOracle {
  private static String encode(Component component) {
    DataResult<JsonElement> result =
        ComponentSerialization.CODEC.encodeStart(JsonOps.INSTANCE, component);
    return result
        .result()
        .map(JsonElement::toString)
        .orElseThrow(
            () ->
                new IllegalStateException(
                    result.error().map(DataResult.Error::message).orElse("unknown codec error")));
  }

  public static void main(String[] args) {
    SharedConstants.tryDetectVersion();
    Bootstrap.bootStrap();
    Object[][] cases = {
      {"literal_plain", Component.literal("hello")},
      {"literal_empty", Component.literal("")},
      {
        "literal_styled",
        Component.literal("styled").withStyle(ChatFormatting.GRAY, ChatFormatting.ITALIC)
      },
      {"literal_hex_color", Component.literal("hex").withColor(0xABCDEF)},
      {"translatable_bare", Component.translatable("block.runes.crafting_altar.hint")},
      {
        "translatable_styled",
        Component.translatable("block.runes.crafting_altar.hint")
            .withStyle(ChatFormatting.GRAY, ChatFormatting.ITALIC)
      },
      {"translatable_str_arg", Component.translatable("demo.tip", "arg")},
      {"translatable_int_arg", Component.translatable("demo.block", 7)},
      {
        "translatable_prim_args",
        Component.translatable("demo.prim", true, 42L, 1.5, 2.5f)
      },
      {
        "translatable_literal_arg",
        Component.translatable("demo.lit", Component.literal("foo"))
      },
      {
        "translatable_styled_arg",
        Component.translatable("demo.styled", Component.literal("foo").withStyle(ChatFormatting.RED))
      },
      {
        "translatable_nested_arg",
        Component.translatable(
            "demo.nested", Component.translatable("inner", Component.literal("bar")))
      },
      {
        "translatable_fallback_full",
        Component.translatableWithFallback("demo.fb", "fb", "a", Component.literal("b"))
            .withStyle(ChatFormatting.ITALIC, ChatFormatting.GRAY)
            .append(Component.literal("e"))
      },
      {
        "style_then_append",
        Component.literal("a").withStyle(ChatFormatting.GRAY).append(Component.literal("b"))
      },
      {
        "append_then_style",
        Component.literal("a").append(Component.literal("b")).withStyle(ChatFormatting.GRAY)
      },
      {
        "style_order_igb",
        Component.literal("x")
            .withStyle(ChatFormatting.ITALIC, ChatFormatting.GRAY, ChatFormatting.BOLD)
      },
      {
        "style_all_decorations",
        Component.literal("x")
            .withStyle(
                ChatFormatting.OBFUSCATED,
                ChatFormatting.STRIKETHROUGH,
                ChatFormatting.UNDERLINE,
                ChatFormatting.ITALIC,
                ChatFormatting.BOLD,
                ChatFormatting.RED)
      },
      {
        "style_reset_varargs",
        Component.literal("x")
            .withStyle(
                ChatFormatting.RED,
                ChatFormatting.ITALIC,
                ChatFormatting.RESET,
                ChatFormatting.BOLD)
      },
      {
        "style_reset_chained",
        Component.literal("x")
            .withStyle(ChatFormatting.RED, ChatFormatting.ITALIC)
            .withStyle(ChatFormatting.RESET)
            .withStyle(ChatFormatting.BOLD)
      },
      {
        "nested_extras",
        Component.literal("a")
            .append(Component.literal("b").append(Component.literal("c")))
            .append(Component.literal("d").withStyle(ChatFormatting.GOLD))
      },
      {"translatable_empty_args", Component.translatable("demo.noargs", new Object[0])},
    };
    for (Object[] row : cases) {
      System.out.println("CASE:" + row[0] + "=" + encode((Component) row[1]));
    }
  }
}

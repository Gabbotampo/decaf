import com.google.gson.JsonElement;
import com.mojang.serialization.DataResult;
import com.mojang.serialization.JsonOps;
import java.util.ArrayList;
import java.util.List;
import net.minecraft.ChatFormatting;
import net.minecraft.SharedConstants;
import net.minecraft.network.chat.Component;
import net.minecraft.network.chat.ComponentSerialization;
import net.minecraft.server.Bootstrap;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.TooltipFlag;
import net.minecraft.world.level.Level;

/**
 * Captures Minecraft 26.3's real {@link ComponentSerialization#CODEC} + {@link JsonOps#INSTANCE},
 * {@link Component#translatableEscape}, {@link TooltipFlag.Default}, and
 * {@link Item.TooltipContext#EMPTY} outputs on OpenJDK 25 for {@code fixture_runes_item_tooltip.rs}.
 */
public final class TooltipComponentsOracle {
  private static final class CustomLabel {
    @Override
    public String toString() {
      return "custom-str";
    }
  }

  private record Pair(int x, String y) {}

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
    Object[][] componentCases = {
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
      {"escape_null", Component.translatableEscape("demo.esc", new Object[] {null})},
      {"escape_char", Component.translatableEscape("demo.esc", Character.valueOf('Z'))},
      {"escape_custom", Component.translatableEscape("demo.esc", new CustomLabel())},
      {"escape_record", Component.translatableEscape("demo.esc", new Pair(3, "hi"))},
      {
        "escape_list",
        Component.translatableEscape("demo.esc", new ArrayList<>(List.of("a", "b")))
      },
      {"escape_formatting", Component.translatableEscape("demo.esc", ChatFormatting.RED)},
      {
        "escape_component",
        Component.translatableEscape(
            "demo.esc", Component.literal("lit").withStyle(ChatFormatting.GOLD))
      },
      {"escape_tooltip_flag", Component.translatableEscape("demo.esc", TooltipFlag.NORMAL)},
      {"translatable_double_nan", Component.translatable("demo.nan", Double.NaN)},
      {"translatable_double_pos_inf", Component.translatable("demo.inf", Double.POSITIVE_INFINITY)},
      {"translatable_double_neg_inf", Component.translatable("demo.ninf", Double.NEGATIVE_INFINITY)},
      {"translatable_float_nan", Component.translatable("demo.fnan", Float.NaN)},
      {"translatable_float_pos_inf", Component.translatable("demo.finf", Float.POSITIVE_INFINITY)},
      {"translatable_float_neg_inf", Component.translatable("demo.fninf", Float.NEGATIVE_INFINITY)},
      {
        "escape_nonfinite",
        Component.translatableEscape(
            "demo.esc_nf", Double.NaN, Float.POSITIVE_INFINITY, Float.NEGATIVE_INFINITY)
      },
      {"translatable_float_decimal", Component.translatable("demo.fdec", 1.234567f)},
      {"translatable_float_third", Component.translatable("demo.fthird", 1.0f / 3.0f)},
      {"translatable_float_neg_zero", Component.translatable("demo.fnz", -0.0f)},
      {"translatable_float_sci", Component.translatable("demo.fsci", 1e8f)},
      {"translatable_double_sci", Component.translatable("demo.dsci", 1e10)},
      {"translatable_double_neg_zero", Component.translatable("demo.dnz", -0.0)},
      {"translatable_double_small", Component.translatable("demo.dsmall", 1e-4)},
    };
    for (Object[] row : componentCases) {
      System.out.println("CASE:" + row[0] + "=" + encode((Component) row[1]));
    }

    TooltipFlag.Default normal = TooltipFlag.NORMAL;
    TooltipFlag.Default advanced = TooltipFlag.ADVANCED;
    TooltipFlag.Default normalCreative = normal.asCreative();
    TooltipFlag.Default advancedCreative = advanced.asCreative();
    System.out.println(
        "CASE:flag_default_strings="
            + normal
            + "|"
            + advanced
            + "|"
            + normalCreative
            + "|"
            + advancedCreative);
    System.out.println(
        "CASE:flag_default_hashes="
            + normal.hashCode()
            + ","
            + advanced.hashCode()
            + ","
            + normalCreative.hashCode()
            + ","
            + advancedCreative.hashCode());
    System.out.println(
        "CASE:flag_default_accessors="
            + normal.advanced()
            + ","
            + normal.creative()
            + ","
            + normal.isAdvanced()
            + ","
            + normal.isCreative()
            + "|"
            + advancedCreative.advanced()
            + ","
            + advancedCreative.creative()
            + ","
            + advancedCreative.isAdvanced()
            + ","
            + advancedCreative.isCreative());
    System.out.println(
        "CASE:flag_default_equality="
            + (normal == TooltipFlag.NORMAL)
            + ","
            + normal.equals(TooltipFlag.NORMAL)
            + ","
            + normal.equals(advanced)
            + ","
            + normalCreative.equals(normal.asCreative())
            + ","
            + (normalCreative == normal.asCreative())
            + ","
            + normal.equals("not-a-flag")
            + ","
            + normal.equals(null));
    Object flagObj = normalCreative;
    System.out.println(
        "CASE:flag_default_types="
            + flagObj.getClass().getName()
            + ","
            + (flagObj instanceof TooltipFlag.Default)
            + ","
            + (flagObj instanceof TooltipFlag)
            + ","
            + (flagObj instanceof Record)
            + ","
            + (flagObj instanceof String));

    Item.TooltipContext empty = Item.TooltipContext.EMPTY;
    System.out.println(
        "CASE:ctx_empty="
            + empty.tickRate()
            + ","
            + (empty.registries() == null)
            + ","
            + empty.isPeaceful()
            + ","
            + (empty.mapData(null) == null)
            + ","
            + (Item.TooltipContext.of((Level) null) == empty)
            + ","
            + empty.equals(Item.TooltipContext.EMPTY)
            + ","
            + empty.equals(null));
  }
}

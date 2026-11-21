import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.google.gson.JsonElement;
import com.google.gson.JsonParser;
import com.mojang.serialization.Codec;
import com.mojang.serialization.DataResult;
import com.mojang.serialization.JsonOps;
import com.mojang.serialization.codecs.RecordCodecBuilder;
import io.netty.buffer.ByteBuf;
import io.netty.buffer.Unpooled;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.HexFormat;
import java.util.LinkedList;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.TreeMap;
import java.util.function.Supplier;
import net.minecraft.SharedConstants;
import net.minecraft.advancements.predicates.CollectionPredicate;
import net.minecraft.advancements.predicates.ItemPredicate;
import net.minecraft.core.HolderLookup;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.data.registries.VanillaRegistries;
import net.minecraft.network.codec.ByteBufCodecs;
import net.minecraft.network.codec.StreamCodec;
import net.minecraft.resources.RegistryOps;
import net.minecraft.server.Bootstrap;
import net.minecraft.util.ARGB;
import net.minecraft.util.Mth;
import net.minecraft.world.item.ItemInstance;
import net.minecraft.world.item.ItemStackTemplate;
import org.apache.commons.lang3.math.Fraction;

/**
 * Java 25 ground-truth oracle for Bundle API's host-side surface against real
 * Minecraft 26.3 (`server-26.3.jar`), Commons Lang 3.20.0, DataFixerUpper 10.0.21,
 * and Netty 4.2.16.Final.
 *
 * Writes four deterministic capture files into `args[0]`:
 *   - fraction.txt
 *   - argb.txt
 *   - record_codec.txt
 *   - stream_codec_list.txt
 */
public final class BundleApiOracle {
  private static final HexFormat HEX = HexFormat.of();
  private static final Gson GSON = new GsonBuilder().disableHtmlEscaping().create();

  public record Content(List<ItemStackTemplate> items) {
    public static final Codec<Content> CODEC =
        ItemStackTemplate.CODEC.listOf().xmap(Content::new, Content::items);
  }

  public record BundleLikeComponent(Content content, int sizeMultiplier) {
    public static final Codec<BundleLikeComponent> CODEC =
        RecordCodecBuilder.create(
            instance ->
                instance
                    .group(
                        Content.CODEC.fieldOf("content").forGetter(BundleLikeComponent::content),
                        Codec.INT
                            .optionalFieldOf("size_multiplier", 1)
                            .forGetter(BundleLikeComponent::sizeMultiplier))
                    .apply(instance, BundleLikeComponent::new));
  }

  public record PredicateLike(
      Optional<CollectionPredicate<ItemInstance, ItemPredicate>> items) {
    public static final Codec<PredicateLike> CODEC =
        RecordCodecBuilder.create(
            instance ->
                instance
                    .group(
                        CollectionPredicate.codec(ItemPredicate.CODEC)
                            .optionalFieldOf("items")
                            .forGetter(PredicateLike::items))
                    .apply(instance, PredicateLike::new));
  }

  public record WireEntry(String id, int count) {
    public static final StreamCodec<ByteBuf, WireEntry> STREAM_CODEC =
        StreamCodec.composite(
            ByteBufCodecs.STRING_UTF8,
            WireEntry::id,
            ByteBufCodecs.VAR_INT,
            WireEntry::count,
            WireEntry::new);
  }

  public record WireContent(List<WireEntry> entries) {
    public static final StreamCodec<ByteBuf, WireContent> STREAM_CODEC =
        WireEntry.STREAM_CODEC
            .apply(ByteBufCodecs.list())
            .map(WireContent::new, WireContent::entries);
  }

  private static String fmtFraction(Fraction f) {
    return f.getNumerator()
        + "|"
        + f.getDenominator()
        + "|"
        + f
        + "|"
        + f.toProperString()
        + "|"
        + f.hashCode()
        + "|"
        + Double.doubleToLongBits(f.doubleValue())
        + "|"
        + Float.floatToIntBits(f.floatValue())
        + "|"
        + f.intValue()
        + "|"
        + f.longValue()
        + "|"
        + f.getProperWhole()
        + "|"
        + f.getProperNumerator();
  }

  private static String captureEx(Supplier<?> op) {
    try {
      return "OK:" + op.get();
    } catch (Throwable t) {
      return "EX:" + t.getClass().getName() + ":" + t.getMessage();
    }
  }

  private static String captureFraction() {
    StringBuilder sb = new StringBuilder();
    Map<String, Fraction> constants = new TreeMap<>();
    constants.put("ZERO", Fraction.ZERO);
    constants.put("ONE", Fraction.ONE);
    constants.put("ONE_HALF", Fraction.ONE_HALF);
    constants.put("ONE_THIRD", Fraction.ONE_THIRD);
    constants.put("TWO_THIRDS", Fraction.TWO_THIRDS);
    constants.put("ONE_QUARTER", Fraction.ONE_QUARTER);
    constants.put("TWO_QUARTERS", Fraction.TWO_QUARTERS);
    constants.put("THREE_QUARTERS", Fraction.THREE_QUARTERS);
    constants.put("ONE_FIFTH", Fraction.ONE_FIFTH);
    constants.put("TWO_FIFTHS", Fraction.TWO_FIFTHS);
    constants.put("THREE_FIFTHS", Fraction.THREE_FIFTHS);
    constants.put("FOUR_FIFTHS", Fraction.FOUR_FIFTHS);
    for (var e : constants.entrySet()) {
      sb.append("const.").append(e.getKey()).append('=').append(fmtFraction(e.getValue())).append('\n');
    }

    Fraction f01 = Fraction.getFraction(0, 1);
    Fraction f04 = Fraction.getFraction(0, 4);
    Fraction f24 = Fraction.getFraction(2, 4);
    sb.append("id.get01_eq_ZERO=").append(f01 == Fraction.ZERO).append('\n');
    sb.append("id.get01_equals_ZERO=").append(f01.equals(Fraction.ZERO)).append('\n');
    sb.append("id.red05_eq_ZERO=").append(Fraction.getReducedFraction(0, 5) == Fraction.ZERO).append('\n');
    sb.append("id.twoQ_equals_half=").append(Fraction.TWO_QUARTERS.equals(Fraction.ONE_HALF)).append('\n');
    sb.append("id.twoQ_cmp_half=").append(Fraction.TWO_QUARTERS.compareTo(Fraction.ONE_HALF)).append('\n');
    sb.append("id.twoQ_red_equals_half=").append(Fraction.TWO_QUARTERS.reduce().equals(Fraction.ONE_HALF)).append('\n');
    sb.append("id.zero_add_third=").append(Fraction.ZERO.add(Fraction.ONE_THIRD) == Fraction.ONE_THIRD).append('\n');
    sb.append("id.third_add_zero=").append(Fraction.ONE_THIRD.add(Fraction.ZERO) == Fraction.ONE_THIRD).append('\n');
    sb.append("id.third_mul_zero=").append(Fraction.ONE_THIRD.multiplyBy(Fraction.ZERO) == Fraction.ZERO).append('\n');
    sb.append("id.third_pow0=").append(Fraction.ONE_THIRD.pow(0) == Fraction.ONE).append('\n');
    sb.append("id.third_pow1=").append(Fraction.ONE_THIRD.pow(1) == Fraction.ONE_THIRD).append('\n');
    sb.append("id.third_abs=").append(Fraction.ONE_THIRD.abs() == Fraction.ONE_THIRD).append('\n');
    sb.append("id.third_reduce=").append(Fraction.ONE_THIRD.reduce() == Fraction.ONE_THIRD).append('\n');
    sb.append("id.f04_reduce_ZERO=").append(f04.reduce() == Fraction.ZERO).append('\n');
    sb.append("id.f01_reduce_self=").append(f01.reduce() == f01).append('\n');
    sb.append("id.f24_equals_twoQ=").append(f24.equals(Fraction.TWO_QUARTERS)).append('\n');
    sb.append("id.f24_same_twoQ=").append(f24 == Fraction.TWO_QUARTERS).append('\n');

    sb.append("ctor.ii_negden=").append(fmtFraction(Fraction.getFraction(3, -4))).append('\n');
    sb.append("ctor.ii_bothneg=").append(fmtFraction(Fraction.getFraction(-6, -8))).append('\n');
    sb.append("ctor.iii_pos=").append(fmtFraction(Fraction.getFraction(2, 3, 4))).append('\n');
    sb.append("ctor.iii_neg=").append(fmtFraction(Fraction.getFraction(-2, 3, 4))).append('\n');
    sb.append("ctor.iii_zero=").append(fmtFraction(Fraction.getFraction(0, 3, 4))).append('\n');
    sb.append("ctor.red_min=").append(fmtFraction(Fraction.getReducedFraction(-4, Integer.MIN_VALUE))).append('\n');
    sb.append("ctor.red_neg=").append(fmtFraction(Fraction.getReducedFraction(6, -9))).append('\n');
    sb.append("ctor.dbl_0=").append(fmtFraction(Fraction.getFraction(0.0))).append('\n');
    sb.append("ctor.dbl_half=").append(fmtFraction(Fraction.getFraction(0.5))).append('\n');
    sb.append("ctor.dbl_neg=").append(fmtFraction(Fraction.getFraction(-0.75))).append('\n');
    sb.append("ctor.dbl_third=").append(fmtFraction(Fraction.getFraction(1.0 / 3.0))).append('\n');
    sb.append("ctor.dbl_225=").append(fmtFraction(Fraction.getFraction(2.25))).append('\n');
    sb.append("ctor.str_frac=").append(fmtFraction(Fraction.getFraction("3/4"))).append('\n');
    sb.append("ctor.str_proper=").append(fmtFraction(Fraction.getFraction("-2 1/3"))).append('\n');
    sb.append("ctor.str_dec=").append(fmtFraction(Fraction.getFraction("0.25"))).append('\n');
    sb.append("ctor.str_int=").append(fmtFraction(Fraction.getFraction("7"))).append('\n');
    sb.append("ctor.str_neg=").append(fmtFraction(Fraction.getFraction("-5/10"))).append('\n');

    Fraction occ = Fraction.ZERO
        .add(Fraction.getFraction(1, 64))
        .add(Fraction.getFraction(3, 64))
        .add(Fraction.getFraction(1, 16));
    sb.append("arith.occ=").append(fmtFraction(occ)).append('\n');
    sb.append("arith.occ_div_2=").append(fmtFraction(occ.divideBy(Fraction.getFraction(2, 1)))).append('\n');
    sb.append("arith.coprime_add=").append(fmtFraction(Fraction.ONE_THIRD.add(Fraction.ONE_QUARTER))).append('\n');
    sb.append("arith.common_add=").append(fmtFraction(Fraction.getFraction(1, 6).add(Fraction.getFraction(1, 10)))).append('\n');
    sb.append("arith.common_sub=").append(fmtFraction(Fraction.getFraction(5, 12).subtract(Fraction.getFraction(1, 18)))).append('\n');
    sb.append("arith.sub_zero_same=").append(fmtFraction(Fraction.ONE_HALF.subtract(Fraction.ONE_HALF))).append('\n');
    sb.append("arith.sub_zero_unred=").append(fmtFraction(Fraction.ONE_HALF.subtract(Fraction.TWO_QUARTERS))).append('\n');
    sb.append("arith.sub_from_zero=").append(fmtFraction(Fraction.ZERO.subtract(Fraction.TWO_THIRDS))).append('\n');
    sb.append("arith.mul=").append(fmtFraction(Fraction.getFraction(6, 7).multiplyBy(Fraction.getFraction(14, 9)))).append('\n');
    sb.append("arith.div=").append(fmtFraction(Fraction.getFraction(3, 4).divideBy(Fraction.getFraction(-9, 8)))).append('\n');
    sb.append("arith.neg=").append(fmtFraction(Fraction.getFraction(5, 3).negate())).append('\n');
    sb.append("arith.abs_neg=").append(fmtFraction(Fraction.getFraction(-5, 3).abs())).append('\n');
    sb.append("arith.inv_neg=").append(fmtFraction(Fraction.getFraction(-3, 7).invert())).append('\n');
    sb.append("arith.reduce_min=").append(fmtFraction(Fraction.getFraction(Integer.MIN_VALUE, 2).reduce())).append('\n');
    sb.append("arith.pow2=").append(fmtFraction(Fraction.TWO_THIRDS.pow(2))).append('\n');
    sb.append("arith.pow3_neg=").append(fmtFraction(Fraction.getFraction(-2, 3).pow(3))).append('\n');
    sb.append("arith.pow_neg2=").append(fmtFraction(Fraction.TWO_THIRDS.pow(-2))).append('\n');
    sb.append("arith.pow_one_min=").append(fmtFraction(Fraction.ONE.pow(Integer.MIN_VALUE))).append('\n');
    sb.append("arith.cmp_lt=").append(Fraction.ONE_THIRD.compareTo(Fraction.ONE_HALF)).append('\n');
    sb.append("arith.cmp_gt=").append(Fraction.THREE_QUARTERS.compareTo(Fraction.ONE_HALF)).append('\n');

    sb.append("ex.den0=").append(captureEx(() -> Fraction.getFraction(1, 0))).append('\n');
    sb.append("ex.neg_min=").append(captureEx(() -> Fraction.getFraction(Integer.MIN_VALUE, -1))).append('\n');
    sb.append("ex.iii_negnum=").append(captureEx(() -> Fraction.getFraction(1, -1, 2))).append('\n');
    sb.append("ex.iii_negden=").append(captureEx(() -> Fraction.getFraction(1, 1, -2))).append('\n');
    sb.append("ex.iii_ovf=").append(captureEx(() -> Fraction.getFraction(Integer.MAX_VALUE, 1, 2))).append('\n');
    sb.append("ex.dbl_nan=").append(captureEx(() -> Fraction.getFraction(Double.NaN))).append('\n');
    sb.append("ex.dbl_inf=").append(captureEx(() -> Fraction.getFraction(Double.POSITIVE_INFINITY))).append('\n');
    sb.append("ex.str_null=").append(captureEx(() -> Fraction.getFraction((String) null))).append('\n');
    sb.append("ex.str_bad=").append(captureEx(() -> Fraction.getFraction("1 2"))).append('\n');
    sb.append("ex.inv0=").append(captureEx(Fraction.ZERO::invert)).append('\n');
    sb.append("ex.div0=").append(captureEx(() -> Fraction.ONE.divideBy(Fraction.ZERO))).append('\n');
    sb.append("ex.neg_min_num=").append(captureEx(() -> Fraction.getFraction(Integer.MIN_VALUE, 1).negate())).append('\n');
    sb.append("ex.inv_min_num=").append(captureEx(() -> Fraction.getFraction(Integer.MIN_VALUE, 1).invert())).append('\n');
    sb.append("ex.add_ovf=").append(captureEx(() -> Fraction.getFraction(Integer.MAX_VALUE, 1).add(Fraction.ONE))).append('\n');
    sb.append("ex.sub_ovf=").append(captureEx(() -> Fraction.getFraction(Integer.MIN_VALUE, 1).subtract(Fraction.ONE))).append('\n');
    sb.append("ex.mul_ovf=").append(captureEx(() -> Fraction.getFraction(Integer.MAX_VALUE, 2).multiplyBy(Fraction.getFraction(3, 1)))).append('\n');
    sb.append("ex.add_null=").append(captureEx(() -> Fraction.ONE.add(null))).append('\n');

    sb.append("mth.mul_0=").append(Mth.mulAndTruncate(Fraction.ZERO, 12)).append('\n');
    sb.append("mth.mul_half=").append(Mth.mulAndTruncate(Fraction.ONE_HALF, 12)).append('\n');
    sb.append("mth.mul_3q=").append(Mth.mulAndTruncate(Fraction.THREE_QUARTERS, 12)).append('\n');
    sb.append("mth.mul_1_64=").append(Mth.mulAndTruncate(Fraction.getFraction(1, 64), 12)).append('\n');
    sb.append("mth.mul_63_64=").append(Mth.mulAndTruncate(Fraction.getFraction(63, 64), 12)).append('\n');
    sb.append("mth.mul_1=").append(Mth.mulAndTruncate(Fraction.ONE, 12)).append('\n');
    sb.append("mth.mul_2=").append(Mth.mulAndTruncate(Fraction.getFraction(2, 1), 12)).append('\n');
    sb.append("mth.mul_neg=").append(Mth.mulAndTruncate(Fraction.getFraction(-3, 4), 12)).append('\n');
    return sb.toString();
  }

  private static String captureArgb() {
    StringBuilder sb = new StringBuilder();
    float[][] cases = {
        {1.0f, 0.4f, 0.4f, 1.0f},
        {0.0f, 0.0f, 0.0f, 0.0f},
        {1.0f, 1.0f, 1.0f, 1.0f},
        {0.5f, 0.25f, 0.75f, 0.1f},
        {1.0f, 0.0f, 0.5f, 1.0f},
        {-0.1f, 1.5f, 0.0f, 2.0f},
        {Float.NaN, Float.POSITIVE_INFINITY, Float.NEGATIVE_INFINITY, -0.0f},
    };
    for (int i = 0; i < cases.length; i++) {
      float[] c = cases[i];
      int v = ARGB.colorFromFloat(c[0], c[1], c[2], c[3]);
      sb.append("colorFromFloat.")
          .append(i)
          .append('=')
          .append(v)
          .append("|0x")
          .append(String.format("%08x", v))
          .append('\n');
    }
    sb.append("as8Bit.04=").append(ARGB.as8BitChannel(0.4f)).append('\n');
    sb.append("as8Bit.neg=").append(ARGB.as8BitChannel(-0.1f)).append('\n');
    sb.append("as8Bit.nan=").append(ARGB.as8BitChannel(Float.NaN)).append('\n');
    int bar = ARGB.colorFromFloat(1.0f, 0.4f, 0.4f, 1.0f);
    sb.append("channels.bar=")
        .append(ARGB.alpha(bar))
        .append('|')
        .append(ARGB.red(bar))
        .append('|')
        .append(ARGB.green(bar))
        .append('|')
        .append(ARGB.blue(bar))
        .append('\n');
    sb.append("color3=").append(ARGB.color(102, 102, 255)).append('\n');
    sb.append("gray=").append(ARGB.gray(0.5f)).append('\n');
    sb.append("multiply=").append(ARGB.multiply(0xFF804020, 0x80FFFFFF)).append('\n');
    sb.append("average=").append(ARGB.average(0xFF102030, 0x7F304050)).append('\n');
    sb.append("floats.bar=")
        .append(Float.floatToIntBits(ARGB.alphaFloat(bar)))
        .append('|')
        .append(Float.floatToIntBits(ARGB.redFloat(bar)))
        .append('|')
        .append(Float.floatToIntBits(ARGB.greenFloat(bar)))
        .append('|')
        .append(Float.floatToIntBits(ARGB.blueFloat(bar)))
        .append('\n');
    return sb.toString();
  }

  private static String captureRecordCodec(RegistryOps<JsonElement> ops) {
    StringBuilder sb = new StringBuilder();
    sb.append("predicate.codec_nonnull=").append(PredicateLike.CODEC != null).append('\n');

    Map<String, String> inputs = new TreeMap<>();
    inputs.put("01_empty_default", """
        {"content":[]}
        """);
    inputs.put("02_shorthand_mult4", """
        {"content":["minecraft:apple"],"size_multiplier":4}
        """);
    inputs.put("03_explicit_default_mult", """
        {"content":[{"id":"minecraft:diamond","count":16},{"id":"minecraft:iron_sword","components":{"minecraft:damage":5,"minecraft:custom_name":"Blade"}}],"size_multiplier":1}
        """);
    inputs.put("04_missing_content", """
        {"size_multiplier":2}
        """);
    inputs.put("05_bad_count_zero", """
        {"content":[{"id":"minecraft:apple","count":0}]}
        """);
    inputs.put("06_bad_count_100", """
        {"content":[{"id":"minecraft:apple","count":100}]}
        """);
    inputs.put("07_air_item", """
        {"content":["minecraft:air"]}
        """);
    inputs.put("08_bad_mult_type", """
        {"content":[],"size_multiplier":"four"}
        """);

    for (var e : inputs.entrySet()) {
      DataResult<BundleLikeComponent> decoded =
          BundleLikeComponent.CODEC.parse(ops, JsonParser.parseString(e.getValue().trim()));
      if (decoded.isSuccess()) {
        BundleLikeComponent comp = decoded.getOrThrow();
        DataResult<JsonElement> reencoded = BundleLikeComponent.CODEC.encodeStart(ops, comp);
        String encodedJson = GSON.toJson(reencoded.getOrThrow());
        sb.append("case.")
            .append(e.getKey())
            .append("=OK|size=")
            .append(comp.content().items().size())
            .append("|mult=")
            .append(comp.sizeMultiplier())
            .append("|json=")
            .append(encodedJson)
            .append('\n');
      } else {
        sb.append("case.").append(e.getKey()).append("=ERROR\n");
      }
    }
    return sb.toString();
  }

  private static <T> String roundTripHex(StreamCodec<ByteBuf, T> codec, T value) {
    ByteBuf buf = Unpooled.buffer();
    try {
      codec.encode(buf, value);
      byte[] bytes = new byte[buf.readableBytes()];
      buf.getBytes(buf.readerIndex(), bytes);
      T decoded = codec.decode(buf);
      return "hex="
          + HEX.formatHex(bytes)
          + "|remaining="
          + buf.readableBytes()
          + "|eq="
          + value.equals(decoded)
          + "|val="
          + decoded;
    } finally {
      buf.release();
    }
  }

  private static String captureStreamCodecList() {
    StringBuilder sb = new StringBuilder();
    StreamCodec<ByteBuf, List<Integer>> intList = ByteBufCodecs.VAR_INT.apply(ByteBufCodecs.list());
    StreamCodec<ByteBuf, List<Integer>> intList2 = ByteBufCodecs.VAR_INT.apply(ByteBufCodecs.list());
    StreamCodec<ByteBuf, List<String>> strList3 =
        ByteBufCodecs.STRING_UTF8.apply(ByteBufCodecs.list(3));
    StreamCodec<ByteBuf, List<Integer>> fixed2 =
        ByteBufCodecs.VAR_INT.apply(ByteBufCodecs.fixedSizeList(2));
    StreamCodec<ByteBuf, List<Optional<Integer>>> optList =
        ByteBufCodecs.optional(ByteBufCodecs.VAR_INT).apply(ByteBufCodecs.list());
    StreamCodec<ByteBuf, LinkedList<Integer>> linkedList =
        ByteBufCodecs.VAR_INT.apply(ByteBufCodecs.collection(n -> new LinkedList<>(), 4));

    sb.append("class.list=").append(intList.getClass().getName()).append('\n');
    sb.append("class.fixed=").append(fixed2.getClass().getName()).append('\n');
    sb.append("class.map=").append(WireContent.STREAM_CODEC.getClass().getName()).append('\n');
    sb.append("class.composite2=").append(WireEntry.STREAM_CODEC.getClass().getName()).append('\n');
    sb.append("distinct.list=").append(intList != intList2).append('\n');

    sb.append("rt.int_empty=").append(roundTripHex(intList, List.of())).append('\n');
    sb.append("rt.int_vals=")
        .append(
            roundTripHex(
                intList,
                List.of(0, 1, 127, 128, 16384, -1, Integer.MAX_VALUE, Integer.MIN_VALUE)))
        .append('\n');
    sb.append("rt.str_empty=").append(roundTripHex(strList3, List.of())).append('\n');
    sb.append("rt.str_vals=")
        .append(roundTripHex(strList3, List.of("alpha", "\u03b2", "")))
        .append('\n');
    sb.append("rt.wire_content=")
        .append(
            roundTripHex(
                WireContent.STREAM_CODEC,
                new WireContent(
                    List.of(new WireEntry("minecraft:apple", 3), new WireEntry("runes:fire_rune", 64)))))
        .append('\n');
    sb.append("rt.fixed2=").append(roundTripHex(fixed2, List.of(7, 42))).append('\n');
    sb.append("rt.opt_list=")
        .append(roundTripHex(optList, List.of(Optional.empty(), Optional.of(99))))
        .append('\n');
    LinkedList<Integer> ll = new LinkedList<>(List.of(10, 20));
    sb.append("rt.linked=").append(roundTripHex(linkedList, ll)).append('\n');

    sb.append("ex.encode_max=")
        .append(
            captureEx(
                () -> {
                  ByteBuf b = Unpooled.buffer();
                  try {
                    strList3.encode(b, List.of("a", "b", "c", "d"));
                    return "ok";
                  } finally {
                    b.release();
                  }
                }))
        .append('\n');
    sb.append("ex.decode_max=")
        .append(
            captureEx(
                () -> {
                  ByteBuf b = Unpooled.buffer();
                  try {
                    ByteBufCodecs.STRING_UTF8
                        .apply(ByteBufCodecs.list())
                        .encode(b, List.of("a", "b", "c", "d"));
                    return strList3.decode(b);
                  } finally {
                    b.release();
                  }
                }))
        .append('\n');
    sb.append("ex.fixed_wrong_size=")
        .append(
            captureEx(
                () -> {
                  ByteBuf b = Unpooled.buffer();
                  try {
                    fixed2.encode(b, List.of(7));
                    return "ok";
                  } finally {
                    b.release();
                  }
                }))
        .append('\n');
    return sb.toString();
  }

  public static void main(String[] args) throws IOException {
    Path outDir = Path.of(args[0]);
    Files.createDirectories(outDir);

    SharedConstants.tryDetectVersion();
    Bootstrap.bootStrap();
    HolderLookup.Provider lookup = VanillaRegistries.createWorldLookup();
    BuiltInRegistries.DATA_COMPONENT_INITIALIZERS.build(lookup).forEach(p -> p.apply());
    RegistryOps<JsonElement> ops = RegistryOps.create(JsonOps.INSTANCE, lookup);

    Files.writeString(outDir.resolve("fraction.txt"), captureFraction(), StandardCharsets.UTF_8);
    Files.writeString(outDir.resolve("argb.txt"), captureArgb(), StandardCharsets.UTF_8);
    Files.writeString(
        outDir.resolve("record_codec.txt"), captureRecordCodec(ops), StandardCharsets.UTF_8);
    Files.writeString(
        outDir.resolve("stream_codec_list.txt"),
        captureStreamCodecList(),
        StandardCharsets.UTF_8);
  }
}

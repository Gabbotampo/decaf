package demo;

import java.util.List;
import io.netty.buffer.ByteBuf;
import io.netty.buffer.Unpooled;
import net.minecraft.network.FriendlyByteBuf;
import net.minecraft.network.codec.StreamCodec;
import net.minecraft.network.codec.StreamDecoder;
import net.minecraft.network.codec.StreamEncoder;
import net.minecraft.network.codec.StreamMemberEncoder;
import net.minecraft.network.protocol.common.custom.CustomPacketPayload;

public final class StreamCodecs {
  private static final class Payload implements CustomPacketPayload {
    @Override public Type<? extends CustomPacketPayload> type() {
      return null;
    }
  }

  private static StreamCodec<ByteBuf, Payload> compositeOne() {
    return StreamCodec.<ByteBuf, Payload, Integer>composite(null, null, null);
  }

  private static StreamCodec<ByteBuf, Payload> compositeTwo() {
    return StreamCodec.<ByteBuf, Payload, Integer, Long>composite(null, null, null, null, null);
  }

  private static StreamCodec<ByteBuf, Payload> memberCodec() {
    StreamMemberEncoder<ByteBuf, Payload> encoder = (value, output) -> {};
    StreamDecoder<ByteBuf, Payload> decoder = input -> new Payload();
    return CustomPacketPayload.codec(encoder, decoder);
  }

  private static StreamCodec<FriendlyByteBuf, CustomPacketPayload> fallbackCodec() {
    CustomPacketPayload.FallbackProvider<FriendlyByteBuf> fallback = input -> null;
    return CustomPacketPayload.codec(
        fallback, List.of());
  }

  private static String facts(StreamCodec<?, ?> codec) {
    return codec.getClass().getName() + "|" + (codec instanceof StreamEncoder)
        + "|" + (codec instanceof StreamDecoder);
  }

  public static String constructions() {
    StreamCodec<ByteBuf, Payload> one = compositeOne();
    StreamCodec<ByteBuf, Payload> oneAgain = compositeOne();
    StreamCodec<ByteBuf, Payload> two = compositeTwo();
    StreamCodec<ByteBuf, Payload> member = memberCodec();
    StreamCodec<FriendlyByteBuf, CustomPacketPayload> fallback = fallbackCodec();
    return facts(one) + "|" + facts(two) + "|" + facts(member) + "|" + facts(fallback)
        + "|" + (one == one) + "|" + (one == oneAgain) + "|" + (one.equals(one))
        + "|" + one.equals(oneAgain);
  }

  public static String encodeDecode() {
    StreamCodec<ByteBuf, Payload> one = compositeOne();
    try {
      one.encode(null, new Payload());
      return String.valueOf(one.decode(null));
    } catch (Throwable x) {
      return x.getClass().getName();
    }
  }
}

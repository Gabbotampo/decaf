package net.minecraft.network.protocol.common.custom;

import java.util.List;
import net.minecraft.network.codec.StreamCodec;
import net.minecraft.network.codec.StreamDecoder;
import net.minecraft.network.codec.StreamMemberEncoder;

public interface CustomPacketPayload {
  interface Type<T> {}
  Type<? extends CustomPacketPayload> type();
  @FunctionalInterface
  interface FallbackProvider<B> {
    CustomPacketPayload fallback(B buffer);
  }

  interface TypeAndCodec<B, T> {}

  static <B, T extends CustomPacketPayload> StreamCodec<B, T> codec(
      StreamMemberEncoder<B, T> encoder, StreamDecoder<B, T> decoder) {
    return null;
  }

  static <B> StreamCodec<B, CustomPacketPayload> codec(
      FallbackProvider<B> fallback, List<TypeAndCodec<? super B, ?>> codecs) {
    return null;
  }
}

package net.minecraft.network.codec;

// Compile-only stub of Minecraft 26.3's interface (the oracle runs the real one).
public interface ByteBufCodecs {
    StreamCodec<Object, Integer> INT = new StreamCodec<>() {};
    StreamCodec<Object, Integer> VAR_INT = new StreamCodec<>() {};
}

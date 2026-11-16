package net.minecraft.util;

import java.util.function.Supplier;

public interface StringRepresentable {
    final class EnumCodec implements StringRepresentable {}
    static EnumCodec fromEnum(Supplier<?> supplier) { return null; }
}

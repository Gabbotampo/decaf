package net.minecraft.network.syncher; import net.minecraft.network.codec.RegistryFriendlyByteBuf; public interface EntityDataSerializer<T>{ Object codec(); T copy(T x); }

package demo;

import net.minecraft.server.Bootstrap;
import net.minecraft.SharedConstants;
import net.minecraft.core.Holder;
import net.minecraft.world.effect.MobEffects;
import net.minecraft.world.entity.ai.attributes.Attributes;

public final class Probe {
    static String facts(Holder<?> holder) {
        Holder.Reference raw = (Holder.Reference) holder;
        return holder.getRegisteredName() + "|" + holder.unwrapKey().get() + "|" + raw.key()
            + "|" + holder.isBound() + "|" + raw.is(raw.key()) + "|"
            + (holder == MobEffects.SPEED) + "|" + (holder.value() == holder.value());
    }

    public static void main(String[] args) {
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        System.out.println(facts(MobEffects.SPEED));
        System.out.println(facts(Attributes.ARMOR));
        Object forged = "client/mob-effect/minecraft:speed";
        System.out.println((forged instanceof Holder) + "|" + forged.equals(MobEffects.SPEED)
            + "|" + MobEffects.SPEED.equals(forged));
    }
}

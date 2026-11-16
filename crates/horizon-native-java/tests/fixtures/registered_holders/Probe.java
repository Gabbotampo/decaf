package demo;

import net.minecraft.SharedConstants;
import net.minecraft.server.Bootstrap;
import net.minecraft.core.Holder;
import net.minecraft.core.Registry;
import net.minecraft.core.MappedRegistry;
import net.minecraft.core.registries.Registries;
import com.mojang.serialization.Lifecycle;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.resources.Identifier;
import net.minecraft.world.effect.MobEffect;
import net.minecraft.world.effect.MobEffectCategory;
import net.minecraft.world.entity.ai.attributes.Attribute;
import net.minecraft.world.entity.ai.attributes.RangedAttribute;

public final class Probe {
    static final class Effect extends MobEffect {
        Effect() { super(MobEffectCategory.BENEFICIAL, 1); }
    }

    static String facts(String label, Holder<?> holder, Identifier id, Object expected) {
        Object key = holder.unwrapKey().orElse(null);
        return label + "|" + holder.getRegisteredName() + "|" + key + "|"
            + (holder.value() == expected) + "|" + holder.isBound() + "|" + holder.kind()
            + "|" + holder.is(id);
    }

    public static void main(String[] args) {
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        Identifier effectId = Identifier.fromNamespaceAndPath("probe", "effect");
        MobEffect effect = new Effect();
        Registry<MobEffect> effectRegistry = new MappedRegistry<>(Registries.MOB_EFFECT, Lifecycle.stable());
        Holder.Reference<MobEffect> effectHolder = Registry.registerForHolder(
            effectRegistry, effectId, effect);
        effectRegistry.freeze();
        System.out.println(facts("effect", effectHolder, effectId, effect));

        Identifier attributeId = Identifier.fromNamespaceAndPath("probe", "attribute");
        Registry<Attribute> attributeRegistry = new MappedRegistry<>(Registries.ATTRIBUTE, Lifecycle.stable());
        Attribute attribute = Registry.register(attributeRegistry, attributeId,
            new RangedAttribute("attribute", 1.0, 0.0, 2.0));
        attributeRegistry.freeze();
        System.out.println(facts("direct", Holder.direct(attribute), attributeId, attribute));
        System.out.println(facts("wrapped", attributeRegistry.wrapAsHolder(attribute), attributeId, attribute));
    }
}

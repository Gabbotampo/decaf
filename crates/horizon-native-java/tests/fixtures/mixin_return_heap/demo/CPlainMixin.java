package demo;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

@Mixin(value = Target.class, targets = {"Ldemo/Target;"})
public abstract class CPlainMixin {
    @Inject(method = "plain()Ljava/lang/Object;", at = @At("RETURN"))
    private static void onPlain(CallbackInfoReturnable<Object> info) {
        Target.garbage();
    }
}

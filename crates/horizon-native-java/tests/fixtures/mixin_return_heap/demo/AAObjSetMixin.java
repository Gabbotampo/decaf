package demo;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

@Mixin(value = Target.class, targets = {"Ldemo/Target;"})
public abstract class AAObjSetMixin {
    @Inject(method = "obj()Ljava/lang/Object;", at = @At("RETURN"), cancellable = true)
    private static void onObj(CallbackInfoReturnable<Object> info) {
        Probe.mark("saw:" + ((Cell) info.getReturnValue()).show());
        Target.garbage();
        info.setReturnValue(new Cell(7, "override"));
    }
}

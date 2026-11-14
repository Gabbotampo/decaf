package demo;
import java.lang.reflect.Method;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;
/** Java 25 oracle: what woven code answers. obj(): target, then AA, then BB on one returnable; plain(): target then C. */
public final class Oracle {
    static Method m(Class<?> c, String n) throws Exception { Method x = c.getDeclaredMethod(n, CallbackInfoReturnable.class); x.setAccessible(true); return x; }
    public static void main(String[] a) throws Exception {
        CallbackInfoReturnable<Object> info = new CallbackInfoReturnable<>();
        info.setReturnValue(Target.obj());
        m(AAObjSetMixin.class, "onObj").invoke(null, info);
        System.gc();
        m(BBObjSeeMixin.class, "onObj").invoke(null, info);
        System.out.println("discovered.read=" + Target.read(info.getReturnValue()));
        CallbackInfoReturnable<Object> plain = new CallbackInfoReturnable<>();
        plain.setReturnValue(Target.plain());
        m(CPlainMixin.class, "onPlain").invoke(null, plain);
        System.gc();
        System.out.println("plain.read=" + Target.read(plain.getReturnValue()));
    }
}

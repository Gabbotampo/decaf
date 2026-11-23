package demo;

import java.util.concurrent.RejectedExecutionHandler;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.ScheduledThreadPoolExecutor;
import java.util.concurrent.ThreadFactory;
import java.util.concurrent.TimeUnit;

public class Probe {
    static String policies(ScheduledThreadPoolExecutor pool) {
        return pool.getExecuteExistingDelayedTasksAfterShutdownPolicy() + ":" + pool.getRemoveOnCancelPolicy();
    }

    interface Build {
        Object make() throws Exception;
    }

    static String built(Build build) {
        try {
            build.make();
            return "ok";
        } catch (Throwable t) {
            return t.getClass().getName() + ": " + t.getMessage();
        }
    }

    static String rows() throws Exception {
        StringBuilder s = new StringBuilder();
        ThreadFactory factory = r -> new Thread(r);
        RejectedExecutionHandler handler = (r, e) -> { };

        // Defaults, whichever constructor built the pool.
        s.append("defaults ").append(policies(new ScheduledThreadPoolExecutor(1))).append('\n');
        s.append("defaults-factory ").append(policies(new ScheduledThreadPoolExecutor(1, factory))).append('\n');
        s.append("defaults-handler ").append(policies(new ScheduledThreadPoolExecutor(2, handler))).append('\n');
        s.append("defaults-both ").append(policies(new ScheduledThreadPoolExecutor(0, factory, handler))).append('\n');

        // The constructors refuse the arguments ThreadPoolExecutor's refuses: the
        // size first, then the factory and the handler by name.
        ThreadFactory noFactory = null;
        RejectedExecutionHandler noHandler = null;
        s.append("ctor-neg ").append(built(() -> new ScheduledThreadPoolExecutor(-1))).append('\n');
        s.append("ctor-min ").append(built(() -> new ScheduledThreadPoolExecutor(Integer.MIN_VALUE))).append('\n');
        s.append("ctor-neg-factory ").append(built(() -> new ScheduledThreadPoolExecutor(-1, noFactory))).append('\n');
        s.append("ctor-neg-handler ").append(built(() -> new ScheduledThreadPoolExecutor(-1, noHandler))).append('\n');
        s.append("ctor-neg-both ").append(built(() -> new ScheduledThreadPoolExecutor(-1, noFactory, noHandler))).append('\n');
        s.append("ctor-null-factory ").append(built(() -> new ScheduledThreadPoolExecutor(1, noFactory))).append('\n');
        s.append("ctor-null-handler ").append(built(() -> new ScheduledThreadPoolExecutor(1, noHandler))).append('\n');
        s.append("ctor-null-factory3 ").append(built(() -> new ScheduledThreadPoolExecutor(1, noFactory, handler))).append('\n');
        s.append("ctor-null-handler3 ").append(built(() -> new ScheduledThreadPoolExecutor(1, factory, noHandler))).append('\n');
        s.append("ctor-null-both ").append(built(() -> new ScheduledThreadPoolExecutor(1, noFactory, noHandler))).append('\n');
        s.append("ctor-zero ").append(built(() -> new ScheduledThreadPoolExecutor(0))).append('\n');
        s.append("ctor-max ").append(built(() -> new ScheduledThreadPoolExecutor(Integer.MAX_VALUE, factory, handler))).append('\n');

        // Each setter changes only its own policy, and the getter answers it.
        ScheduledThreadPoolExecutor pool = new ScheduledThreadPoolExecutor(1, factory);
        pool.setRemoveOnCancelPolicy(true);
        s.append("remove-on-cancel ").append(policies(pool)).append('\n');
        pool.setExecuteExistingDelayedTasksAfterShutdownPolicy(false);
        s.append("execute-existing-off ").append(policies(pool)).append('\n');
        pool.setRemoveOnCancelPolicy(false);
        s.append("remove-on-cancel-off ").append(policies(pool)).append('\n');
        pool.setExecuteExistingDelayedTasksAfterShutdownPolicy(true);
        s.append("execute-existing-on ").append(policies(pool)).append('\n');
        pool.setRemoveOnCancelPolicy(true);
        pool.setRemoveOnCancelPolicy(true);
        pool.setExecuteExistingDelayedTasksAfterShutdownPolicy(false);
        pool.setExecuteExistingDelayedTasksAfterShutdownPolicy(false);
        s.append("repeated ").append(policies(pool)).append('\n');

        // Pools keep their own policies.
        ScheduledThreadPoolExecutor other = new ScheduledThreadPoolExecutor(1);
        s.append("independent ").append(policies(pool)).append('/').append(policies(other)).append('\n');

        // The JEI delayed executor's pool: policies set, then shut down with
        // nothing queued. (Tasks on such a pool are not answered: Pending.java.)
        ScheduledThreadPoolExecutor jei = new ScheduledThreadPoolExecutor(1, factory);
        jei.setRemoveOnCancelPolicy(true);
        jei.setExecuteExistingDelayedTasksAfterShutdownPolicy(false);
        ScheduledExecutorService service = jei;
        s.append("jei ").append(policies(jei)).append('\n');

        // Shutdown with nothing queued: the policies stay readable and settable.
        service.shutdown();
        s.append("shutdown ").append(service.isShutdown()).append(':')
            .append(service.awaitTermination(1, TimeUnit.SECONDS)).append(':')
            .append(service.isTerminated()).append(' ').append(policies(jei)).append('\n');
        jei.setRemoveOnCancelPolicy(false);
        jei.setExecuteExistingDelayedTasksAfterShutdownPolicy(true);
        s.append("after-shutdown ").append(policies(jei)).append(' ').append(service.isShutdown()).append('\n');
        return s.toString();
    }

    public static void main(String[] args) throws Exception {
        System.out.print(rows());
    }
}

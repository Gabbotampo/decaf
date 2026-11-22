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

    static String rows() throws Exception {
        StringBuilder s = new StringBuilder();
        ThreadFactory factory = r -> new Thread(r);
        RejectedExecutionHandler handler = (r, e) -> { };

        // Defaults, whichever constructor built the pool.
        s.append("defaults ").append(policies(new ScheduledThreadPoolExecutor(1))).append('\n');
        s.append("defaults-factory ").append(policies(new ScheduledThreadPoolExecutor(1, factory))).append('\n');
        s.append("defaults-handler ").append(policies(new ScheduledThreadPoolExecutor(2, handler))).append('\n');
        s.append("defaults-both ").append(policies(new ScheduledThreadPoolExecutor(0, factory, handler))).append('\n');

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

        // The JEI delayed executor's pool, then a task that runs at once.
        ScheduledThreadPoolExecutor jei = new ScheduledThreadPoolExecutor(1, factory);
        jei.setRemoveOnCancelPolicy(true);
        jei.setExecuteExistingDelayedTasksAfterShutdownPolicy(false);
        ScheduledExecutorService service = jei;
        int[] ran = new int[1];
        Object result = service.schedule(() -> { ran[0]++; return "done"; }, 0, TimeUnit.NANOSECONDS).get();
        s.append("jei ").append(policies(jei)).append(' ').append(result).append(' ').append(ran[0]).append('\n');

        // Shutdown with nothing queued: the policies stay readable and settable.
        service.shutdown();
        s.append("shutdown ").append(service.isShutdown()).append(':')
            .append(service.awaitTermination(1, TimeUnit.SECONDS)).append(':')
            .append(service.isTerminated()).append(' ').append(policies(jei)).append('\n');
        jei.setRemoveOnCancelPolicy(false);
        jei.setExecuteExistingDelayedTasksAfterShutdownPolicy(true);
        s.append("after-shutdown ").append(policies(jei)).append(' ').append(service.isShutdown()).append('\n');
        try {
            service.schedule(() -> { ran[0]++; }, 0, TimeUnit.NANOSECONDS);
            s.append("rejected none");
        } catch (java.util.concurrent.RejectedExecutionException e) {
            s.append("rejected ").append(e.getClass().getName()).append(' ').append(ran[0]);
        }
        s.append('\n');
        return s.toString();
    }

    public static void main(String[] args) throws Exception {
        System.out.print(rows());
    }
}

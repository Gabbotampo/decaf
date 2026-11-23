package demo;

import java.util.concurrent.Callable;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.ScheduledFuture;
import java.util.concurrent.ScheduledThreadPoolExecutor;
import java.util.concurrent.ThreadFactory;
import java.util.concurrent.TimeUnit;

/**
 * Rows the runtime does NOT answer: a task call first, a policy setter or getter
 * after it, on the same pool. The runtime runs a task when it is scheduled, in the
 * caller's thread, ignoring the delay and the pool's thread, so Java's answer
 * (java25-late.txt) differs from the runtime's; the setter or getter that follows
 * stays an unknown host call, as it is upstream.
 */
public class Late {
    static final String[] NAMES = {
        "late-cancel", "late-getter-await", "late-exec-off", "late-name", "late-name-getter",
        "late-shutdownnow", "late-execute-name", "late-submit-name",
    };

    static ThreadFactory named(String name) {
        return r -> {
            Thread t = new Thread(r, name);
            t.setDaemon(true);
            return t;
        };
    }

    static String row(String name) throws Exception {
        ScheduledThreadPoolExecutor pool = new ScheduledThreadPoolExecutor(1, named("W"));
        ScheduledExecutorService svc = pool;
        try {
            switch (name) {
                case "late-cancel": {
                    ScheduledFuture<?> f = svc.schedule(() -> { }, 1, TimeUnit.HOURS);
                    pool.setRemoveOnCancelPolicy(true);
                    boolean cancel = f.cancel(false);
                    return "cancel=" + cancel + " cancelled=" + f.isCancelled() + " done=" + f.isDone();
                }
                case "late-getter-await": {
                    svc.schedule(() -> { }, 1, TimeUnit.HOURS);
                    boolean g = pool.getRemoveOnCancelPolicy();
                    svc.shutdown();
                    boolean await = svc.awaitTermination(1, TimeUnit.MILLISECONDS);
                    return "g=" + g + " await=" + await + " term=" + svc.isTerminated();
                }
                case "late-exec-off": {
                    ScheduledFuture<?> f = svc.schedule(() -> { }, 1, TimeUnit.HOURS);
                    svc.shutdown();
                    pool.setExecuteExistingDelayedTasksAfterShutdownPolicy(false);
                    boolean await = svc.awaitTermination(1, TimeUnit.SECONDS);
                    return "cancelled=" + f.isCancelled() + " done=" + f.isDone() + " await=" + await;
                }
                case "late-name": {
                    String seen = svc.schedule(() -> Thread.currentThread().getName(), 0, TimeUnit.NANOSECONDS).get();
                    pool.setRemoveOnCancelPolicy(true);
                    return seen;
                }
                case "late-name-getter": {
                    String seen = svc.schedule(() -> Thread.currentThread().getName(), 0, TimeUnit.NANOSECONDS).get();
                    pool.getExecuteExistingDelayedTasksAfterShutdownPolicy();
                    return seen;
                }
                case "late-shutdownnow": {
                    svc.schedule(() -> { }, 1, TimeUnit.HOURS);
                    pool.setRemoveOnCancelPolicy(true);
                    return "pending=" + svc.shutdownNow().size();
                }
                case "late-execute-name": {
                    String[] seen = new String[1];
                    CountDownLatch done = new CountDownLatch(1);
                    svc.execute(() -> {
                        seen[0] = Thread.currentThread().getName();
                        done.countDown();
                    });
                    done.await();
                    pool.getRemoveOnCancelPolicy();
                    return seen[0];
                }
                case "late-submit-name": {
                    Callable<String> task = () -> Thread.currentThread().getName();
                    String seen = svc.submit(task).get();
                    pool.setExecuteExistingDelayedTasksAfterShutdownPolicy(false);
                    return seen;
                }
                default:
                    return "unknown row";
            }
        } finally {
            pool.shutdownNow();
        }
    }

    static String rows() throws Exception {
        StringBuilder s = new StringBuilder();
        for (String name : NAMES) {
            String result;
            try {
                result = row(name);
            } catch (Throwable t) {
                result = "EX " + t.getClass().getName() + ": " + t.getMessage();
            }
            s.append(name).append(' ').append(result).append('\n');
        }
        return s.toString();
    }

    public static void main(String[] args) throws Exception {
        System.out.print(rows());
    }
}

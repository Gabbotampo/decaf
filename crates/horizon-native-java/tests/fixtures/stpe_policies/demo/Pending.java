package demo;

import java.util.concurrent.Callable;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.RejectedExecutionHandler;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.ScheduledThreadPoolExecutor;
import java.util.concurrent.ThreadFactory;
import java.util.concurrent.ThreadPoolExecutor;
import java.util.concurrent.TimeUnit;

/**
 * Rows the runtime does NOT answer: a task, run by a pool whose policy a jar set
 * or read, or by a pool built with a handler after shutdown. Java's answer is
 * recorded (java25-counted.txt); the runtime counts each row as an unknown host
 * call instead of answering it, because it runs a task when it is scheduled, in
 * the caller's thread, without the pool's factory or handler.
 */
public class Pending {
    static final String[] NAMES = {
        "factory-count", "factory-name", "factory-throws", "throwing-task", "throwing-callable",
        "null-unit", "getter-only-name", "execute-name", "submit-name", "handler-discard",
        "handler-caller-runs", "handler-abort", "delayed-drop-on-shutdown", "delayed-run-on-shutdown",
    };

    static ThreadFactory named(String name, int[] calls) {
        return r -> {
            calls[0]++;
            Thread t = new Thread(r, name);
            t.setDaemon(true);
            return t;
        };
    }

    static String get(java.util.concurrent.Future<?> f) throws Exception {
        try {
            return "got " + f.get();
        } catch (ExecutionException e) {
            return "get-EE " + e.getCause().getClass().getName() + ": " + e.getCause().getMessage();
        }
    }

    static String row(String name) throws Exception {
        int[] calls = new int[1];
        ScheduledThreadPoolExecutor pool = new ScheduledThreadPoolExecutor(1, named("W-custom", calls));
        pool.setRemoveOnCancelPolicy(true);
        ScheduledExecutorService svc = pool;
        try {
            switch (name) {
                case "factory-count":
                    svc.schedule(() -> 1, 0, TimeUnit.NANOSECONDS).get();
                    return "factoryCalls=" + calls[0];
                case "factory-name":
                    return get(svc.schedule(() -> Thread.currentThread().getName(), 0, TimeUnit.NANOSECONDS));
                case "factory-throws": {
                    ScheduledThreadPoolExecutor p = new ScheduledThreadPoolExecutor(1, r -> {
                        throw new IllegalStateException("boom");
                    });
                    p.setRemoveOnCancelPolicy(true);
                    ScheduledExecutorService q = p;
                    q.schedule(() -> { }, 0, TimeUnit.NANOSECONDS);
                    return "scheduled";
                }
                case "throwing-task":
                    return "scheduled " + get(svc.schedule((Runnable) () -> {
                        throw new IllegalStateException("task");
                    }, 0, TimeUnit.NANOSECONDS));
                case "throwing-callable":
                    return "scheduled " + get(svc.schedule((Callable<Object>) () -> {
                        throw new java.io.IOException("io");
                    }, 0, TimeUnit.NANOSECONDS));
                case "null-unit":
                    svc.schedule(() -> { }, 0, null);
                    return "no-exception";
                case "getter-only-name": {
                    ScheduledThreadPoolExecutor p = new ScheduledThreadPoolExecutor(1, named("W-getter", calls));
                    boolean flag = p.getRemoveOnCancelPolicy();
                    return flag + " " + get(p.schedule(() -> Thread.currentThread().getName(), 0, TimeUnit.NANOSECONDS));
                }
                case "execute-name": {
                    String[] seen = new String[1];
                    java.util.concurrent.CountDownLatch done = new java.util.concurrent.CountDownLatch(1);
                    svc.execute(() -> {
                        seen[0] = Thread.currentThread().getName();
                        done.countDown();
                    });
                    done.await();
                    return seen[0];
                }
                case "submit-name":
                    return get(svc.submit(() -> Thread.currentThread().getName()));
                case "handler-discard": {
                    int[] handled = new int[1];
                    ScheduledThreadPoolExecutor p = new ScheduledThreadPoolExecutor(1, (r, e) -> handled[0]++);
                    ScheduledExecutorService q = p;
                    q.shutdown();
                    try {
                        q.schedule(() -> { }, 0, TimeUnit.NANOSECONDS);
                        return "no-exception handlerCalls=" + handled[0];
                    } catch (java.util.concurrent.RejectedExecutionException e) {
                        return "REE handlerCalls=" + handled[0];
                    }
                }
                case "handler-caller-runs": {
                    int[] ran = new int[1];
                    ScheduledExecutorService q = new ScheduledThreadPoolExecutor(1, new ThreadPoolExecutor.CallerRunsPolicy());
                    q.shutdown();
                    try {
                        q.schedule(() -> ran[0]++, 0, TimeUnit.NANOSECONDS);
                        return "no-exception ran=" + ran[0];
                    } catch (java.util.concurrent.RejectedExecutionException e) {
                        return "REE ran=" + ran[0];
                    }
                }
                case "handler-abort": {
                    ScheduledExecutorService q = new ScheduledThreadPoolExecutor(1, new ThreadPoolExecutor.AbortPolicy());
                    q.shutdown();
                    try {
                        q.schedule(() -> { }, 0, TimeUnit.NANOSECONDS);
                        return "no-exception";
                    } catch (java.util.concurrent.RejectedExecutionException e) {
                        return "REE " + e.getClass().getName();
                    }
                }
                case "delayed-drop-on-shutdown": {
                    pool.setExecuteExistingDelayedTasksAfterShutdownPolicy(false);
                    svc.schedule(() -> { }, 1, TimeUnit.HOURS);
                    svc.shutdown();
                    return "terminated=" + svc.awaitTermination(1, TimeUnit.SECONDS);
                }
                case "delayed-run-on-shutdown": {
                    svc.schedule(() -> { }, 1, TimeUnit.HOURS);
                    svc.shutdown();
                    return "terminated=" + svc.awaitTermination(1, TimeUnit.MILLISECONDS);
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

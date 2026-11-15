package demo;

import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.time.Duration;

public final class Probe {
    static String bool(boolean value) { return value ? "T" : "F"; }

    public static void main(String[] args) { System.out.println(run()); }

    public static String run() {
        HttpClient.Builder builder = HttpClient.newBuilder();
        HttpClient defaults = builder.build();
        HttpClient.Builder chained = builder.connectTimeout(Duration.ofSeconds(5))
                .version(HttpClient.Version.HTTP_1_1)
                .followRedirects(HttpClient.Redirect.ALWAYS);
        HttpClient client = chained.build();
        HttpRequest.Builder requests = HttpRequest.newBuilder(URI.create("https://example.com/a"));
        HttpRequest request = requests.uri(URI.create("https://example.com/b"))
                .header("X-Test", "yes").GET().build();
        StringBuilder out = new StringBuilder();
        out.append(bool(builder == chained));
        out.append("|").append(defaults.connectTimeout().isEmpty());
        out.append("|").append(defaults.version());
        out.append("|").append(defaults.followRedirects());
        out.append("|").append(client.connectTimeout().orElseThrow().getSeconds());
        out.append("|").append(client.version());
        out.append("|").append(client.followRedirects());
        out.append("|").append(request.uri()).append("|").append(request.method());
        try { HttpRequest.newBuilder().build(); out.append("|no-error"); }
        catch (IllegalStateException e) { out.append("|missing-uri"); }
        try { HttpRequest.newBuilder().header("bad name", "x"); out.append("|no-error"); }
        catch (IllegalArgumentException e) { out.append("|bad-header"); }
        try { HttpRequest.newBuilder().header("X", "bad\nvalue"); out.append("|no-error"); }
        catch (IllegalArgumentException e) { out.append("|bad-value"); }
        try { client.send(request, HttpResponse.BodyHandlers.ofString()); out.append("|no-error"); }
        catch (Exception e) { out.append("|send-").append(e.getClass().getName()); }
        return out.toString();
    }
}

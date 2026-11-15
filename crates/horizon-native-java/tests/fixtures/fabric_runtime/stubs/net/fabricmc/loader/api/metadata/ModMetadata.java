package net.fabricmc.loader.api.metadata;
public interface ModMetadata {
    String getId();
    java.util.Collection<Person> getAuthors();
    java.util.Collection<String> getLicense();
    java.util.Collection<String> getContributors();
    java.util.Collection<String> getProvides();
    java.util.Collection<Object> getDependencies();
}

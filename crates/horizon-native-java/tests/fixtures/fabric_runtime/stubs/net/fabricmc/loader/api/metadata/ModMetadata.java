package net.fabricmc.loader.api.metadata;
import java.util.*; import net.fabricmc.loader.api.Version;
public interface ModMetadata {
 String getType(); String getId(); Collection<String> getProvides(); Version getVersion(); ModEnvironment getEnvironment();
 Collection<ModDependency> getDependencies(); default Collection<ModDependency> getDepends(){return getDependencies();} default Collection<ModDependency> getRecommends(){return getDependencies();} default Collection<ModDependency> getSuggests(){return getDependencies();} default Collection<ModDependency> getConflicts(){return getDependencies();} default Collection<ModDependency> getBreaks(){return getDependencies();}
 String getName(); String getDescription(); Collection<Person> getAuthors(); Collection<Person> getContributors(); ContactInformation getContact(); Collection<String> getLicense(); Optional<String> getIconPath(int i); boolean containsCustomValue(String k); CustomValue getCustomValue(String k); Map<String,CustomValue> getCustomValues(); boolean containsCustomElement(String k);
}

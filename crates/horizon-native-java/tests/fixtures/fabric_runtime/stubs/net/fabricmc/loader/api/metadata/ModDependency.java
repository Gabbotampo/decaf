package net.fabricmc.loader.api.metadata;
import java.util.Collection;
import java.util.List;
import net.fabricmc.loader.api.Version;
import net.fabricmc.loader.api.metadata.version.VersionInterval;
import net.fabricmc.loader.api.metadata.version.VersionPredicate;
public interface ModDependency {
  enum Kind { DEPENDS, RECOMMENDS, SUGGESTS, CONFLICTS, BREAKS }
  Kind getKind(); String getModId(); boolean matches(Version v);
  Collection<VersionPredicate> getVersionRequirements();
  List<VersionInterval> getVersionIntervals();
}

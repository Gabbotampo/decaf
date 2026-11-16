//! Guava 33.6.0 ImmutableMap/HashBiMap parity fixture, compiled with Java 25.
//!
//! Oracle command: javac --release 25 -cp guava-33.6.0-jre.jar GuavaOracle.java
//! followed by java -cp .:guava-33.6.0-jre.jar GuavaOracle.
//!
//! Oracle rows:
//! copy=RegularImmutableMap|{a=1, b=2}|1|true|true
//! copy-one=SingletonImmutableBiMap
//! builder=java.lang.IllegalArgumentException:Multiple entries with same key: a=2 and a=1
//! replace={a=3, b=2}|{3=a, 2=b}
//! bimap={a=1, b=2}|[1, 2]|1|true|java.lang.IllegalArgumentException:value already present: 2|{a=1, c=2}|{1=a, 2=c}|true|{d=2}|[2]|{2=d}
//! nulls=K=java.lang.NullPointerException:null key in entry: null=1|V=java.lang.NullPointerException:null value in entry: a=null
//! views={a=1, b=2}|{1=a, 2=b}|true|{b=2}|{2=b}|true|{}|{}|true
//! entry=a=1|1|{a=9, b=2}|{9=a, 2=b}
//! iter=a|{b=2}|{2=b}
//! iterable=com.google.common.collect.RegularImmutableMap|{a=1, b=2}
//! iterable-bimap=com.google.common.collect.RegularImmutableBiMap|{a=1, b=2}
//! iterable-duplicate=java.lang.IllegalArgumentException:Multiple entries with same key: a=2 and a=1
//! iterable-null-entry=java.lang.NullPointerException:null
//! key-retain=true|{a=1, c=3}|{1=a, 3=c}
//! value-retain=true|{a=1, c=3}|{1=a, 3=c}
//! entry-removeall=true|{b=2}|{2=b}
//! cme=java.util.ConcurrentModificationException:null
//! inverse-cme=java.util.ConcurrentModificationException:null

use horizon_native_java::{NativeJar, Value};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use zip::write::SimpleFileOptions;

const IMMUTABLE_MAP: &str = r#"package com.google.common.collect;
import java.util.*;
public class ImmutableMap<K,V> extends AbstractMap<K,V> {
  public static <K,V> ImmutableMap<K,V> copyOf(Map<? extends K,? extends V> map) { return null; }
  public static <K,V> ImmutableMap<K,V> copyOf(Iterable<? extends Map.Entry<? extends K,? extends V>> entries) { return null; }
  public static <K,V> Builder<K,V> builder() { return null; }
  public static class Builder<K,V> { public Builder<K,V> put(K k,V v){return this;} public ImmutableMap<K,V> build(){return null;} }
  public V get(Object key) { return null; }
  public Set<Map.Entry<K,V>> entrySet() { return null; }
}
"#;
const IMMUTABLE_BIMAP: &str = r#"package com.google.common.collect;
import java.util.*;
public class ImmutableBiMap<K,V> extends ImmutableMap<K,V> implements BiMap<K,V> {
  public static <K,V> ImmutableBiMap<K,V> copyOf(Map<? extends K,? extends V> map) { return null; }
  public static <K,V> ImmutableBiMap<K,V> copyOf(Iterable<? extends Map.Entry<? extends K,? extends V>> entries) { return null; }
  public V forcePut(K k,V v){return null;} public BiMap<V,K> inverse(){return null;}
}
"#;
const BIMAP: &str = r#"package com.google.common.collect;
import java.util.*;
public interface BiMap<K,V> extends Map<K,V> { V forcePut(K k,V v); BiMap<V,K> inverse(); }
"#;
const HASH_BIMAP: &str = r#"package com.google.common.collect;
import java.util.*;
public abstract class HashBiMap<K,V> extends AbstractMap<K,V> implements BiMap<K,V> {
  public static <K,V> HashBiMap<K,V> create(){return null;}
  public V put(K k,V v){return null;} public V forcePut(K k,V v){return null;}
  public BiMap<V,K> inverse(){return null;} public V get(Object k){return null;}
  public boolean containsKey(Object k){return false;} public boolean containsValue(Object v){return false;}
  public V remove(Object k){return null;} public void clear(){} public int size(){return 0;}
  public boolean isEmpty(){return true;} public Set<V> values(){return null;}
  public Set<K> keySet(){return null;} public Set<Map.Entry<K,V>> entrySet(){return null;}
}
"#;
const PROBE: &str = r#"package demo;
import java.util.*; import com.google.common.collect.*;
public final class Probe {
  static String ex(Throwable t){return t.getClass().getName()+":"+t.getMessage();}
  public static String copy() {
    Map<String,Integer> m = new LinkedHashMap<>(); m.put("a",1); m.put("b",2);
    Map<String,Integer> x = ImmutableMap.copyOf(m);
    return x.getClass().getName()+"|"+x+"|"+x.get("a")+"|"+x.containsKey("b")+"|"+(ImmutableMap.copyOf(x)==x);
  }
  public static String copyOne() {
    Map<String,Integer> m = new LinkedHashMap<>(); m.put("a",1);
    return ImmutableMap.copyOf(m).getClass().getName();
  }
  public static String builderDuplicate() {
    try { ImmutableMap.builder().put("a",1).put("a",2).build(); return "ok"; }
    catch (Throwable t) { return ex(t); }
  }
  public static String putReplace() {
    HashBiMap<String,Integer> b=HashBiMap.create(); b.put("a",1); b.put("b",2); b.put("a",3);
    return b+"|"+b.inverse();
  }
  public static String bimap() {
    HashBiMap<String,Integer> b=HashBiMap.create(); b.put("a",1); b.put("b",2);
    String first=b+"|"+b.values()+"|"+b.get("a")+"|"+b.containsValue(2);
    String duplicate; try { b.put("c",2); duplicate="ok"; } catch(Throwable t){duplicate=ex(t);}
    b.forcePut("c",2);
    boolean identity=b.inverse().inverse()==b;
    String force=b+"|"+b.inverse()+"|"+identity;
    b.remove("a"); b.inverse().put(2,"d");
    return first+"|"+duplicate+"|"+force+"|"+b+"|"+b.values()+"|"+b.inverse();
  }
  public static String views() {
    HashBiMap<String,Integer> b=HashBiMap.create(); b.put("a",1); b.put("b",2);
    Set<String> k=b.keySet(); Set<Integer> v=b.values();
    return b+"|"+b.inverse()+"|"+k.remove("a")+"|"+b+"|"+b.inverse()+"|"+v.remove(2)+"|"+b+"|"+b.inverse()+"|"+(b.values() instanceof Set);
  }
  public static String entryView() {
    HashBiMap<String,Integer> b=HashBiMap.create(); b.put("a",1); b.put("b",2);
    Map.Entry<String,Integer> e=b.entrySet().iterator().next();
    return e.getKey()+"="+e.getValue()+"|"+e.setValue(9)+"|"+b+"|"+b.inverse();
  }
  public static String iteratorRemove() {
    HashBiMap<String,Integer> b=HashBiMap.create(); b.put("a",1); b.put("b",2);
    Iterator<String> it=b.keySet().iterator(); String first=it.next(); it.remove();
    return first+"|"+b+"|"+b.inverse();
  }
  public static String bulkViews() {
    HashBiMap<String,Integer> b=HashBiMap.create(); b.put("a",1); b.put("b",2); b.put("c",3);
    boolean kr=b.keySet().retainAll(Set.of("a","c"));
    String key=kr+"|"+b+"|"+b.inverse();
    b=HashBiMap.create(); b.put("a",1); b.put("b",2); b.put("c",3);
    boolean vr=b.values().retainAll(Set.of(1,3));
    String value=vr+"|"+b+"|"+b.inverse();
    b=HashBiMap.create(); b.put("a",1); b.put("b",2); b.put("c",3);
    boolean er=b.entrySet().removeAll(Set.of(Map.entry("a",1), Map.entry("c",3)));
    return key+"|"+value+"|"+er+"|"+b+"|"+b.inverse();
  }
  public static String cmeViews() {
    HashBiMap<String,Integer> b=HashBiMap.create(); b.put("a",1); b.put("b",2);
    Iterator<String> i=b.keySet().iterator(); b.put("c",3);
    String first; try { first=String.valueOf(i.next()); } catch(Throwable t){first=ex(t);}
    b=HashBiMap.create(); b.put("a",1); b.put("b",2);
    Iterator<Integer> j=b.inverse().keySet().iterator(); b.put("c",3);
    String second; try { second=String.valueOf(j.next()); } catch(Throwable t){second=ex(t);}
    return first+"|"+second;
  }

  public static String iterableCopy() {
    Map<String,Integer> m=new LinkedHashMap<>(); m.put("a",1); m.put("b",2);
    return ImmutableMap.copyOf(m.entrySet()).getClass().getName()+"|"+ImmutableMap.copyOf(m.entrySet())
      +"|"+ImmutableBiMap.copyOf(m.entrySet()).getClass().getName()+"|"+ImmutableBiMap.copyOf(m.entrySet());
  }
  public static String iterableErrors() {
    StringBuilder out=new StringBuilder();
    try { List<Map.Entry<String,Integer>> l=new ArrayList<>(); l.add(new AbstractMap.SimpleEntry<>("a",1)); l.add(new AbstractMap.SimpleEntry<>("a",2)); ImmutableMap.copyOf(l); } catch(Throwable t){out.append(ex(t));}
    try { List<Map.Entry<String,Integer>> l=new ArrayList<>(); l.add(null); ImmutableMap.copyOf(l); } catch(Throwable t){out.append("|").append(ex(t));}
    return out.toString();
  }

  public static String nulls() {
    StringBuilder s=new StringBuilder();
    try { Map<String,Integer> m=new LinkedHashMap<>();m.put(null,1);ImmutableMap.copyOf(m); } catch(Throwable t){s.append("K=").append(ex(t));}
    try { Map<String,Integer> m=new LinkedHashMap<>();m.put("a",null);ImmutableMap.copyOf(m); } catch(Throwable t){s.append("|V=").append(ex(t));}
    return s.toString();
  }
}
"#;
fn javac() -> PathBuf {
    std::env::var_os("JAVA_HOME")
        .map(|h| PathBuf::from(h).join("bin/javac"))
        .unwrap_or_else(|| "javac".into())
}
fn files(root: &Path) -> Vec<PathBuf> {
    fs::read_dir(root)
        .unwrap()
        .flat_map(|e| {
            let p = e.unwrap().path();
            if p.is_dir() { files(&p) } else { vec![p] }
        })
        .collect()
}
struct Fixture {
    root: PathBuf,
    jar: NativeJar,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "horizon-guava-bimap-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let classes = root.join("classes");
        let srcs = [
            ("com/google/common/collect/ImmutableMap.java", IMMUTABLE_MAP),
            (
                "com/google/common/collect/ImmutableBiMap.java",
                IMMUTABLE_BIMAP,
            ),
            ("com/google/common/collect/BiMap.java", BIMAP),
            ("com/google/common/collect/HashBiMap.java", HASH_BIMAP),
            ("demo/Probe.java", PROBE),
        ];
        let mut cmd = Command::new(javac());
        cmd.args(["--release", "25", "-d"]).arg(&classes);
        for (name, text) in srcs {
            let p = root.join("src").join(name);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, text).unwrap();
            cmd.arg(p);
        }
        assert!(cmd.status().unwrap().success());
        let jar_path = root.join("fixture.jar");
        let mut zip = zip::ZipWriter::new(fs::File::create(&jar_path).unwrap());
        zip.start_file("fabric.mod.json", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(
            br#"{"id":"guava-bimap-fixture","version":"1","entrypoints":{"main":"demo.Probe"}}"#,
        )
        .unwrap();
        for p in files(&classes) {
            let n = p
                .strip_prefix(&classes)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            zip.start_file(n, SimpleFileOptions::default()).unwrap();
            zip.write_all(&fs::read(p).unwrap()).unwrap();
        }
        zip.finish().unwrap();
        Self {
            root,
            jar: NativeJar::from_path(&jar_path).unwrap(),
        }
    }
    fn text(&self, name: &str) -> String {
        match self
            .jar
            .invoke_method(
                "demo/Probe",
                name,
                "()Ljava/lang/String;",
                None,
                &mut horizon_native_java::StdHost::new(),
            )
            .unwrap()
        {
            Value::Ref(Some(s)) => s,
            v => panic!("{name}: {v:?}"),
        }
    }
}
#[test]
fn immutable_copy_order_same_instance() {
    assert_eq!(
        Fixture::new().text("copy"),
        "com.google.common.collect.RegularImmutableMap|{a=1, b=2}|1|true|true"
    );
}
#[test]
fn immutable_copy_one_entry_uses_the_real_guava_class() {
    assert_eq!(
        Fixture::new().text("copyOne"),
        "com.google.common.collect.SingletonImmutableBiMap"
    );
}

#[test]
fn immutable_duplicate_and_null_messages() {
    let f = Fixture::new();
    assert_eq!(
        f.text("builderDuplicate"),
        "java.lang.IllegalArgumentException:Multiple entries with same key: a=2 and a=1"
    );
    assert_eq!(
        f.text("nulls"),
        "K=java.lang.NullPointerException:null key in entry: null=1|V=java.lang.NullPointerException:null value in entry: a=null"
    );
}
#[test]
fn hash_bimap_views_are_live_and_write_through() {
    let f = Fixture::new();
    assert_eq!(
        f.text("views"),
        "{a=1, b=2}|{1=a, 2=b}|true|{b=2}|{2=b}|true|{}|{}|true"
    );
    assert_eq!(f.text("entryView"), "a=1|1|{a=9, b=2}|{9=a, 2=b}");
    assert_eq!(f.text("iteratorRemove"), "a|{b=2}|{2=b}");
}

#[test]
fn immutable_copy_iterable_matches_guava() {
    let f = Fixture::new();
    assert_eq!(
        f.text("iterableCopy"),
        "com.google.common.collect.RegularImmutableMap|{a=1, b=2}|com.google.common.collect.RegularImmutableBiMap|{a=1, b=2}"
    );
    assert_eq!(
        f.text("iterableErrors"),
        "java.lang.IllegalArgumentException:Multiple entries with same key: a=2 and a=1|java.lang.NullPointerException:null"
    );
}

#[test]
fn hash_bimap_bulk_views_are_live_and_cme_is_exact() {
    let f = Fixture::new();
    assert_eq!(
        f.text("bulkViews"),
        "true|{a=1, c=3}|{1=a, 3=c}|true|{a=1, c=3}|{1=a, 3=c}|true|{b=2}|{2=b}"
    );
    assert_eq!(
        f.text("cmeViews"),
        "java.util.ConcurrentModificationException:null|java.util.ConcurrentModificationException:null"
    );
}

#[test]
fn hash_bimap_put_replaces_inverse() {
    assert_eq!(Fixture::new().text("putReplace"), "{a=3, b=2}|{3=a, 2=b}");
}
#[test]
fn hash_bimap_mutation_inverse_and_order() {
    assert_eq!(
        Fixture::new().text("bimap"),
        "{a=1, b=2}|[1, 2]|1|true|java.lang.IllegalArgumentException:value already present: 2|{a=1, c=2}|{1=a, 2=c}|true|{d=2}|[2]|{2=d}"
    );
}

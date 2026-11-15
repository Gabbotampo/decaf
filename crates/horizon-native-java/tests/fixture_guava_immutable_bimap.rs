//! Guava 33.6.0 ImmutableMap/HashBiMap parity fixture, compiled with Java 25.

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
  public static <K,V> Builder<K,V> builder() { return null; }
  public static class Builder<K,V> { public Builder<K,V> put(K k,V v){return this;} public ImmutableMap<K,V> build(){return null;} }
  public V get(Object key) { return null; }
  public Set<Map.Entry<K,V>> entrySet() { return null; }
}
"#;
const BIMAP: &str = r#"package com.google.common.collect;
import java.util.*;
public interface BiMap<K,V> extends Map<K,V> { V forcePut(K k,V v); BiMap<V,K> inverse(); }
"#;
const HASH_BIMAP: &str = r#"package com.google.common.collect;
import java.util.*;
public class HashBiMap<K,V> extends AbstractMap<K,V> implements BiMap<K,V> {
  public static <K,V> HashBiMap<K,V> create(){return null;}
  public V put(K k,V v){return null;} public V forcePut(K k,V v){return null;}
  public BiMap<V,K> inverse(){return null;} public V get(Object k){return null;}
  public boolean containsKey(Object k){return false;} public boolean containsValue(Object v){return false;}
  public V remove(Object k){return null;} public void clear(){} public int size(){return 0;}
  public boolean isEmpty(){return true;} public Collection<V> values(){return null;}
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

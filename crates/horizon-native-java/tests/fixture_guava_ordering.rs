#[path = "support/javac_fixture.rs"] mod javac_fixture;
use javac_fixture::Fixture;
use horizon_native_java::StdHost;
const ORDERING: &str = r#"package com.google.common.collect; public abstract class Ordering<T> implements java.util.Comparator<T> { public static Ordering natural(){throw new AssertionError();} public Ordering reverse(){throw new AssertionError();} public int compare(T a,T b){throw new AssertionError();} public String toString(){throw new AssertionError();} }"#;
const PROBE: &str = r#"package demo; import com.google.common.collect.Ordering; public final class Probe { public static String rows(){ Ordering n=Ordering.natural(); Ordering r=n.reverse(); StringBuilder s=new StringBuilder(); s.append(n==r.reverse()).append('|').append(n.compare("a","b")).append('|').append(r.compare("a","b")).append('|').append(n.toString()).append('|').append(r.toString()); try { n.compare(null,"a"); } catch(Exception e) { s.append('|').append(e.getClass().getName()).append(':').append(e.getMessage()); } return s.toString(); }}"#;
const ORACLE: &str = include_str!("fixtures/guava_ordering/java25.txt");
#[test] fn guava_ordering_matches_java25() { let f=Fixture::build("guava-ordering", &[("com/google/common/collect/Ordering.java",ORDERING),("demo/Probe.java",PROBE)]); let mut h=f.jar.seed_host(StdHost::new()); assert_eq!(f.text_on(&mut h,"rows"), ORACLE.trim_end()); }

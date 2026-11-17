#[path = "support/scratch.rs"]
mod scratch;
use horizon_native_java::{NativeJar, StdHost, Value};
use std::{fs, io::Write, path::PathBuf, process::Command};
use zip::write::SimpleFileOptions;
const SOURCE: &str = r#"import java.nio.file.*; import java.util.*; public class FilesProbe { public static String rows() throws Exception { Path d=Paths.get("game/files-vfs"); Path sub=d.resolve("sub"); Path p=sub.resolve("x.txt"); List<String> r=new ArrayList<>(); r.add("dir0="+Files.exists(sub, LinkOption.NOFOLLOW_LINKS)); Files.createDirectories(sub); r.add("dir1="+Files.exists(sub, LinkOption.NOFOLLOW_LINKS)+",isDir="+Files.isDirectory(sub, LinkOption.NOFOLLOW_LINKS)+",isReg="+Files.isRegularFile(sub, LinkOption.NOFOLLOW_LINKS)+",read="+Files.isReadable(sub)+",sym="+Files.isSymbolicLink(sub)+",exec="+Files.isExecutable(sub)); Files.writeString(p,"hello"); r.add("file="+Files.exists(p, LinkOption.NOFOLLOW_LINKS)+",isDir="+Files.isDirectory(p, LinkOption.NOFOLLOW_LINKS)+",isReg="+Files.isRegularFile(p, LinkOption.NOFOLLOW_LINKS)+",read="+Files.isReadable(p)+",sym="+Files.isSymbolicLink(p)+",exec="+Files.isExecutable(p)+",text="+Files.readString(p)); r.add("list="+Files.list(sub).count()); Files.delete(p); r.add("afterDelete="+Files.exists(p, LinkOption.NOFOLLOW_LINKS)+",not="+Files.notExists(p)); Path m=d.resolve("missing"); r.add("missing="+Files.exists(m, LinkOption.NOFOLLOW_LINKS)+",not="+Files.notExists(m)+",read="+Files.isReadable(m)+",sym="+Files.isSymbolicLink(m)+",exec="+Files.isExecutable(m)); return String.join("\\n",r); } }"#;
fn javac() -> PathBuf {
    std::env::var_os("JAVA_HOME")
        .map(|h| PathBuf::from(h).join("bin/javac"))
        .unwrap_or_else(|| "javac".into())
}
fn jar() -> NativeJar {
    let root = scratch::Scratch::new("files-vfs");
    let src = root.join("FilesProbe.java");
    fs::write(&src, SOURCE).unwrap();
    fs::create_dir_all(root.join("classes")).unwrap();
    assert!(
        Command::new(javac())
            .args(["--release", "17", "-d"])
            .arg(root.join("classes"))
            .arg(&src)
            .status()
            .unwrap()
            .success()
    );
    let jp = root.join("p.jar");
    let mut z = zip::ZipWriter::new(fs::File::create(&jp).unwrap());
    z.start_file("FilesProbe.class", SimpleFileOptions::default())
        .unwrap();
    z.write_all(&fs::read(root.join("classes/FilesProbe.class")).unwrap())
        .unwrap();
    z.finish().unwrap();
    NativeJar::from_path(&jp).unwrap()
}
#[test]
fn files_vfs_matches_java25_oracle() {
    let j = jar();
    let mut h = StdHost::new();
    let v = j
        .invoke_method_with_args(
            "FilesProbe",
            "rows",
            "()Ljava/lang/String;",
            None,
            &[],
            &mut h,
        )
        .unwrap();
    assert_eq!(
        v,
        Value::Ref(Some(
            include_str!("fixtures/files_vfs/oracle.txt").to_owned()
        ))
    );
}

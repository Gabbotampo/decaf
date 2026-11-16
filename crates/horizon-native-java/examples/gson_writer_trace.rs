use horizon_native_java::{NativeJar, StdHost, Value};
use std::{env, path::Path, process::ExitCode};

fn main() -> ExitCode {
    let Some(path) = env::args().nth(1) else {
        eprintln!("usage: gson_writer_trace <jar>");
        return ExitCode::from(2);
    };
    let jar = match NativeJar::from_path(Path::new(&path)) {
        Ok(jar) => jar,
        Err(error) => {
            eprintln!("load: {error}");
            return ExitCode::from(1);
        }
    };
    let mut host = StdHost::new();
    match jar.invoke_method_with_args(
        "demo/Probe",
        "trace",
        "()Ljava/lang/String;",
        None,
        &[],
        &mut host,
    ) {
        Ok(Value::Ref(Some(trace))) => {
            print!("{trace}");
            ExitCode::SUCCESS
        }
        Ok(value) => {
            eprintln!("trace returned {value:?}");
            ExitCode::from(1)
        }
        Err(error) => {
            eprintln!("trace: {error:?}");
            ExitCode::from(1)
        }
    }
}

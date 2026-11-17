#[path = "support/javac_fixture.rs"] mod javac_fixture;
use horizon_native_java::{StdHost,Value};
use javac_fixture::Fixture;
const PROBE:&str=include_str!("fixtures/log4j_get_logger_factory/demo/Probe.java");
const JAVA25:&str=include_str!("fixtures/log4j_get_logger_factory/java25.txt");
#[test]
fn log4j_factory(){
 let logger=r#"package org.apache.logging.log4j; public interface Logger { String getName(); Object getMessageFactory(); }"#;
 let mf=r#"package org.apache.logging.log4j.message; public interface MessageFactory { Object newMessage(String s); Object newMessage(Object o); Object newMessage(String s,Object... a); }"#;
 let m2=r#"package org.apache.logging.log4j.message; public abstract class AbstractMessageFactory implements MessageFactory {}"#;
 let fm=r#"package org.apache.logging.log4j.message; public class FormattedMessageFactory implements MessageFactory {}"#;
 let lm=r#"package org.apache.logging.log4j; public class LogManager { public static Logger getLogger(String n, MessageFactory f){return null;} public static Logger getLogger(String n){return null;} }"#;
 let f=Fixture::build("log4j_get_logger_factory",&[("demo/Probe.java",PROBE),("org/apache/logging/log4j/Logger.java",logger),("org/apache/logging/log4j/LogManager.java",lm),("org/apache/logging/log4j/message/MessageFactory.java",mf),("org/apache/logging/log4j/message/AbstractMessageFactory.java",m2),("org/apache/logging/log4j/message/FormattedMessageFactory.java",fm)]);
 let mut h=f.jar.seed_host(StdHost::new()); let a=f.jar.invoke_method_with_args("demo/Probe","rows","()Ljava/lang/String;",None,&[],&mut h).unwrap(); assert_eq!(a,Value::Ref(Some(JAVA25.trim_end().into()))); assert!(h.unknown_host_calls().is_empty(),"{:?}",h.unknown_host_calls());
}

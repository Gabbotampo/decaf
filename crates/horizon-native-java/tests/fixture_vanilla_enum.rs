use std::collections::BTreeMap;
#[test]
fn captured_rows_are_complete_and_well_formed() {
 let text=include_str!("fixtures/vanilla_enum/java25.txt"); let mut n=0; let mut ord=BTreeMap::new();
 for line in text.lines() { if line.starts_with("CLASS "){continue;} let p:Vec<_>=line.split('|').collect(); assert_eq!(p.len(),4); let o:u32=p[1].parse().unwrap(); assert!(ord.insert((p[0].to_owned(),o),()).is_none()); n+=1; }
 assert_eq!(n,16);
}

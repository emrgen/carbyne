use serde::Serialize;

pub fn print_yaml(v: impl Serialize) {
    let yaml = serde_yaml::to_string(&v).unwrap();
    println!("---\n{}", yaml);
}

// macro_rules! print_yaml {
//     ($m:expr,$v:expr) => {
//         println!("{}", $m);
//         print_yaml_nl($v);
//     };
//     ($v:expr) => {
//         print_yaml_nl($v);
//     };
//     () => {};
// }

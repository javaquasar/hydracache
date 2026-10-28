use std::collections::HashMap;
use zbus::{
    Type,
    wire::{LE, serialized::Context, to_bytes},
};

#[test]
fn struct_with_hashmap() {
    use serde::{Deserialize, Serialize};

    let mut hmap = HashMap::new();
    hmap.insert("key".into(), "value".into());

    #[derive(Type, Deserialize, Serialize, PartialEq, Debug)]
    struct Foo {
        hmap: HashMap<String, String>,
    }

    let foo = Foo { hmap };
    assert_eq!(Foo::SIGNATURE, "(a{ss})");

    let ctxt = Context::new(LE, 0);
    let encoded = to_bytes(ctxt, &(&foo, 1)).unwrap();
    let f: Foo = encoded.deserialize().unwrap().0;
    assert_eq!(f, foo);
}

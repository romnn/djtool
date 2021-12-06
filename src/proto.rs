pub mod djtool {
    use serde::{Deserialize, Serialize};
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/proto/proto.djtool.rs"
    ));
}

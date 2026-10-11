//! The UniFFI binding generator, pinned to this crate's uniffi version so the generated Swift and
//! Kotlin always match the scaffolding compiled into the library. `build.sh` runs it.
fn main() {
    uniffi::uniffi_bindgen_main()
}

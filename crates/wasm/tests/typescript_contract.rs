//! Pins the split between generated and hand-written TypeScript declarations.
//!
//! `crates/wasm/src/lib.rs` carries `#![cfg(target_arch = "wasm32")]`, so on the
//! host target — the one `cargo test` and CI build — that module is empty. These
//! pins therefore read the **source text** rather than the compiled item, which
//! is what lets them run in the ordinary workspace test job without a wasm
//! toolchain. They are cheap, and the invariant they hold is not.
//!
//! # The invariant
//!
//! `wasm_bindgen` generates a `.d.ts` declaration for every exported symbol,
//! derived from the Rust signature. The `typescript_custom_section` exists only
//! to describe the *interior* of values typed `JsValue`, which `wasm_bindgen`
//! can render no better than `any`. Those two roles must not overlap.
//!
//! When they did, TypeScript did not report a conflict — it merged, and each
//! merge mode was its own defect. A stale by-hand `buildSignDocBytes`
//! declaration became an **overload**, so the eight-argument call that
//! `nonce: Vec<u8>` was made non-optional to forbid still compiled: the
//! signature covered a nonce-less preimage while a fabricated nonce went on the
//! wire, rewritable by any observer. By-hand `TxBuilderWasm` / `VcClaimBuilder`
//! classes collided outright (`TS2300`), which made the package's own `.d.ts`
//! invalid and forced consumers onto `skipLibCheck: true` — which is exactly
//! what kept anyone from seeing the overload. One root cause, two defects, and
//! the second concealed the first.
//!
//! Neither is reachable while these two pins hold.

/// The crate source, read at compile time so the pins do not depend on the
/// working directory a test runner happens to use.
const LIB_RS: &str = include_str!("../src/lib.rs");
const BINDINGS_RS: &str = include_str!("../src/bindings.rs");

/// The body of the `TS_TYPES` raw-string literal.
///
/// Panics rather than returning an `Option`: every caller is a pin whose only
/// sensible response to "the section is gone or was reshaped" is to fail, and a
/// pin that skipped itself when it could not find its subject would be the
/// silent no-op this whole area already suffered from once.
fn ts_custom_section() -> &'static str {
    const OPEN: &str = "const TS_TYPES: &str = r#\"";
    const CLOSE: &str = "\"#;";

    let start = LIB_RS
        .find(OPEN)
        .expect("TS_TYPES raw-string literal not found in lib.rs — did its declaration change?")
        + OPEN.len();
    let len = LIB_RS[start..]
        .find(CLOSE)
        .expect("TS_TYPES raw-string literal is unterminated");
    &LIB_RS[start..start + len]
}

/// Every type named by an `unchecked_return_type` / `unchecked_param_type`
/// attribute in `bindings.rs`, in source order.
fn unchecked_type_names() -> Vec<&'static str> {
    const ATTRS: [&str; 2] = ["unchecked_return_type = \"", "unchecked_param_type = \""];

    let mut names = Vec::new();
    for attr in ATTRS {
        let mut rest = BINDINGS_RS;
        while let Some(at) = rest.find(attr) {
            rest = &rest[at + attr.len()..];
            let end = rest
                .find('"')
                .expect("unterminated unchecked_*_type attribute value");
            names.push(&rest[..end]);
            rest = &rest[end..];
        }
    }
    names
}

/// The custom section declares shapes only — never a symbol `wasm_bindgen`
/// already generates from a Rust signature.
///
/// `export interface` is the only admissible form. A function or class here is
/// a second declaration of something that already has one, and TypeScript
/// merges rather than rejects it.
#[test]
fn ts_custom_section_declares_no_symbol_wasm_bindgen_generates() {
    let offenders: Vec<&str> = ts_custom_section()
        .lines()
        .map(str::trim_start)
        .filter(|line| line.starts_with("export function") || line.starts_with("export class"))
        .collect();

    assert!(
        offenders.is_empty(),
        "TS_TYPES re-declares {} symbol(s) wasm_bindgen already generates:\n  {}\n\n\
         Delete them. A hand-written copy of a generated declaration drifts, and \
         TypeScript merges the drift instead of reporting it: functions become an \
         overload set that re-admits the old call shape, classes collide and force \
         consumers onto skipLibCheck. To give a `JsValue` a precise type, declare an \
         `export interface` here and name it from an `unchecked_return_type` / \
         `unchecked_param_type` attribute on the Rust signature.",
        offenders.len(),
        offenders.join("\n  "),
    );
}

/// Every type an `unchecked_*_type` attribute names is actually declared.
///
/// The attribute is *unchecked* by design — `wasm_bindgen` writes the name
/// through verbatim without ever resolving it. A typo, or an interface renamed
/// on one side only, therefore emits a `.d.ts` that references a type nobody
/// declares, and the package stops typechecking for every consumer while this
/// crate still builds green. That is the same shape of failure as the one
/// above: the artifact is broken, the producing repo cannot tell.
#[test]
fn every_unchecked_type_attribute_names_a_declared_interface() {
    let section = ts_custom_section();
    let names = unchecked_type_names();

    assert!(
        !names.is_empty(),
        "no unchecked_*_type attributes found in bindings.rs — the JsValue returns \
         have gone back to `any`, or the attribute spelling changed"
    );

    let undeclared: Vec<&str> = names
        .iter()
        .copied()
        .filter(|name| !section.contains(&format!("export interface {name} ")))
        .collect();

    assert!(
        undeclared.is_empty(),
        "unchecked_*_type names a type TS_TYPES does not declare: {}\n\n\
         wasm_bindgen copies the name into the .d.ts without resolving it, so this \
         ships a package that references an undeclared type.",
        undeclared.join(", "),
    );
}

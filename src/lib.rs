// Max clippy pedanticness.
#![deny(
    // Try not to use `.unwrap()`. If you have confirmed the invariant or it's difficult to propagate an
    // error properly, use `.expect()` with an explanation of the invariant.
    clippy::unwrap_used,
    // Using this macro for debugging is fine, but it shouldn't be checked in.
    clippy::dbg_macro,
    // This is an `.unwrap()` in a different guise.
    clippy::indexing_slicing,
    // Project doesn't use mod.rs anywhere, so enforce consistency.
    clippy::mod_module_files,
    // Splitting the implementation of a type makes the code harder to navigate.
    clippy::multiple_inherent_impl,
    // Separating literals is more readable.
    clippy::unseparated_literal_suffix,
    // `.to_owned()` is clearer for str -> String conversions.
    clippy::str_to_string,
    // `.clone()` is clearer from String -> String.
    clippy::string_to_string,
    // This macro should not be present in production code
    clippy::todo,
    // Documenting why unsafe things are okay is useful.
    clippy::undocumented_unsafe_blocks,
    // Removing these improves readability.
    clippy::unnecessary_self_imports,
    // Improves readability.
    clippy::unneeded_field_pattern,
    // If we can return a result, we should.
    clippy::unwrap_in_result,
    // Cargo manifest lints.
    clippy::cargo,
    // May regret adding this.
    clippy::pedantic,
)]
#![allow(
    // This is covered by other lints anyway, and we want to allow assert! for tests.
    clippy::panic_in_result_fn,
    // Done by downstream crates, not much that can be done for it.
    clippy::multiple_crate_versions,
    // Mostly not using this as a shared library.
    clippy::missing_errors_doc,
    // Not worth it IMHO.
    clippy::case_sensitive_file_extension_comparisons,
    // I find this often more readable.
    clippy::module_name_repetitions,
    // Not usually worth fixing.
    clippy::needless_pass_by_value,
)]

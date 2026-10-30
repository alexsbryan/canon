// SPDX-License-Identifier: AGPL-3.0-or-later
//! canon's contract, as files in `contract/` that the tests hold the code to.
//!
//! **The declaration is in the tree, and a change to it is a diff.** What
//! canon-core offers (`api.txt`), what the CLI takes (`cli.txt`), and what the
//! fixtures decide and mint (`records/`) are checked in. A test that finds the
//! code and the file apart fails in the commit that moved them apart. When the
//! change is meant, `CANON_BLESS=1 cargo test` rewrites the files, and an
//! agent's edit or bless of `contract/` asks the operator first
//! (`.claude/hooks/contract-guard.py`): the decision is made at the edit, not
//! by a later verdict.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use quote::ToTokens;
use syn::visit_mut::VisitMut;
use syn::{Item, Visibility};

/// The repository root, where `contract/` lives.
pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Hold `actual` to the checked-in `contract/<name>`, or with `CANON_BLESS`
/// set, write it there.
pub fn golden(name: &str, actual: &str) {
    let path = root().join("contract").join(name);
    if std::env::var_os("CANON_BLESS").is_some() {
        std::fs::create_dir_all(path.parent().expect("contract/ has a parent")).unwrap();
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_default();
    if expected != actual {
        let gone = expected
            .lines()
            .filter(|l| !actual.lines().any(|a| a == *l));
        let new = actual
            .lines()
            .filter(|l| !expected.lines().any(|e| e == *l));
        let diff: Vec<String> = gone
            .map(|l| format!("- {l}"))
            .chain(new.map(|l| format!("+ {l}")))
            .take(40)
            .collect();
        panic!(
            "contract/{name} is not what the code does:\n{}\n\nIf the change is meant, \
             `CANON_BLESS=1 cargo test` rewrites it, and the diff is the record.",
            diff.join("\n")
        );
    }
}

/// [`surface`] as `contract/api.txt` holds it: one item per line.
pub fn api(src: &Path) -> String {
    let surface = surface(src).unwrap_or_else(|e| panic!("reading {}: {e}", src.display()));
    surface.iter().map(|(k, v)| format!("{k}  {v}\n")).collect()
}

struct StripDocs;

macro_rules! strip_visit {
    ($($method:ident: $node:ident),* $(,)?) => {$(
        fn $method(&mut self, node: &mut syn::$node) {
            node.attrs.retain(|a| !a.path().is_ident("doc"));
            syn::visit_mut::$method(self, node);
        }
    )*};
}

impl VisitMut for StripDocs {
    strip_visit! {
        visit_item_struct_mut: ItemStruct, visit_item_enum_mut: ItemEnum,
        visit_item_trait_mut: ItemTrait, visit_item_fn_mut: ItemFn,
        visit_item_const_mut: ItemConst, visit_item_type_mut: ItemType,
        visit_item_impl_mut: ItemImpl, visit_impl_item_fn_mut: ImplItemFn,
        visit_trait_item_fn_mut: TraitItemFn, visit_variant_mut: Variant,
        visit_field_mut: Field,
    }

    fn visit_signature_mut(&mut self, signature: &mut syn::Signature) {
        for input in &mut signature.inputs {
            if let syn::FnArg::Typed(argument) = input {
                *argument.pat = syn::parse_quote!(_);
            }
        }
        syn::visit_mut::visit_signature_mut(self, signature);
    }
}

fn public(vis: &Visibility) -> bool {
    matches!(vis, Visibility::Public(_))
}

/// canon-core's public surface: each exported name, module, and public item
/// of a public module, with its signature or definition, docs stripped.
pub fn surface(src: &Path) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let root = syn::parse_file(&std::fs::read_to_string(src.join("lib.rs"))?)?;
    let mut surface = BTreeMap::new();
    for item in &root.items {
        if let Item::Use(export) = item {
            if public(&export.vis) {
                exports(&export.tree, "", &mut surface);
            }
        }
        let uncovered = match item {
            Item::Fn(value) => public(&value.vis),
            Item::Struct(value) => public(&value.vis),
            Item::Enum(value) => public(&value.vis),
            Item::Trait(value) => public(&value.vis),
            Item::Type(value) => public(&value.vis),
            Item::Const(value) => public(&value.vis),
            Item::Static(value) => public(&value.vis),
            Item::Union(value) => public(&value.vis),
            Item::Impl(_) | Item::Macro(_) => true,
            _ => false,
        };
        if uncovered {
            return Err("new root-level definitions require an inspector extension".into());
        }
    }
    for item in root.items {
        let Item::Mod(module) = item else { continue };
        if !public(&module.vis) {
            continue;
        }
        if module.content.is_some() {
            return Err("inline public modules require an explicit inspector extension".into());
        }
        let name = module.ident.to_string();
        surface.insert(format!("module::{name}"), "public module".into());
        let mut file = syn::parse_file(&std::fs::read_to_string(src.join(format!("{name}.rs")))?)?;
        StripDocs.visit_file_mut(&mut file);
        for item in file.items {
            match item {
                Item::Struct(value) if public(&value.vis) => {
                    surface.insert(
                        format!("{name}::{}", value.ident),
                        value.to_token_stream().to_string(),
                    );
                }
                Item::Enum(value) if public(&value.vis) => {
                    surface.insert(
                        format!("{name}::{}", value.ident),
                        value.to_token_stream().to_string(),
                    );
                }
                Item::Fn(value) if public(&value.vis) => {
                    surface.insert(
                        format!("{name}::{}", value.sig.ident),
                        value.sig.to_token_stream().to_string(),
                    );
                }
                Item::Const(value) if public(&value.vis) => {
                    surface.insert(
                        format!("{name}::{}", value.ident),
                        value.to_token_stream().to_string(),
                    );
                }
                Item::Type(value) if public(&value.vis) => {
                    surface.insert(
                        format!("{name}::{}", value.ident),
                        value.to_token_stream().to_string(),
                    );
                }
                Item::Trait(mut value) if public(&value.vis) => {
                    for member in &mut value.items {
                        if let syn::TraitItem::Fn(method) = member {
                            method.default = None;
                            method.semi_token = Some(Default::default());
                        }
                    }
                    surface.insert(
                        format!("{name}::{}", value.ident),
                        value.to_token_stream().to_string(),
                    );
                }
                Item::Impl(value) => {
                    let ty = value.self_ty.to_token_stream().to_string();
                    if let Some((_, path, _)) = &value.trait_ {
                        surface.insert(
                            format!("{name}::{ty}::impl::{}", path.to_token_stream()),
                            value.generics.to_token_stream().to_string(),
                        );
                    }
                    for member in value.items {
                        if let syn::ImplItem::Fn(method) = member {
                            if public(&method.vis) {
                                surface.insert(
                                    format!("{name}::{ty}::{}", method.sig.ident),
                                    method.sig.to_token_stream().to_string(),
                                );
                            }
                        }
                    }
                }
                Item::Macro(_) => {
                    return Err("top-level macro expansion is not covered by this inspector".into())
                }
                Item::Mod(value) if public(&value.vis) => {
                    return Err("nested public modules are not covered by this inspector".into())
                }
                Item::Use(value) if public(&value.vis) => {
                    return Err(
                        "module-local public re-exports are not covered by this inspector".into(),
                    )
                }
                Item::Union(value) if public(&value.vis) => {
                    return Err("public unions are not covered by this inspector".into())
                }
                Item::Static(value) if public(&value.vis) => {
                    return Err("public statics are not covered by this inspector".into())
                }
                _ => {}
            }
        }
    }
    Ok(surface)
}

fn exports(tree: &syn::UseTree, prefix: &str, surface: &mut BTreeMap<String, String>) {
    match tree {
        syn::UseTree::Path(path) => {
            exports(&path.tree, &format!("{prefix}{}::", path.ident), surface)
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                exports(item, prefix, surface);
            }
        }
        syn::UseTree::Name(name) => {
            surface.insert(
                format!("export::{}", name.ident),
                format!("{prefix}{}", name.ident),
            );
        }
        syn::UseTree::Rename(rename) => {
            surface.insert(
                format!("export::{}", rename.rename),
                format!("{prefix}{}", rename.ident),
            );
        }
        syn::UseTree::Glob(_) => {
            surface.insert(format!("export::{prefix}*"), "glob".into());
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn canon_cores_public_surface_is_the_checked_in_one() {
        super::golden(
            "api.txt",
            &super::api(&super::root().join("crates/canon-core/src")),
        );
    }
}

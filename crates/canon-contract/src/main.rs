// SPDX-License-Identifier: AGPL-3.0-or-later
//! Development-only syntax inspection. syn supplies the parser; compatibility
//! is established by baseline-owned consumers and tests, not by syntax alone.

use std::collections::BTreeMap;
use std::path::Path;

use quote::{quote, ToTokens};
use syn::spanned::Spanned;
use syn::visit_mut::VisitMut;
use syn::{Fields, Item, Visibility};

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

fn inspect(src: &Path) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let root = syn::parse_file(&std::fs::read_to_string(src.join("lib.rs"))?)?;
    let mut surface = BTreeMap::new();
    let mut consumer = String::from("#![allow(dead_code, unused_imports)]\n");
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
        consumer.push_str(&format!("mod {name} {{\nuse canon_core::*;\nuse canon_core::{name}::*;\nuse std::collections::{{BTreeMap, BTreeSet}};\n"));
        for item in file.items {
            match item {
                Item::Struct(value) if public(&value.vis) => {
                    surface.insert(
                        format!("{name}::{}", value.ident),
                        value.to_token_stream().to_string(),
                    );
                    let ident = &value.ident;
                    let module_name = &module.ident;
                    let (generics, types, clause) = value.generics.split_for_impl();
                    let mut accesses = Vec::new();
                    for (i, field) in value.fields.iter().enumerate() {
                        if public(&field.vis) {
                            let member = field
                                .ident
                                .clone()
                                .map(syn::Member::Named)
                                .unwrap_or_else(|| syn::Member::Unnamed(syn::Index::from(i)));
                            let ty = &field.ty;
                            accesses.push(quote!(let _: &#ty = &value.#member;));
                        }
                    }
                    let function = syn::Ident::new(&format!("fields_{ident}"), ident.span());
                    consumer.push_str(
                        &quote!(#[allow(non_snake_case)] fn #function #generics
                        (value: canon_core::#module_name::#ident #types) #clause { #(#accesses)* })
                        .to_string()
                        .replace("crate ::", "canon_core ::"),
                    );
                }
                Item::Enum(value) if public(&value.vis) => {
                    surface.insert(
                        format!("{name}::{}", value.ident),
                        value.to_token_stream().to_string(),
                    );
                    let ident = &value.ident;
                    let module_name = &module.ident;
                    let (generics, types, clause) = value.generics.split_for_impl();
                    let arms: Vec<_> = value
                        .variants
                        .iter()
                        .map(|variant| {
                            let variant_name = &variant.ident;
                            let fields = match &variant.fields {
                                Fields::Named(fields) => {
                                    let names =
                                        fields.named.iter().map(|f| f.ident.as_ref().unwrap());
                                    quote!({ #(#names: _,)* })
                                }
                                Fields::Unnamed(fields) => {
                                    let ignore = fields.unnamed.iter().map(|_| quote!(_));
                                    quote!((#(#ignore,)*))
                                }
                                Fields::Unit => quote!(),
                            };
                            quote!(canon_core::#module_name::#ident::#variant_name #fields => {})
                        })
                        .collect();
                    let wildcard = value
                        .attrs
                        .iter()
                        .any(|a| a.path().is_ident("non_exhaustive"))
                        .then(|| quote!(_ => {}));
                    let function = syn::Ident::new(&format!("variants_{ident}"), ident.span());
                    consumer.push_str(
                        &quote!(#[allow(non_snake_case)] fn #function #generics
                        (value: canon_core::#module_name::#ident #types) #clause {
                            match value { #(#arms,)* #wildcard }
                        })
                        .to_string(),
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
        consumer.push_str("\n}\n");
    }
    consumer.push_str("\n#[test] fn baseline_consumer_compiles() {}\n");
    Ok(serde_json::json!({"surface": surface, "consumer": consumer}))
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

fn test_item(item: &Item) -> bool {
    let attrs = match item {
        Item::Mod(value) => &value.attrs,
        Item::ExternCrate(value) => &value.attrs,
        _ => return false,
    };
    attrs
        .iter()
        .any(|a| a.to_token_stream().to_string().replace(' ', "") == "#[cfg(test)]")
}

fn range(source: &str, item: &Item) -> std::ops::Range<usize> {
    let span = item.span();
    let offset = |position: proc_macro2::LineColumn| {
        source
            .split_inclusive('\n')
            .take(position.line - 1)
            .map(str::len)
            .sum::<usize>()
            + position.column
    };
    offset(span.start())..offset(span.end())
}

fn transplant(baseline: &str, candidate: &str) -> syn::Result<String> {
    let old = syn::parse_file(baseline)?;
    let new = syn::parse_file(candidate)?;
    let mut output = candidate.to_string();
    let mut removals: Vec<_> = new
        .items
        .iter()
        .filter(|i| test_item(i))
        .map(|i| range(candidate, i))
        .collect();
    removals.sort_by_key(|r| r.start);
    for removal in removals.into_iter().rev() {
        output.replace_range(removal, "");
    }
    for item in old.items.iter().filter(|i| test_item(i)) {
        output.push('\n');
        output.push_str(&baseline[range(baseline, item)]);
        output.push('\n');
    }
    Ok(output)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [command, path] if command == "inspect" => println!("{}", inspect(Path::new(path))?),
        [command, baseline, candidate, output] if command == "transplant" => {
            let merged = transplant(&std::fs::read_to_string(baseline)?, &std::fs::read_to_string(candidate)?)?;
            std::fs::write(output, merged)?;
        }
        _ => return Err("usage: canon-contract inspect <src> | transplant <baseline.rs> <candidate.rs> <output.rs>".into()),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_cannot_replace_the_baseline_tests() {
        let old = "fn implementation() {}\n#[cfg(test)] mod tests { #[test] fn law() { assert!(false); } }";
        let new =
            "fn implementation() { /* new */ }\n#[cfg(test)] mod tests { #[test] fn law() {} }";
        let merged = transplant(old, new).unwrap();
        assert!(merged.contains("/* new */"));
        assert!(merged.contains("assert!(false)"));
        assert!(!merged.contains("fn law() {}"));
    }

    #[test]
    fn transplant_keeps_production_text_and_handles_unicode_before_tests() {
        let old = "// café\n#[cfg(test)] extern crate std;\n#[cfg(test)] mod tests {}";
        let new = "// café\nfn live() {}";
        let merged = transplant(old, new).unwrap();
        assert!(merged.starts_with(new));
        syn::parse_file(&merged).unwrap();
        assert!(merged.contains("extern crate std;"));
    }
}

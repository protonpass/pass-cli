/*
 *  Copyright (c) 2026 Proton AG
 *  This file is part of Proton AG and Proton Pass.
 *
 *  Proton Pass is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Proton Pass is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Proton Pass.  If not, see <https://www.gnu.org/licenses/>.
 *
 */

//! `#[sdk_export]`: marks items of the `pass` crate as part of the public SDK
//! surface, so SDK crates (`pass-web-sdk`, ...) don't have to re-declare them.
//!
//! The annotations are target-agnostic; each FFI target is a backend that
//! turns them into target-specific code behind that target's cfg (set by
//! `pass`'s build script from its cargo features):
//! - [`wasm`] (`wasm_runtime`): wasm-bindgen exports, for `pass-web-sdk`.
//! - [`uniffi`] (`uniffi_runtime`): uniffi exports, for `pass-mobile-sdk`.
//!
//! Supported items:
//! - **structs/enums**: get the derives the target needs, keeping the type's
//!   serde representation (for wasm: `Tsify`, plus serde if it doesn't derive
//!   it already, plus TS types for foreign fields such as `jiff` datetimes;
//!   for uniffi: `Record`/`Enum`). Upstream `proton-pass-types` types are
//!   already exported under both targets, so they can be used in signatures
//!   and fields without annotating anything.
//!   Struct fields marked `#[sdk_export(default)]` can be omitted by SDK
//!   callers and take their type's `Default` value (for wasm: `serde(default)`;
//!   for uniffi: `uniffi(default)`).
//! - **free functions**: get an exported wrapper, named in the target's
//!   convention (`fooBar` for JS, unchanged for uniffi).
//! - **`impl Foo<C>` blocks**: every method inside marked `#[sdk_export]`
//!   gets a wrapper on `crate::sdk::Foo`, a concrete handle that holds a
//!   `Foo<SdkContext>` in an `inner` field. Unmarked methods stay internal.
//!
//! Exported functions must take `&self` (methods) and plain-identifier
//! arguments, and may be `async` and return `anyhow::Result<T>` or `T`.

mod types;
mod uniffi;
mod wasm;

use proc_macro::TokenStream;
use quote::quote;
use syn::{Attribute, ImplItem, Item, ItemImpl, parse_macro_input, spanned::Spanned};

#[proc_macro_attribute]
pub fn sdk_export(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        let attr = proc_macro2::TokenStream::from(attr);
        return syn::Error::new(attr.span(), "`sdk_export` takes no arguments")
            .to_compile_error()
            .into();
    }

    let item = parse_macro_input!(item as Item);
    let result = match item {
        Item::Struct(_) | Item::Enum(_) => expand_type(item),
        Item::Fn(item_fn) => expand_fn(item_fn),
        Item::Impl(item_impl) => expand_impl(item_impl),
        other => Err(syn::Error::new(
            other.span(),
            "`sdk_export` supports structs, enums, functions and impl blocks",
        )),
    };

    result.unwrap_or_else(syn::Error::into_compile_error).into()
}

fn expand_type(mut item: Item) -> syn::Result<proc_macro2::TokenStream> {
    expand_field_defaults(&mut item)?;
    let wasm = wasm::prepare_type(&mut item);
    let uniffi = uniffi::prepare_type(&item);
    // `uniffi` goes first: for id newtypes it is a standalone item, and must not
    // sit between `wasm`'s attributes and the type they apply to.
    Ok(quote! {
        #uniffi
        #wasm
        #item
    })
}

/// Replaces `#[sdk_export(default)]` on struct fields with each target's
/// attribute for a field that falls back to its type's `Default`.
fn expand_field_defaults(item: &mut Item) -> syn::Result<()> {
    let (attrs, fields): (&[Attribute], Vec<&mut syn::Field>) = match item {
        Item::Struct(s) => (&s.attrs, s.fields.iter_mut().collect()),
        Item::Enum(e) => {
            for field in e.variants.iter_mut().flat_map(|v| v.fields.iter_mut()) {
                if let Some(attr) = field.attrs.iter().find(|a| a.path().is_ident("sdk_export")) {
                    return Err(syn::Error::new(
                        attr.span(),
                        "`#[sdk_export(default)]` is only supported on struct fields",
                    ));
                }
            }
            return Ok(());
        }
        _ => unreachable!("only called for structs and enums"),
    };
    let derives_serde = wasm::derives_serde(attrs);

    for field in fields {
        let mut is_default = false;
        let mut error = None;
        field.attrs.retain(|attr| {
            if !attr.path().is_ident("sdk_export") {
                return true;
            }
            match attr.parse_args::<syn::Ident>() {
                Ok(arg) if arg == "default" => is_default = true,
                _ => {
                    error = Some(syn::Error::new(
                        attr.span(),
                        "expected `#[sdk_export(default)]` on struct fields",
                    ))
                }
            }
            false
        });
        if let Some(error) = error {
            return Err(error);
        }
        if is_default {
            field.attrs.push(wasm::default_field_attr(derives_serde));
            field.attrs.push(uniffi::default_field_attr());
        }
    }
    Ok(())
}

fn expand_fn(item_fn: syn::ItemFn) -> syn::Result<proc_macro2::TokenStream> {
    let wasm = wasm::export_fn(&item_fn)?;
    let uniffi = uniffi::export_fn(&item_fn)?;
    Ok(quote! {
        #item_fn

        #[cfg(wasm_runtime)]
        #wasm

        #[cfg(uniffi_runtime)]
        const _: () = {
            #uniffi
        };
    })
}

fn expand_impl(mut item_impl: ItemImpl) -> syn::Result<proc_macro2::TokenStream> {
    if item_impl.trait_.is_some() {
        return Err(syn::Error::new(
            item_impl.span(),
            "`sdk_export` can't be used on trait impls",
        ));
    }

    let self_ident = match &*item_impl.self_ty {
        syn::Type::Path(path) => path.path.segments.last().map(|s| s.ident.clone()),
        _ => None,
    }
    .ok_or_else(|| syn::Error::new(item_impl.self_ty.span(), "unsupported impl target"))?;

    let mut wasm_wrappers = Vec::new();
    let mut uniffi_wrappers = Vec::new();
    for impl_item in &mut item_impl.items {
        if let ImplItem::Fn(method) = impl_item
            && take_marker(&mut method.attrs)
        {
            wasm_wrappers.push(wasm::export_method(method)?);
            uniffi_wrappers.push(uniffi::export_method(method)?);
        }
    }

    if wasm_wrappers.is_empty() {
        return Err(syn::Error::new(
            item_impl.span(),
            "no methods are marked `#[sdk_export]` in this impl block",
        ));
    }

    // A named (hidden) module would clash across the many impl blocks of the
    // same type, so the wrappers go in an anonymous const. wasm-bindgen needs
    // the impl target as a bare identifier (it's also the JS class name), so
    // the SDK handle is imported there, shadowing the core type. uniffi
    // accepts any number of exported impl blocks for the same object.
    Ok(quote! {
        #item_impl

        #[cfg(wasm_runtime)]
        const _: () = {
            use ::core::result::Result;
            use ::wasm_bindgen::prelude::wasm_bindgen;
            use crate::sdk::#self_ident;

            #[wasm_bindgen]
            impl #self_ident {
                #(#wasm_wrappers)*
            }
        };

        #[cfg(uniffi_runtime)]
        const _: () = {
            use crate::sdk::#self_ident;

            #[::uniffi::export(async_runtime = "tokio")]
            impl #self_ident {
                #(#uniffi_wrappers)*
            }
        };
    })
}

/// Removes `#[sdk_export]` from a method's attributes, returning whether it was there.
fn take_marker(attrs: &mut Vec<Attribute>) -> bool {
    let before = attrs.len();
    attrs.retain(|attr| !attr.path().is_ident("sdk_export"));
    attrs.len() != before
}

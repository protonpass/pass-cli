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

//! The wasm backend: expands `#[sdk_export]` into wasm-bindgen exports.
//!
//! Native types cross as-is; domain types cross as `tsify::Ts<T>` (typed in
//! the generated `.d.ts` through their `Tsify` derive); errors become
//! `JsError`s through `crate::sdk::js_error`. All generated code is emitted
//! behind `#[cfg(wasm_runtime)]` by the caller.

use crate::types::{Shape, shape, to_camel_case, unwrap_result};
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::spanned::Spanned;
use syn::{
    Attribute, FnArg, ImplItemFn, Item, ItemFn, Pat, ReceiverKind, ReturnType, Signature, Type,
    parse_quote,
};

/// Adds what an exported struct/enum needs: `Tsify`, serde if the type
/// doesn't derive it yet, and TS type overrides for foreign field types.
///
/// The type's serde representation is kept as is (Rust field names), which
/// is also what the upstream `proton-pass-types` types use, so the whole SDK
/// surface is consistent and existing types (cache entries, CLI JSON output)
/// don't change shape.
pub fn prepare_type(item: &mut Item) -> TokenStream {
    let (attrs, fields): (_, Vec<&mut syn::Field>) = match item {
        Item::Struct(s) => (&s.attrs, s.fields.iter_mut().collect()),
        Item::Enum(e) => (
            &e.attrs,
            e.variants
                .iter_mut()
                .flat_map(|v| v.fields.iter_mut())
                .collect(),
        ),
        _ => unreachable!("only called for structs and enums"),
    };

    let serde = (!derives_serde(attrs)).then(|| {
        quote! { #[cfg_attr(wasm_runtime, derive(::serde::Serialize, ::serde::Deserialize))] }
    });

    for field in fields {
        let has_override = field
            .attrs
            .iter()
            .any(|attr| attr.to_token_stream().to_string().contains("tsify"));
        if !has_override && let Some(ts) = foreign_ts_type(&field.ty) {
            field
                .attrs
                .push(parse_quote! { #[cfg_attr(wasm_runtime, tsify(type = #ts))] });
        }
    }

    quote! {
        #[cfg_attr(wasm_runtime, derive(::tsify::Tsify))]
        #serde
    }
}

/// Whether the type derives serde itself, rather than only under wasm.
pub fn derives_serde(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("derive") && attr.to_token_stream().to_string().contains("Serialize")
    })
}

/// Lets JS callers omit the field. Types that derive serde themselves get it
/// on every target, so their serde representation stays the same everywhere.
pub fn default_field_attr(derives_serde: bool) -> Attribute {
    if derives_serde {
        parse_quote! { #[serde(default)] }
    } else {
        parse_quote! { #[cfg_attr(wasm_runtime, serde(default))] }
    }
}

/// TS types for foreign field types that don't implement `Tsify`, keyed by
/// how they serialize. Anything else can be overridden on the field with
/// `#[cfg_attr(wasm_runtime, tsify(type = "..."))]`.
fn foreign_ts_type(ty: &Type) -> Option<String> {
    if let Shape::Option(inner) = shape(ty)
        && let Shape::Custom(inner) = *inner
    {
        return foreign_ts_type(inner).map(|ts| format!("{ts} | undefined"));
    }
    let Type::Path(path) = ty else { return None };
    let first = path.path.segments.first()?;
    // jiff's serde impls serialize all of its date/time types as ISO 8601 strings.
    (first.ident == "jiff").then(|| "string".to_string())
}

/// Exported wrapper for a free function, placed next to it.
pub fn export_fn(item_fn: &ItemFn) -> syn::Result<TokenStream> {
    let name = &item_fn.sig.ident;
    let wrapper_name = format_ident!("__sdk_export_{}", name);
    let wrapper = wrapper(
        &item_fn.sig,
        &item_fn.attrs,
        &wrapper_name,
        quote! { #name },
        false,
    )?;
    Ok(quote! {
        const _: () = {
            use ::core::result::Result;
            use ::wasm_bindgen::prelude::wasm_bindgen;
            #wrapper
        };
    })
}

/// Exported wrapper for a method, to be placed in a `#[wasm_bindgen] impl` of
/// the SDK handle type, which forwards to `self.inner`. Like the free
/// function wrapper, it expects `Result` and `wasm_bindgen` to be imported
/// bare, since wasm-bindgen recognizes both by name.
pub fn export_method(method: &ImplItemFn) -> syn::Result<TokenStream> {
    let name = &method.sig.ident;
    wrapper(
        &method.sig,
        &method.attrs,
        name,
        quote! { self.inner.#name },
        true,
    )
}

fn wrapper(
    sig: &Signature,
    attrs: &[Attribute],
    wrapper_name: &syn::Ident,
    callee: TokenStream,
    is_method: bool,
) -> syn::Result<TokenStream> {
    if !sig.generics.params.is_empty() {
        return Err(syn::Error::new(
            sig.generics.span(),
            "exported functions can't be generic",
        ));
    }

    let mut has_receiver = false;
    let mut params = Vec::new();
    let mut args = Vec::new();
    for input in &sig.inputs {
        match input {
            FnArg::Receiver(receiver) => {
                if !matches!(receiver.kind, ReceiverKind::Reference(_, _, None)) {
                    return Err(syn::Error::new(
                        receiver.span(),
                        "exported methods must take `&self`",
                    ));
                }
                has_receiver = true;
            }
            FnArg::Typed(typed) => {
                let Pat::Ident(pat) = &*typed.pat else {
                    return Err(syn::Error::new(
                        typed.pat.span(),
                        "exported arguments must be plain identifiers",
                    ));
                };
                let ident = &pat.ident;
                let (ty, arg) = param(ident, &typed.ty)?;
                let js_name = to_camel_case(&ident.to_string());
                params.push(quote! { #[wasm_bindgen(js_name = #js_name)] #ident: #ty });
                args.push(arg);
            }
        }
    }
    if is_method && !has_receiver {
        return Err(syn::Error::new(
            sig.span(),
            "exported methods must take `&self`",
        ));
    }

    let (ret_ty, fallible) = match &sig.output {
        ReturnType::Default => (None, false),
        ReturnType::Type(_, ty) => {
            let (inner, fallible) = unwrap_result(ty);
            (Some(inner), fallible)
        }
    };
    let (out_ty, out_expr) = match ret_ty {
        Some(ty) => output(ty)?,
        None => (quote! { () }, quote! { value }),
    };

    let asyncness = &sig.asyncness;
    let await_ = asyncness.map(|_| quote! { .await });
    let unwrap_error =
        fallible.then(|| quote! { let value = value.map_err(crate::sdk::js_error)?; });
    let js_name = to_camel_case(&sig.ident.to_string());
    let receiver = has_receiver.then(|| quote! { &self, });
    let docs = attrs.iter().filter(|attr| attr.path().is_ident("doc"));

    Ok(quote! {
        #(#docs)*
        #[wasm_bindgen(js_name = #js_name)]
        pub #asyncness fn #wrapper_name(#receiver #(#params),*)
            -> Result<#out_ty, ::wasm_bindgen::JsError>
        {
            let value = #callee(#(#args),*) #await_;
            #unwrap_error
            Ok(#out_expr)
        }
    })
}

/// Maps an original parameter to `(wrapper parameter type, argument expression)`.
fn param(ident: &syn::Ident, ty: &Type) -> syn::Result<(TokenStream, TokenStream)> {
    let unsupported = || syn::Error::new(ty.span(), "unsupported argument type for `sdk_export`");
    Ok(match shape(ty) {
        Shape::Native(ty) => (quote! { #ty }, quote! { #ident }),
        Shape::Str => (quote! { String }, quote! { &#ident }),
        Shape::Custom(ty) => (quote! { ::tsify::Ts<#ty> }, quote! { #ident.to_rust()? }),
        Shape::Vec(ty) => (
            quote! { Vec<::tsify::Ts<#ty>> },
            quote! { #ident.iter().map(::tsify::Ts::to_rust).collect::<::core::result::Result<Vec<_>, _>>()? },
        ),
        Shape::Ref(inner) => match *inner {
            Shape::Native(ty) => (quote! { #ty }, quote! { &#ident }),
            Shape::Custom(ty) => (quote! { ::tsify::Ts<#ty> }, quote! { &#ident.to_rust()? }),
            _ => return Err(unsupported()),
        },
        Shape::Option(inner) => match *inner {
            Shape::Native(ty) => (quote! { Option<#ty> }, quote! { #ident }),
            Shape::Str => (quote! { Option<String> }, quote! { #ident.as_deref() }),
            Shape::Custom(ty) => (
                quote! { Option<::tsify::Ts<#ty>> },
                quote! { #ident.as_ref().map(::tsify::Ts::to_rust).transpose()? },
            ),
            // `Option<&T>`: the converted value is a temporary that lives
            // until the end of the call statement, so it can be borrowed.
            Shape::Ref(inner) => match *inner {
                Shape::Native(ty) => (quote! { Option<#ty> }, quote! { #ident.as_ref() }),
                Shape::Custom(ty) => (
                    quote! { Option<::tsify::Ts<#ty>> },
                    quote! { #ident.as_ref().map(::tsify::Ts::to_rust).transpose()?.as_ref() },
                ),
                _ => return Err(unsupported()),
            },
            _ => return Err(unsupported()),
        },
        Shape::Unit => return Err(unsupported()),
    })
}

/// Maps an original (successful) return type to `(wrapper type, expression
/// converting `value`)`.
fn output(ty: &Type) -> syn::Result<(TokenStream, TokenStream)> {
    let unsupported = || syn::Error::new(ty.span(), "unsupported return type for `sdk_export`");
    Ok(match shape(ty) {
        Shape::Unit => (quote! { () }, quote! { () }),
        Shape::Native(ty) => (quote! { #ty }, quote! { value }),
        Shape::Custom(ty) => (
            quote! { ::tsify::Ts<#ty> },
            quote! { ::tsify::Tsify::into_ts(&value)? },
        ),
        Shape::Vec(ty) => (
            quote! { Vec<::tsify::Ts<#ty>> },
            quote! { value.iter().map(::tsify::Tsify::into_ts).collect::<::core::result::Result<Vec<_>, _>>()? },
        ),
        Shape::Option(inner) => match *inner {
            Shape::Native(ty) => (quote! { Option<#ty> }, quote! { value }),
            Shape::Custom(ty) => (
                quote! { Option<::tsify::Ts<#ty>> },
                quote! { value.as_ref().map(::tsify::Tsify::into_ts).transpose()? },
            ),
            _ => return Err(unsupported()),
        },
        Shape::Str | Shape::Ref(_) => return Err(unsupported()),
    })
}

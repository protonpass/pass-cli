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

//! The uniffi backend: expands `#[sdk_export]` into uniffi exports, for the
//! Kotlin/Swift bindings built by `pass-mobile-sdk`.
//!
//! Domain types become uniffi records/enums and cross by value; references
//! are taken by value in the wrapper and borrowed for the call. Async
//! functions run on uniffi's tokio runtime, since the native transport is
//! tokio-based. Errors become `crate::sdk::PassError`. All generated code is
//! emitted behind `#[cfg(uniffi_runtime)]` by the caller.

use crate::types::{Shape, shape, unwrap_result};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{
    Attribute, FnArg, ImplItemFn, Item, ItemFn, Pat, ReceiverKind, ReturnType, Signature, Type,
};

/// Adds the uniffi derive an exported struct/enum needs. Foreign field types
/// (e.g. `jiff` datetimes) are registered once as custom types in `crate::sdk`.
pub fn prepare_type(item: &Item) -> TokenStream {
    match item {
        // uniffi records need named fields: single-field tuple structs (id
        // newtypes) cross as their inner type instead.
        Item::Struct(s) if matches!(&s.fields, syn::Fields::Unnamed(f) if f.unnamed.len() == 1) => {
            let name = &s.ident;
            let inner = &s.fields.iter().next().expect("one field").ty;
            quote! {
                #[cfg(uniffi_runtime)]
                ::uniffi::custom_newtype!(#name, #inner);
            }
        }
        Item::Struct(_) => quote! { #[cfg_attr(uniffi_runtime, derive(::uniffi::Record))] },
        Item::Enum(_) => quote! { #[cfg_attr(uniffi_runtime, derive(::uniffi::Enum))] },
        _ => unreachable!("only called for structs and enums"),
    }
}

/// Exported wrapper for a free function, placed next to it. The wrapper gets
/// a private name so it doesn't clash with the original, and is exported
/// under the original one.
pub fn export_fn(item_fn: &ItemFn) -> syn::Result<TokenStream> {
    let name = &item_fn.sig.ident;
    let wrapper_name = format_ident!("__sdk_export_uniffi_{}", name);
    let wrapper = wrapper(
        &item_fn.sig,
        &item_fn.attrs,
        &wrapper_name,
        quote! { #name },
        false,
    )?;
    let export_name = name.to_string();
    let runtime = item_fn
        .sig
        .asyncness
        .map(|_| quote! { async_runtime = "tokio", });
    Ok(quote! {
        #[::uniffi::export(#runtime name = #export_name)]
        #wrapper
    })
}

/// Exported wrapper for a method, to be placed in a `#[uniffi::export] impl`
/// of the SDK handle type, which forwards to `self.inner`.
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
                params.push(quote! { #ident: #ty });
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
    if let Some(ty) = ret_ty {
        check_output(ty)?;
    }
    let out_ty = match ret_ty {
        Some(ty) => quote! { #ty },
        None => quote! { () },
    };

    let asyncness = &sig.asyncness;
    let await_ = asyncness.map(|_| quote! { .await });
    let receiver = has_receiver.then(|| quote! { &self, });
    let docs = attrs.iter().filter(|attr| attr.path().is_ident("doc"));
    let call = quote! { #callee(#(#args),*) #await_ };

    Ok(if fallible {
        quote! {
            #(#docs)*
            pub #asyncness fn #wrapper_name(#receiver #(#params),*)
                -> ::core::result::Result<#out_ty, crate::sdk::PassError>
            {
                #call.map_err(crate::sdk::PassError::from)
            }
        }
    } else {
        quote! {
            #(#docs)*
            pub #asyncness fn #wrapper_name(#receiver #(#params),*) -> #out_ty {
                #call
            }
        }
    })
}

/// Maps an original parameter to `(wrapper parameter type, argument expression)`.
fn param(ident: &syn::Ident, ty: &Type) -> syn::Result<(TokenStream, TokenStream)> {
    let unsupported = || syn::Error::new(ty.span(), "unsupported argument type for `sdk_export`");
    Ok(match shape(ty) {
        Shape::Native(ty) | Shape::Custom(ty) => (quote! { #ty }, quote! { #ident }),
        Shape::Vec(ty) => (quote! { Vec<#ty> }, quote! { #ident }),
        Shape::Str => (quote! { String }, quote! { &#ident }),
        Shape::Ref(inner) => match *inner {
            Shape::Native(ty) | Shape::Custom(ty) => (quote! { #ty }, quote! { &#ident }),
            _ => return Err(unsupported()),
        },
        Shape::Option(inner) => match *inner {
            Shape::Native(ty) | Shape::Custom(ty) => (quote! { Option<#ty> }, quote! { #ident }),
            Shape::Str => (quote! { Option<String> }, quote! { #ident.as_deref() }),
            Shape::Ref(inner) => match *inner {
                Shape::Native(ty) | Shape::Custom(ty) => {
                    (quote! { Option<#ty> }, quote! { #ident.as_ref() })
                }
                _ => return Err(unsupported()),
            },
            _ => return Err(unsupported()),
        },
        Shape::Unit => return Err(unsupported()),
    })
}

/// Return types cross as they are; only borrowed ones can't.
fn check_output(ty: &Type) -> syn::Result<()> {
    let unsupported = || syn::Error::new(ty.span(), "unsupported return type for `sdk_export`");
    match shape(ty) {
        Shape::Str | Shape::Ref(_) => Err(unsupported()),
        Shape::Option(inner) if matches!(*inner, Shape::Str | Shape::Ref(_)) => Err(unsupported()),
        _ => Ok(()),
    }
}

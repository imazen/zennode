//! Shared code generation helpers.

use proc_macro2::TokenStream;
use quote::quote;

/// Generate the `FormatHint` literal from parsed attributes.
pub fn format_hint_tokens(
    preferred: &Option<syn::Ident>,
    alpha: &Option<syn::Ident>,
    changes_dimensions: bool,
    is_neighborhood: bool,
) -> TokenStream {
    let preferred = match preferred {
        Some(p) => quote! { ::zennode::PixelFormatPreference::#p },
        None => quote! { ::zennode::PixelFormatPreference::Any },
    };
    let alpha = match alpha {
        Some(a) => quote! { ::zennode::AlphaHandling::#a },
        None => quote! { ::zennode::AlphaHandling::Process },
    };

    quote! {
        ::zennode::FormatHint {
            preferred: #preferred,
            alpha: #alpha,
            changes_dimensions: #changes_dimensions,
            is_neighborhood: #is_neighborhood,
        }
    }
}

/// Generate coalesce info tokens.
pub fn coalesce_tokens(
    coalesce_group: &Option<syn::LitStr>,
    fusable: bool,
    is_target: bool,
) -> TokenStream {
    match coalesce_group {
        Some(group) => quote! {
            ::core::option::Option::Some(::zennode::CoalesceInfo {
                group: #group,
                fusable: #fusable,
                is_target: #is_target,
            })
        },
        None if fusable => quote! {
            ::core::option::Option::Some(::zennode::CoalesceInfo {
                group: "",
                fusable: true,
                is_target: #is_target,
            })
        },
        None => quote! { ::core::option::Option::None },
    }
}

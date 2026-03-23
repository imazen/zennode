//! `#[derive(NodeEnum)]` implementation.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields};

use crate::attrs;

pub fn derive_node_enum_impl(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    match derive_node_enum_inner(&input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn derive_node_enum_inner(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let enum_name = &input.ident;
    let variants = match &input.data {
        Data::Enum(data) => &data.variants,
        _ => {
            return Err(syn::Error::new_spanned(
                enum_name,
                "NodeEnum can only be derived on enums",
            ));
        }
    };

    let mut variant_descriptors = Vec::new();
    let mut name_arms = Vec::new();
    let mut from_str_arms = Vec::new();

    for variant in variants {
        if !matches!(variant.fields, Fields::Unit) {
            return Err(syn::Error::new_spanned(
                &variant.ident,
                "NodeEnum variants must be unit variants (no fields)",
            ));
        }

        let variant_ident = &variant.ident;
        let snake_name = to_snake_case(&variant_ident.to_string());
        let doc = attrs::extract_doc_comment(&variant.attrs);

        // Check for #[variant(label = "...", alias = "...")]
        let mut custom_label = None;
        let mut aliases: Vec<String> = Vec::new();

        for attr in &variant.attrs {
            if attr.path().is_ident("variant") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("label") {
                        meta.input.parse::<syn::Token![=]>()?;
                        let lit: syn::LitStr = meta.input.parse()?;
                        custom_label = Some(lit.value());
                    } else if meta.path.is_ident("alias") {
                        meta.input.parse::<syn::Token![=]>()?;
                        let lit: syn::LitStr = meta.input.parse()?;
                        aliases.push(lit.value());
                    } else {
                        return Err(meta.error("unknown variant attribute"));
                    }
                    Ok(())
                })?;
            }
        }

        let label =
            custom_label.unwrap_or_else(|| attrs::ident_to_label(&variant_ident.to_string()));

        variant_descriptors.push(quote! {
            ::zenode::EnumVariant {
                name: #snake_name,
                label: #label,
                description: #doc,
            }
        });

        name_arms.push(quote! {
            Self::#variant_ident => #snake_name,
        });

        from_str_arms.push(quote! {
            #snake_name => ::core::result::Result::Ok(Self::#variant_ident),
        });

        for alias in &aliases {
            from_str_arms.push(quote! {
                #alias => ::core::result::Result::Ok(Self::#variant_ident),
            });
        }
    }

    let num_variants = variant_descriptors.len();
    let variants_name = format_ident!(
        "__ZENODE_{}_VARIANTS",
        to_screaming_snake(&enum_name.to_string())
    );

    Ok(quote! {
        static #variants_name: [::zenode::EnumVariant; #num_variants] = [
            #(#variant_descriptors),*
        ];

        impl #enum_name {
            /// Static variant descriptors for schema generation.
            pub fn zenode_variants() -> &'static [::zenode::EnumVariant] {
                &#variants_name
            }

            /// Get the snake_case name for this variant.
            pub fn zenode_name(&self) -> &'static str {
                match self {
                    #(#name_arms)*
                }
            }
        }

        impl ::core::fmt::Display for #enum_name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.write_str(self.zenode_name())
            }
        }

        impl ::core::str::FromStr for #enum_name {
            type Err = ::zenode::NodeError;
            fn from_str(s: &str) -> ::core::result::Result<Self, Self::Err> {
                match s {
                    #(#from_str_arms)*
                    _ => {
                        let mut msg = ::zenode::__private::String::from("unknown variant: ");
                        msg.push_str(s);
                        ::core::result::Result::Err(::zenode::NodeError::Other(msg))
                    }
                }
            }
        }
    })
}

fn to_snake_case(name: &str) -> String {
    let mut result = String::new();
    for (i, ch) in name.chars().enumerate() {
        if i > 0 && ch.is_uppercase() {
            result.push('_');
        }
        result.push(ch.to_ascii_lowercase());
    }
    result
}

fn to_screaming_snake(name: &str) -> String {
    let mut result = String::new();
    for (i, ch) in name.chars().enumerate() {
        if i > 0 && ch.is_uppercase() {
            result.push('_');
        }
        result.push(ch.to_ascii_uppercase());
    }
    result
}

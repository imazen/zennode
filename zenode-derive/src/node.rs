//! `#[derive(Node)]` implementation.

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields};

use crate::attrs::{self, NodeAttrs, ParamAttrs};
use crate::codegen;

pub fn derive_node_impl(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    match derive_node_inner(&input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn derive_node_inner(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let node_attrs = NodeAttrs::from_ast(&input.attrs)?;

    // Validate required attributes
    let id = node_attrs
        .id
        .as_ref()
        .ok_or_else(|| syn::Error::new(Span::call_site(), "missing #[node(id = \"...\")]"))?;
    let group = node_attrs
        .group
        .as_ref()
        .ok_or_else(|| syn::Error::new(Span::call_site(), "missing #[node(group = ...)]"))?;
    let role = node_attrs.role.as_ref().ok_or_else(|| {
        syn::Error::new(
            Span::call_site(),
            "missing #[node(role = ...)] (or legacy #[node(phase = ...)])",
        )
    })?;

    let struct_name = &input.ident;
    let struct_doc = attrs::extract_doc_comment(&input.attrs);

    let label = match &node_attrs.label {
        Some(l) => l.value(),
        None => attrs::ident_to_label(&struct_name.to_string()),
    };

    let version = node_attrs.version.unwrap_or(1);
    let compat_version = node_attrs.compat_version.unwrap_or(1);

    // Extract named fields
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new(
                    Span::call_site(),
                    "Node requires named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new(
                Span::call_site(),
                "Node can only be derived on structs",
            ));
        }
    };

    // Parse field attributes and generate param descriptors
    let mut param_desc_tokens = Vec::new();
    let mut to_params_tokens = Vec::new();
    let mut get_param_arms = Vec::new();
    let mut set_param_arms = Vec::new();
    let mut default_init_tokens = Vec::new();
    let mut from_kv_tokens = Vec::new();
    let mut identity_checks = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_name_str = field_name.to_string();
        let field_type = &field.ty;
        let param_attrs = ParamAttrs::from_ast(&field.attrs)?;
        let field_doc = attrs::extract_doc_comment(&field.attrs);

        let param_label = match &param_attrs.label {
            Some(l) => l.value(),
            None => attrs::snake_to_label(&field_name_str),
        };
        let unit = param_attrs
            .unit
            .as_ref()
            .map(|l| l.value())
            .unwrap_or_default();
        let section = param_attrs
            .section
            .as_ref()
            .map(|l| l.value())
            .unwrap_or_else(|| "Main".to_string());
        let since = param_attrs.since.unwrap_or(1);
        let visible_when = param_attrs
            .visible_when
            .as_ref()
            .map(|l| l.value())
            .unwrap_or_default();

        let slider_tokens = match &param_attrs.slider {
            Some(s) => quote! { ::zenode::SliderMapping::#s },
            None => quote! { ::zenode::SliderMapping::Linear },
        };

        let kv_keys: Vec<_> = param_attrs.kv_keys.iter().collect();
        let kv_keys_tokens = if kv_keys.is_empty() {
            quote! { &[] }
        } else {
            quote! { &[#(#kv_keys),*] }
        };

        // Determine ParamKind from field type and attributes
        let (kind_tokens, value_variant, default_expr, identity_expr) =
            field_param_kind(field_type, &param_attrs, &field_name_str)?;

        param_desc_tokens.push(quote! {
            ::zenode::ParamDesc {
                name: #field_name_str,
                label: #param_label,
                description: #field_doc,
                kind: #kind_tokens,
                unit: #unit,
                section: #section,
                slider: #slider_tokens,
                kv_keys: #kv_keys_tokens,
                since_version: #since,
                visible_when: #visible_when,
            }
        });

        // to_params
        to_params_tokens.push(gen_to_params(
            field_name,
            &field_name_str,
            field_type,
            value_variant,
        ));

        // get_param
        get_param_arms.push(gen_get_param(
            field_name,
            &field_name_str,
            field_type,
            value_variant,
        ));

        // set_param
        set_param_arms.push(gen_set_param(
            field_name,
            &field_name_str,
            field_type,
            value_variant,
        ));

        // Default initializer
        default_init_tokens.push(quote! { #field_name: #default_expr });

        // from_kv
        if !param_attrs.kv_keys.is_empty() {
            from_kv_tokens.push(gen_from_kv(
                field_name,
                &param_attrs.kv_keys,
                field_type,
                id,
            ));
        }

        // is_identity check
        if let Some(id_expr) = &identity_expr {
            identity_checks.push(gen_identity_check(field_name, field_type, id_expr));
        }
    }

    let tags: Vec<_> = node_attrs.tags.iter().collect();
    let tags_tokens = if tags.is_empty() {
        quote! { &[] }
    } else {
        quote! { &[#(#tags),*] }
    };

    let format_tokens = codegen::format_hint_tokens(
        &node_attrs.preferred_format,
        &node_attrs.alpha_handling,
        node_attrs.changes_dimensions,
        node_attrs.neighborhood,
    );

    let coalesce_tokens = codegen::coalesce_tokens(
        &node_attrs.coalesce,
        node_attrs.fusable,
        node_attrs.coalesce_target,
    );

    let num_params = param_desc_tokens.len();

    // Generated names
    let schema_name = format_ident!(
        "__ZENODE_{}_SCHEMA",
        to_screaming_snake(&struct_name.to_string())
    );
    let params_name = format_ident!(
        "__ZENODE_{}_PARAMS",
        to_screaming_snake(&struct_name.to_string())
    );
    let def_struct = format_ident!("{}NodeDef", struct_name);
    let def_static = format_ident!("{}_NODE", to_screaming_snake(&struct_name.to_string()));

    let is_identity_body = if identity_checks.is_empty() {
        quote! { false }
    } else {
        quote! { #(#identity_checks)&&* }
    };

    let from_kv_body = if from_kv_tokens.is_empty() {
        quote! { Ok(None) }
    } else {
        quote! {
            let mut __node = #struct_name { #(#default_init_tokens),* };
            let mut __matched = false;
            #(#from_kv_tokens)*
            if __matched { Ok(Some(::zenode::__private::Box::new(__node))) } else { Ok(None) }
        }
    };

    Ok(quote! {
        // Static param descriptors
        static #params_name: [::zenode::ParamDesc; #num_params] = [
            #(#param_desc_tokens),*
        ];

        // Static schema
        static #schema_name: ::zenode::NodeSchema = ::zenode::NodeSchema {
            id: #id,
            label: #label,
            description: #struct_doc,
            group: ::zenode::NodeGroup::#group,
            role: ::zenode::NodeRole::#role,
            params: &#params_name,
            tags: #tags_tokens,
            coalesce: #coalesce_tokens,
            format: #format_tokens,
            version: #version,
            compat_version: #compat_version,
        };

        /// Node definition (factory) for [`#struct_name`].
        pub struct #def_struct;

        /// Static node definition singleton.
        pub static #def_static: #def_struct = #def_struct;

        impl ::zenode::NodeDef for #def_struct {
            fn schema(&self) -> &'static ::zenode::NodeSchema {
                &#schema_name
            }

            fn create(&self, params: &::zenode::ParamMap) -> ::core::result::Result<::zenode::__private::Box<dyn ::zenode::NodeInstance>, ::zenode::NodeError> {
                let mut __node = #struct_name { #(#default_init_tokens),* };
                for (__name, __value) in params {
                    if !<#struct_name as ::zenode::NodeInstance>::set_param(&mut __node, __name, __value.clone()) {
                        // Ignore unknown params for forward compatibility
                    }
                }
                Ok(::zenode::__private::Box::new(__node))
            }

            fn from_kv(&self, kv: &mut ::zenode::KvPairs) -> ::core::result::Result<::core::option::Option<::zenode::__private::Box<dyn ::zenode::NodeInstance>>, ::zenode::NodeError> {
                #from_kv_body
            }
        }

        impl ::zenode::NodeInstance for #struct_name {
            fn schema(&self) -> &'static ::zenode::NodeSchema {
                &#schema_name
            }

            fn to_params(&self) -> ::zenode::ParamMap {
                let mut __map = ::zenode::ParamMap::new();
                #(#to_params_tokens)*
                __map
            }

            fn get_param(&self, name: &str) -> ::core::option::Option<::zenode::ParamValue> {
                match name {
                    #(#get_param_arms)*
                    _ => None,
                }
            }

            fn set_param(&mut self, name: &str, value: ::zenode::ParamValue) -> bool {
                match name {
                    #(#set_param_arms)*
                    _ => false,
                }
            }

            fn as_any(&self) -> &dyn ::core::any::Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn ::core::any::Any {
                self
            }

            fn clone_boxed(&self) -> ::zenode::__private::Box<dyn ::zenode::NodeInstance> {
                ::zenode::__private::Box::new(self.clone())
            }

            fn is_identity(&self) -> bool {
                #is_identity_body
            }
        }
    })
}

/// Try to parse a type string as `[f32;N]` and return `N` if it matches.
fn parse_f32_array(type_str: &str) -> Option<usize> {
    let s = type_str.strip_prefix('[')?.strip_suffix(']')?;
    let (elem, len_str) = s.split_once(';')?;
    if elem.trim() != "f32" {
        return None;
    }
    len_str.trim().parse::<usize>().ok()
}

/// Determine ParamKind tokens, ParamValue variant, default expr, identity expr for a field.
fn field_param_kind(
    ty: &syn::Type,
    attrs: &ParamAttrs,
    _field_name: &str,
) -> syn::Result<(
    TokenStream2,
    &'static str,
    TokenStream2,
    Option<TokenStream2>,
)> {
    let type_str = quote!(#ty).to_string().replace(' ', "");

    // Check for [f32; N] array type before the scalar match
    if let Some(len) = parse_f32_array(&type_str) {
        let min = attrs
            .range_min
            .as_ref()
            .map(|e| quote!(#e))
            .unwrap_or(quote!(f32::MIN));
        let max = attrs
            .range_max
            .as_ref()
            .map(|e| quote!(#e))
            .unwrap_or(quote!(f32::MAX));
        let default = attrs
            .default
            .as_ref()
            .map(|e| quote!(#e))
            .unwrap_or(quote!(0.0));
        let labels: Vec<_> = attrs.labels.iter().collect();
        let labels_tokens = if labels.is_empty() {
            quote! { &[] }
        } else {
            quote! { &[#(#labels),*] }
        };
        let kind = quote! {
            ::zenode::ParamKind::FloatArray {
                len: #len, min: #min, max: #max, default: #default, labels: #labels_tokens,
            }
        };
        let default_expr = quote! { [#default; #len] };
        let id_expr = attrs.identity.as_ref().map(|e| quote!(#e));
        return Ok((kind, "F32Array", default_expr, id_expr));
    }

    match type_str.as_str() {
        "f32" => {
            let min = attrs
                .range_min
                .as_ref()
                .map(|e| quote!(#e))
                .unwrap_or(quote!(f32::MIN));
            let max = attrs
                .range_max
                .as_ref()
                .map(|e| quote!(#e))
                .unwrap_or(quote!(f32::MAX));
            let default = attrs
                .default
                .as_ref()
                .map(|e| quote!(#e))
                .unwrap_or(quote!(0.0));
            let identity = attrs
                .identity
                .as_ref()
                .map(|e| quote!(#e))
                .unwrap_or(quote!(0.0));
            let step = attrs
                .step
                .as_ref()
                .map(|e| quote!(#e))
                .unwrap_or(quote!(0.1));
            let kind = quote! {
                ::zenode::ParamKind::Float {
                    min: #min, max: #max, default: #default, identity: #identity, step: #step,
                }
            };
            let id_expr = attrs.identity.as_ref().map(|e| quote!(#e));
            Ok((kind, "F32", default, id_expr))
        }
        "i32" => {
            let min = attrs
                .range_min
                .as_ref()
                .map(|e| quote!(#e))
                .unwrap_or(quote!(i32::MIN));
            let max = attrs
                .range_max
                .as_ref()
                .map(|e| quote!(#e))
                .unwrap_or(quote!(i32::MAX));
            let default = attrs
                .default
                .as_ref()
                .map(|e| quote!(#e))
                .unwrap_or(quote!(0));
            let kind = quote! {
                ::zenode::ParamKind::Int { min: #min, max: #max, default: #default }
            };
            Ok((kind, "I32", default, None))
        }
        "u32" => {
            let min = attrs
                .range_min
                .as_ref()
                .map(|e| quote!(#e))
                .unwrap_or(quote!(0));
            let max = attrs
                .range_max
                .as_ref()
                .map(|e| quote!(#e))
                .unwrap_or(quote!(u32::MAX));
            let default = attrs
                .default
                .as_ref()
                .map(|e| quote!(#e))
                .unwrap_or(quote!(0));
            let kind = quote! {
                ::zenode::ParamKind::U32 { min: #min, max: #max, default: #default }
            };
            Ok((kind, "U32", default, None))
        }
        "bool" => {
            let default = attrs
                .default
                .as_ref()
                .map(|e| quote!(#e))
                .unwrap_or(quote!(false));
            let kind = quote! { ::zenode::ParamKind::Bool { default: #default } };
            Ok((kind, "Bool", default, None))
        }
        "String" => {
            let default_lit = attrs
                .default
                .as_ref()
                .map(|e| quote!(#e))
                .unwrap_or(quote!(""));
            let kind = quote! { ::zenode::ParamKind::Str { default: #default_lit } };
            let default_expr = attrs
                .default
                .as_ref()
                .map(|e| quote!(::zenode::__private::String::from(#e)))
                .unwrap_or_else(|| quote!(::zenode::__private::String::new()));
            Ok((kind, "Str", default_expr, None))
        }
        _ => {
            // Unknown type: treat as string
            let default_lit = attrs
                .default
                .as_ref()
                .map(|e| quote!(#e))
                .unwrap_or(quote!(""));
            let kind = quote! { ::zenode::ParamKind::Str { default: #default_lit } };
            let default_expr = attrs
                .default
                .as_ref()
                .map(|e| quote!(::zenode::__private::String::from(#e)))
                .unwrap_or_else(|| quote!(::zenode::__private::String::new()));
            Ok((kind, "Str", default_expr, None))
        }
    }
}

fn gen_to_params(
    field_name: &syn::Ident,
    field_name_str: &str,
    field_type: &syn::Type,
    _variant: &str,
) -> TokenStream2 {
    let type_str = quote!(#field_type).to_string().replace(' ', "");
    if parse_f32_array(&type_str).is_some() {
        return quote! {
            __map.insert(
                ::zenode::__private::String::from(#field_name_str),
                ::zenode::ParamValue::F32Array(::zenode::__private::Vec::from(self.#field_name.as_slice())),
            );
        };
    }
    match type_str.as_str() {
        "f32" => quote! {
            __map.insert(::zenode::__private::String::from(#field_name_str), ::zenode::ParamValue::F32(self.#field_name));
        },
        "i32" => quote! {
            __map.insert(::zenode::__private::String::from(#field_name_str), ::zenode::ParamValue::I32(self.#field_name));
        },
        "u32" => quote! {
            __map.insert(::zenode::__private::String::from(#field_name_str), ::zenode::ParamValue::U32(self.#field_name));
        },
        "bool" => quote! {
            __map.insert(::zenode::__private::String::from(#field_name_str), ::zenode::ParamValue::Bool(self.#field_name));
        },
        _ => quote! {
            __map.insert(::zenode::__private::String::from(#field_name_str), ::zenode::ParamValue::Str(::zenode::__private::ToString::to_string(&self.#field_name)));
        },
    }
}

fn gen_get_param(
    field_name: &syn::Ident,
    field_name_str: &str,
    field_type: &syn::Type,
    _variant: &str,
) -> TokenStream2 {
    let type_str = quote!(#field_type).to_string().replace(' ', "");
    if parse_f32_array(&type_str).is_some() {
        return quote! {
            #field_name_str => ::core::option::Option::Some(
                ::zenode::ParamValue::F32Array(::zenode::__private::Vec::from(self.#field_name.as_slice()))
            ),
        };
    }
    let value_expr = match type_str.as_str() {
        "f32" => quote! { ::zenode::ParamValue::F32(self.#field_name) },
        "i32" => quote! { ::zenode::ParamValue::I32(self.#field_name) },
        "u32" => quote! { ::zenode::ParamValue::U32(self.#field_name) },
        "bool" => quote! { ::zenode::ParamValue::Bool(self.#field_name) },
        _ => {
            quote! { ::zenode::ParamValue::Str(::zenode::__private::ToString::to_string(&self.#field_name)) }
        }
    };
    quote! {
        #field_name_str => ::core::option::Option::Some(#value_expr),
    }
}

fn gen_set_param(
    field_name: &syn::Ident,
    field_name_str: &str,
    field_type: &syn::Type,
    _variant: &str,
) -> TokenStream2 {
    let type_str = quote!(#field_type).to_string().replace(' ', "");
    if let Some(len) = parse_f32_array(&type_str) {
        return quote! {
            #field_name_str => {
                match value.as_f32_array() {
                    ::core::option::Option::Some(arr) if arr.len() == #len => {
                        self.#field_name.copy_from_slice(arr);
                        true
                    }
                    _ => false,
                }
            }
        };
    }
    let extract = match type_str.as_str() {
        "f32" => quote! { value.as_f32() },
        "i32" => quote! { value.as_i32() },
        "u32" => quote! { value.as_u32() },
        "bool" => quote! { value.as_bool() },
        _ => quote! { value.as_str().map(::zenode::__private::ToString::to_string) },
    };
    let assign = quote! { self.#field_name = v; };
    quote! {
        #field_name_str => {
            match #extract {
                ::core::option::Option::Some(v) => { #assign true }
                ::core::option::Option::None => false,
            }
        }
    }
}

fn gen_from_kv(
    field_name: &syn::Ident,
    kv_keys: &[syn::LitStr],
    field_type: &syn::Type,
    node_id: &syn::LitStr,
) -> TokenStream2 {
    let type_str = quote!(#field_type).to_string().replace(' ', "");

    // Arrays don't come from querystrings — skip
    if parse_f32_array(&type_str).is_some() {
        return quote! {};
    }

    let take_method = match type_str.as_str() {
        "f32" => quote! { take_f32 },
        "i32" => quote! { take_i32 },
        "u32" => quote! { take_u32 },
        "bool" => quote! { take_bool },
        "String" => quote! { take_owned },
        _ => quote! { take_owned },
    };

    let first_key = &kv_keys[0];
    let rest_keys = &kv_keys[1..];

    let mut chain = quote! {
        if let ::core::option::Option::Some(__v) = kv.#take_method(#first_key, #node_id) {
            __node.#field_name = __v;
            __matched = true;
        }
    };

    for key in rest_keys {
        chain = quote! {
            #chain
            else if let ::core::option::Option::Some(__v) = kv.#take_method(#key, #node_id) {
                __node.#field_name = __v;
                __matched = true;
            }
        };
    }

    chain
}

fn gen_identity_check(
    field_name: &syn::Ident,
    field_type: &syn::Type,
    identity_expr: &TokenStream2,
) -> TokenStream2 {
    let type_str = quote!(#field_type).to_string().replace(' ', "");
    if parse_f32_array(&type_str).is_some() {
        return quote! { self.#field_name.iter().all(|v| (v - #identity_expr).abs() < 1e-6) };
    }
    match type_str.as_str() {
        "f32" => quote! { (self.#field_name - #identity_expr).abs() < 1e-6 },
        _ => quote! { self.#field_name == #identity_expr },
    }
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

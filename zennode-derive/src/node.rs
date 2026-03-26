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
            Some(s) => quote! { ::zennode::SliderMapping::#s },
            None => quote! { ::zennode::SliderMapping::Linear },
        };

        let kv_keys: Vec<_> = param_attrs.kv_keys.iter().collect();
        let kv_keys_tokens = if kv_keys.is_empty() {
            quote! { &[] }
        } else {
            quote! { &[#(#kv_keys),*] }
        };

        // Determine ParamKind from field type and attributes
        let fk = field_param_kind(field_type, &param_attrs, &field_name_str)?;
        let kind_tokens = &fk.kind_tokens;
        let default_expr = &fk.default_expr;
        let is_optional = fk.is_optional;

        param_desc_tokens.push(quote! {
            ::zennode::ParamDesc {
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
                optional: #is_optional,
            }
        });

        // to_params
        to_params_tokens.push(gen_to_params(
            field_name,
            &field_name_str,
            field_type,
            fk.value_variant,
            is_optional,
        ));

        // get_param
        get_param_arms.push(gen_get_param(
            field_name,
            &field_name_str,
            field_type,
            fk.value_variant,
            is_optional,
        ));

        // set_param
        set_param_arms.push(gen_set_param(
            field_name,
            &field_name_str,
            field_type,
            fk.value_variant,
            is_optional,
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
                is_optional,
            ));
        }

        // is_identity check
        if let Some(id_expr) = &fk.identity_expr {
            identity_checks.push(gen_identity_check(field_name, field_type, id_expr, is_optional));
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
            if __matched { Ok(Some(::zennode::__private::Box::new(__node))) } else { Ok(None) }
        }
    };

    Ok(quote! {
        // Static param descriptors
        static #params_name: [::zennode::ParamDesc; #num_params] = [
            #(#param_desc_tokens),*
        ];

        // Static schema
        static #schema_name: ::zennode::NodeSchema = ::zennode::NodeSchema {
            id: #id,
            label: #label,
            description: #struct_doc,
            group: ::zennode::NodeGroup::#group,
            role: ::zennode::NodeRole::#role,
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

        impl ::zennode::NodeDef for #def_struct {
            fn schema(&self) -> &'static ::zennode::NodeSchema {
                &#schema_name
            }

            fn create(&self, params: &::zennode::ParamMap) -> ::core::result::Result<::zennode::__private::Box<dyn ::zennode::NodeInstance>, ::zennode::NodeError> {
                let mut __node = #struct_name { #(#default_init_tokens),* };
                for (__name, __value) in params {
                    if !<#struct_name as ::zennode::NodeInstance>::set_param(&mut __node, __name, __value.clone()) {
                        // Ignore unknown params for forward compatibility
                    }
                }
                Ok(::zennode::__private::Box::new(__node))
            }

            fn from_kv(&self, kv: &mut ::zennode::KvPairs) -> ::core::result::Result<::core::option::Option<::zennode::__private::Box<dyn ::zennode::NodeInstance>>, ::zennode::NodeError> {
                #from_kv_body
            }
        }

        impl ::zennode::NodeInstance for #struct_name {
            fn schema(&self) -> &'static ::zennode::NodeSchema {
                &#schema_name
            }

            fn to_params(&self) -> ::zennode::ParamMap {
                let mut __map = ::zennode::ParamMap::new();
                #(#to_params_tokens)*
                __map
            }

            fn get_param(&self, name: &str) -> ::core::option::Option<::zennode::ParamValue> {
                match name {
                    #(#get_param_arms)*
                    _ => None,
                }
            }

            fn set_param(&mut self, name: &str, value: ::zennode::ParamValue) -> bool {
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

            fn clone_boxed(&self) -> ::zennode::__private::Box<dyn ::zennode::NodeInstance> {
                ::zennode::__private::Box::new(self.clone())
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

/// Try to parse a type string as `Option<T>` and return the inner type string.
fn parse_option_inner(type_str: &str) -> Option<&str> {
    type_str.strip_prefix("Option<")?.strip_suffix('>')
}

/// Result from analyzing a field type: kind tokens, ParamValue variant name,
/// default expression, identity expression, and whether the field is optional.
struct FieldKindResult {
    kind_tokens: TokenStream2,
    value_variant: &'static str,
    default_expr: TokenStream2,
    identity_expr: Option<TokenStream2>,
    is_optional: bool,
}

/// Determine ParamKind tokens, ParamValue variant, default expr, identity expr for a field.
fn field_param_kind(
    ty: &syn::Type,
    attrs: &ParamAttrs,
    _field_name: &str,
) -> syn::Result<FieldKindResult> {
    let type_str = quote!(#ty).to_string().replace(' ', "");

    // Check for Option<T> — unwrap and recurse on inner type
    if let Some(inner) = parse_option_inner(&type_str) {
        let inner_result = field_param_kind_str(inner, attrs)?;
        // For optional fields, struct default is None
        return Ok(FieldKindResult {
            kind_tokens: inner_result.kind_tokens,
            value_variant: inner_result.value_variant,
            default_expr: quote! { ::core::option::Option::None },
            identity_expr: inner_result.identity_expr,
            is_optional: true,
        });
    }

    field_param_kind_str(&type_str, attrs).map(|r| FieldKindResult {
        is_optional: false,
        ..r
    })
}

/// Inner helper that works on a type string (for recursion from Option<T>).
fn field_param_kind_str(
    type_str: &str,
    attrs: &ParamAttrs,
) -> syn::Result<FieldKindResult> {
    // Check for [f32; N] array type before the scalar match
    if let Some(len) = parse_f32_array(type_str) {
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
            ::zennode::ParamKind::FloatArray {
                len: #len, min: #min, max: #max, default: #default, labels: #labels_tokens,
            }
        };
        let default_expr = quote! { [#default; #len] };
        let id_expr = attrs.identity.as_ref().map(|e| quote!(#e));
        return Ok(FieldKindResult {
            kind_tokens: kind,
            value_variant: "F32Array",
            default_expr,
            identity_expr: id_expr,
            is_optional: false,
        });
    }

    match type_str {
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
                ::zennode::ParamKind::Float {
                    min: #min, max: #max, default: #default, identity: #identity, step: #step,
                }
            };
            let id_expr = attrs.identity.as_ref().map(|e| quote!(#e));
            Ok(FieldKindResult {
                kind_tokens: kind,
                value_variant: "F32",
                default_expr: default,
                identity_expr: id_expr,
                is_optional: false,
            })
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
                ::zennode::ParamKind::Int { min: #min, max: #max, default: #default }
            };
            Ok(FieldKindResult {
                kind_tokens: kind,
                value_variant: "I32",
                default_expr: default,
                identity_expr: None,
                is_optional: false,
            })
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
                ::zennode::ParamKind::U32 { min: #min, max: #max, default: #default }
            };
            Ok(FieldKindResult {
                kind_tokens: kind,
                value_variant: "U32",
                default_expr: default,
                identity_expr: None,
                is_optional: false,
            })
        }
        "bool" => {
            let default = attrs
                .default
                .as_ref()
                .map(|e| quote!(#e))
                .unwrap_or(quote!(false));
            let kind = quote! { ::zennode::ParamKind::Bool { default: #default } };
            Ok(FieldKindResult {
                kind_tokens: kind,
                value_variant: "Bool",
                default_expr: default,
                identity_expr: None,
                is_optional: false,
            })
        }
        "String" => {
            let default_lit = attrs
                .default
                .as_ref()
                .map(|e| quote!(#e))
                .unwrap_or(quote!(""));
            let kind = quote! { ::zennode::ParamKind::Str { default: #default_lit } };
            let default_expr = attrs
                .default
                .as_ref()
                .map(|e| quote!(::zennode::__private::String::from(#e)))
                .unwrap_or_else(|| quote!(::zennode::__private::String::new()));
            Ok(FieldKindResult {
                kind_tokens: kind,
                value_variant: "Str",
                default_expr,
                identity_expr: None,
                is_optional: false,
            })
        }
        _ => {
            // Unknown type: treat as string
            let default_lit = attrs
                .default
                .as_ref()
                .map(|e| quote!(#e))
                .unwrap_or(quote!(""));
            let kind = quote! { ::zennode::ParamKind::Str { default: #default_lit } };
            let default_expr = attrs
                .default
                .as_ref()
                .map(|e| quote!(::zennode::__private::String::from(#e)))
                .unwrap_or_else(|| quote!(::zennode::__private::String::new()));
            Ok(FieldKindResult {
                kind_tokens: kind,
                value_variant: "Str",
                default_expr,
                identity_expr: None,
                is_optional: false,
            })
        }
    }
}

fn gen_to_params(
    field_name: &syn::Ident,
    field_name_str: &str,
    field_type: &syn::Type,
    _variant: &str,
    is_optional: bool,
) -> TokenStream2 {
    let type_str = quote!(#field_type).to_string().replace(' ', "");
    let inner_str = parse_option_inner(&type_str).unwrap_or(&type_str);

    if parse_f32_array(inner_str).is_some() {
        if is_optional {
            return quote! {
                __map.insert(
                    ::zennode::__private::String::from(#field_name_str),
                    match &self.#field_name {
                        ::core::option::Option::Some(__v) => ::zennode::ParamValue::F32Array(::zennode::__private::Vec::from(__v.as_slice())),
                        ::core::option::Option::None => ::zennode::ParamValue::None,
                    },
                );
            };
        }
        return quote! {
            __map.insert(
                ::zennode::__private::String::from(#field_name_str),
                ::zennode::ParamValue::F32Array(::zennode::__private::Vec::from(self.#field_name.as_slice())),
            );
        };
    }

    let value_expr = match inner_str {
        "f32" => quote! { ::zennode::ParamValue::F32 },
        "i32" => quote! { ::zennode::ParamValue::I32 },
        "u32" => quote! { ::zennode::ParamValue::U32 },
        "bool" => quote! { ::zennode::ParamValue::Bool },
        _ => {
            // String or unknown type: use ToString
            if is_optional {
                return quote! {
                    __map.insert(
                        ::zennode::__private::String::from(#field_name_str),
                        match &self.#field_name {
                            ::core::option::Option::Some(__v) => ::zennode::ParamValue::Str(::zennode::__private::ToString::to_string(__v)),
                            ::core::option::Option::None => ::zennode::ParamValue::None,
                        },
                    );
                };
            }
            return quote! {
                __map.insert(::zennode::__private::String::from(#field_name_str), ::zennode::ParamValue::Str(::zennode::__private::ToString::to_string(&self.#field_name)));
            };
        }
    };

    if is_optional {
        quote! {
            __map.insert(
                ::zennode::__private::String::from(#field_name_str),
                match self.#field_name {
                    ::core::option::Option::Some(__v) => #value_expr(__v),
                    ::core::option::Option::None => ::zennode::ParamValue::None,
                },
            );
        }
    } else {
        quote! {
            __map.insert(::zennode::__private::String::from(#field_name_str), #value_expr(self.#field_name));
        }
    }
}

fn gen_get_param(
    field_name: &syn::Ident,
    field_name_str: &str,
    field_type: &syn::Type,
    _variant: &str,
    is_optional: bool,
) -> TokenStream2 {
    let type_str = quote!(#field_type).to_string().replace(' ', "");
    let inner_str = parse_option_inner(&type_str).unwrap_or(&type_str);

    if parse_f32_array(inner_str).is_some() {
        if is_optional {
            return quote! {
                #field_name_str => ::core::option::Option::Some(match &self.#field_name {
                    ::core::option::Option::Some(__v) => ::zennode::ParamValue::F32Array(::zennode::__private::Vec::from(__v.as_slice())),
                    ::core::option::Option::None => ::zennode::ParamValue::None,
                }),
            };
        }
        return quote! {
            #field_name_str => ::core::option::Option::Some(
                ::zennode::ParamValue::F32Array(::zennode::__private::Vec::from(self.#field_name.as_slice()))
            ),
        };
    }

    let value_ctor = match inner_str {
        "f32" => quote! { ::zennode::ParamValue::F32 },
        "i32" => quote! { ::zennode::ParamValue::I32 },
        "u32" => quote! { ::zennode::ParamValue::U32 },
        "bool" => quote! { ::zennode::ParamValue::Bool },
        _ => {
            if is_optional {
                return quote! {
                    #field_name_str => ::core::option::Option::Some(match &self.#field_name {
                        ::core::option::Option::Some(__v) => ::zennode::ParamValue::Str(::zennode::__private::ToString::to_string(__v)),
                        ::core::option::Option::None => ::zennode::ParamValue::None,
                    }),
                };
            }
            return quote! {
                #field_name_str => ::core::option::Option::Some(
                    ::zennode::ParamValue::Str(::zennode::__private::ToString::to_string(&self.#field_name))
                ),
            };
        }
    };

    if is_optional {
        quote! {
            #field_name_str => ::core::option::Option::Some(match self.#field_name {
                ::core::option::Option::Some(__v) => #value_ctor(__v),
                ::core::option::Option::None => ::zennode::ParamValue::None,
            }),
        }
    } else {
        quote! {
            #field_name_str => ::core::option::Option::Some(#value_ctor(self.#field_name)),
        }
    }
}

fn gen_set_param(
    field_name: &syn::Ident,
    field_name_str: &str,
    field_type: &syn::Type,
    _variant: &str,
    is_optional: bool,
) -> TokenStream2 {
    let type_str = quote!(#field_type).to_string().replace(' ', "");
    let inner_str = parse_option_inner(&type_str).unwrap_or(&type_str);

    if let Some(len) = parse_f32_array(inner_str) {
        if is_optional {
            return quote! {
                #field_name_str => {
                    if value.is_none() {
                        self.#field_name = ::core::option::Option::None;
                        return true;
                    }
                    match value.as_f32_array() {
                        ::core::option::Option::Some(arr) if arr.len() == #len => {
                            let mut buf = [0.0f32; #len];
                            buf.copy_from_slice(arr);
                            self.#field_name = ::core::option::Option::Some(buf);
                            true
                        }
                        _ => false,
                    }
                }
            };
        }
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

    let extract = match inner_str {
        "f32" => quote! { value.as_f32() },
        "i32" => quote! { value.as_i32() },
        "u32" => quote! { value.as_u32() },
        "bool" => quote! { value.as_bool() },
        _ => quote! { value.as_str().map(::zennode::__private::ToString::to_string) },
    };

    if is_optional {
        quote! {
            #field_name_str => {
                if value.is_none() {
                    self.#field_name = ::core::option::Option::None;
                    return true;
                }
                match #extract {
                    ::core::option::Option::Some(v) => { self.#field_name = ::core::option::Option::Some(v); true }
                    ::core::option::Option::None => false,
                }
            }
        }
    } else {
        quote! {
            #field_name_str => {
                match #extract {
                    ::core::option::Option::Some(v) => { self.#field_name = v; true }
                    ::core::option::Option::None => false,
                }
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

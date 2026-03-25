//! Attribute parsing for `#[node(...)]`, `#[param(...)]`, and `#[kv(...)]`.

use syn::parse::ParseStream;
use syn::{Expr, Ident, Lit, LitInt, LitStr, Meta, Token};

/// Parsed struct-level `#[node(...)]` attributes.
#[derive(Default)]
pub struct NodeAttrs {
    pub id: Option<LitStr>,
    pub label: Option<LitStr>,
    pub group: Option<Ident>,
    /// Accepts both `role = Filter` and `phase = Filter` (legacy alias).
    pub role: Option<Ident>,
    pub version: Option<u32>,
    pub compat_version: Option<u32>,
    pub fusable: bool,
    pub coalesce: Option<LitStr>,
    pub coalesce_target: bool,
    pub neighborhood: bool,
    pub changes_dimensions: bool,
    pub preferred_format: Option<Ident>,
    pub alpha_handling: Option<Ident>,
    pub tags: Vec<LitStr>,
}

impl NodeAttrs {
    pub fn from_ast(attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut result = Self::default();

        for attr in attrs {
            if !attr.path().is_ident("node") {
                continue;
            }

            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("id") {
                    meta.input.parse::<Token![=]>()?;
                    result.id = Some(meta.input.parse::<LitStr>()?);
                } else if meta.path.is_ident("label") {
                    meta.input.parse::<Token![=]>()?;
                    result.label = Some(meta.input.parse::<LitStr>()?);
                } else if meta.path.is_ident("group") {
                    meta.input.parse::<Token![=]>()?;
                    result.group = Some(meta.input.parse::<Ident>()?);
                } else if meta.path.is_ident("role") || meta.path.is_ident("phase") {
                    meta.input.parse::<Token![=]>()?;
                    result.role = Some(meta.input.parse::<Ident>()?);
                } else if meta.path.is_ident("version") {
                    meta.input.parse::<Token![=]>()?;
                    let lit = meta.input.parse::<LitInt>()?;
                    result.version = Some(lit.base10_parse()?);
                } else if meta.path.is_ident("compat_version") {
                    meta.input.parse::<Token![=]>()?;
                    let lit = meta.input.parse::<LitInt>()?;
                    result.compat_version = Some(lit.base10_parse()?);
                } else if meta.path.is_ident("fusable") {
                    result.fusable = true;
                } else if meta.path.is_ident("coalesce") {
                    meta.input.parse::<Token![=]>()?;
                    result.coalesce = Some(meta.input.parse::<LitStr>()?);
                    result.fusable = true; // coalesce implies fusable
                } else if meta.path.is_ident("coalesce_target") {
                    result.coalesce_target = true;
                } else if meta.path.is_ident("neighborhood") {
                    result.neighborhood = true;
                } else if meta.path.is_ident("changes_dimensions") {
                    result.changes_dimensions = true;
                } else if meta.path.is_ident("format") {
                    let content;
                    syn::parenthesized!(content in meta.input);
                    while !content.is_empty() {
                        let key: Ident = content.parse()?;
                        content.parse::<Token![=]>()?;
                        let val: Ident = content.parse()?;
                        if key == "preferred" {
                            result.preferred_format = Some(val);
                        } else if key == "alpha" {
                            result.alpha_handling = Some(val);
                        }
                        let _ = content.parse::<Token![,]>();
                    }
                } else if meta.path.is_ident("tags") {
                    let content;
                    syn::parenthesized!(content in meta.input);
                    while !content.is_empty() {
                        result.tags.push(content.parse::<LitStr>()?);
                        let _ = content.parse::<Token![,]>();
                    }
                } else {
                    return Err(meta.error("unknown node attribute"));
                }
                Ok(())
            })?;
        }

        Ok(result)
    }
}

/// Parsed field-level `#[param(...)]` attributes.
#[derive(Default)]
pub struct ParamAttrs {
    pub range_min: Option<Expr>,
    pub range_max: Option<Expr>,
    pub default: Option<Expr>,
    pub identity: Option<Expr>,
    pub step: Option<Expr>,
    pub unit: Option<LitStr>,
    pub section: Option<LitStr>,
    pub slider: Option<Ident>,
    pub label: Option<LitStr>,
    pub since: Option<u32>,
    pub labels: Vec<LitStr>,
    pub visible_when: Option<LitStr>,
    pub color: bool,
    pub kv_keys: Vec<LitStr>,
}

impl ParamAttrs {
    pub fn from_ast(attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut result = Self::default();

        for attr in attrs {
            if attr.path().is_ident("param") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("range") {
                        let content;
                        syn::parenthesized!(content in meta.input);
                        let range: syn::ExprRange = content.parse()?;
                        if let Some(start) = range.start {
                            result.range_min = Some(*start);
                        }
                        if let Some(end) = range.end {
                            result.range_max = Some(*end);
                        }
                    } else if meta.path.is_ident("default") {
                        meta.input.parse::<Token![=]>()?;
                        result.default = Some(meta.input.parse::<Expr>()?);
                    } else if meta.path.is_ident("identity") {
                        meta.input.parse::<Token![=]>()?;
                        result.identity = Some(meta.input.parse::<Expr>()?);
                    } else if meta.path.is_ident("step") {
                        meta.input.parse::<Token![=]>()?;
                        result.step = Some(meta.input.parse::<Expr>()?);
                    } else if meta.path.is_ident("unit") {
                        meta.input.parse::<Token![=]>()?;
                        result.unit = Some(meta.input.parse::<LitStr>()?);
                    } else if meta.path.is_ident("section") {
                        meta.input.parse::<Token![=]>()?;
                        result.section = Some(meta.input.parse::<LitStr>()?);
                    } else if meta.path.is_ident("slider") {
                        meta.input.parse::<Token![=]>()?;
                        result.slider = Some(meta.input.parse::<Ident>()?);
                    } else if meta.path.is_ident("label") {
                        meta.input.parse::<Token![=]>()?;
                        result.label = Some(meta.input.parse::<LitStr>()?);
                    } else if meta.path.is_ident("since") {
                        meta.input.parse::<Token![=]>()?;
                        let lit = meta.input.parse::<LitInt>()?;
                        result.since = Some(lit.base10_parse()?);
                    } else if meta.path.is_ident("labels") {
                        let content;
                        syn::parenthesized!(content in meta.input);
                        while !content.is_empty() {
                            result.labels.push(content.parse::<LitStr>()?);
                            let _ = content.parse::<Token![,]>();
                        }
                    } else if meta.path.is_ident("visible_when") {
                        meta.input.parse::<Token![=]>()?;
                        result.visible_when = Some(meta.input.parse::<LitStr>()?);
                    } else if meta.path.is_ident("color") {
                        result.color = true;
                    } else {
                        return Err(meta.error("unknown param attribute"));
                    }
                    Ok(())
                })?;
            } else if attr.path().is_ident("kv") {
                attr.parse_args_with(|input: ParseStream| {
                    while !input.is_empty() {
                        result.kv_keys.push(input.parse::<LitStr>()?);
                        let _ = input.parse::<Token![,]>();
                    }
                    Ok(())
                })?;
            }
        }

        Ok(result)
    }
}

/// Extract doc comment lines from attributes as a single string.
pub fn extract_doc_comment(attrs: &[syn::Attribute]) -> String {
    let mut lines = Vec::new();
    for attr in attrs {
        if attr.path().is_ident("doc") {
            if let Meta::NameValue(nv) = &attr.meta {
                if let Expr::Lit(expr_lit) = &nv.value {
                    if let Lit::Str(s) = &expr_lit.lit {
                        lines.push(s.value().trim().to_string());
                    }
                }
            }
        }
    }
    lines.join(" ").trim().to_string()
}

/// Convert a PascalCase identifier to a space-separated label.
/// e.g., "AdaptiveSharpen" -> "Adaptive Sharpen"
pub fn ident_to_label(name: &str) -> String {
    let mut result = String::new();
    for (i, ch) in name.chars().enumerate() {
        if i > 0 && ch.is_uppercase() {
            result.push(' ');
        }
        result.push(ch);
    }
    result
}

/// Convert a snake_case field name to a title-cased label.
/// e.g., "noise_floor" -> "Noise Floor"
pub fn snake_to_label(name: &str) -> String {
    name.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => {
                    let mut s = c.to_uppercase().to_string();
                    s.extend(chars);
                    s
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

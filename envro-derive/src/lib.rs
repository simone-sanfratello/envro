use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, Attribute, Data, DeriveInput, Error, Expr, ExprLit, Fields, Lit,
    Result as SynResult, Type,
};

#[proc_macro_derive(Envro, attributes(envro))]
pub fn derive_envro(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

struct FieldInfo {
    name: syn::Ident,
    env_key: String,
    optional: bool,
    default: Option<String>,
    ty_kind: TyKind,
    rules: Vec<RuleAttr>,
}

#[derive(Clone, Copy)]
enum TyKind {
    String,
    Bool,
    I32,
    I64,
    U16,
    U32,
    U64,
    F32,
    F64,
}

enum RuleAttr {
    Flag(&'static str),
    MinLen(usize),
    MaxLen(usize),
    ExactLen(usize),
    StartsWith(String),
    EndsWith(String),
    Contains(String),
    IntRange(i64, i64),
    FloatRange(f64, f64),
    OneOf(Vec<String>),
    NotOneOf(Vec<String>),
}

fn expand(input: DeriveInput) -> SynResult<proc_macro2::TokenStream> {
    let name = &input.ident;
    let Data::Struct(data) = &input.data else {
        return Err(Error::new_spanned(
            &input,
            "Envro can only be derived for structs",
        ));
    };
    let Fields::Named(fields) = &data.fields else {
        return Err(Error::new_spanned(
            &input,
            "Envro requires a struct with named fields",
        ));
    };

    let mut infos = Vec::new();
    for field in &fields.named {
        let ident = field
            .ident
            .as_ref()
            .ok_or_else(|| Error::new_spanned(field, "expected named field"))?;
        let (inner_ty, optional) = unwrap_option(&field.ty)?;
        let ty_kind = map_type(inner_ty, ident)?;
        let attrs = parse_envro_attrs(&field.attrs)?;
        let env_key = attrs
            .from
            .unwrap_or_else(|| ident.to_string().to_ascii_uppercase());
        if optional && attrs.default.is_none() {
            return Err(Error::new_spanned(
                ident,
                "optional fields (Option<T>) require #[envro(default = \"...\")]",
            ));
        }
        check_type_rules(ty_kind, &attrs.rules, ident)?;
        infos.push(FieldInfo {
            name: ident.clone(),
            env_key,
            optional,
            default: attrs.default,
            ty_kind,
            rules: attrs.rules,
        });
    }

    let schema_fields = infos.iter().map(|f| {
        let key = &f.env_key;
        let presence = match &f.default {
            Some(default) => quote! { ::envro::Field::default_value(#default) },
            None => quote! { ::envro::Field::required() },
        };
        let rules = f.rules.iter().map(rule_tokens);
        quote! {
            .field(#key, #presence #(.#rules)*)
        }
    });

    let env_keys = infos.iter().map(|f| &f.env_key);

    let coerce_fields = infos.iter().map(|f| {
        let name = &f.name;
        let key = &f.env_key;
        let fn_name = coerce_fn_name(f.ty_kind, f.optional);
        quote! {
            #name: ::envro::coerce::#fn_name(&vars, #key)?
        }
    });

    Ok(quote! {
        impl ::envro::EnvroConfig for #name {
            fn schema() -> ::envro::Schema {
                ::envro::Schema::new()
                    #(#schema_fields)*
            }

            fn from_vars(vars: &::envro::EnvroVars) -> ::std::result::Result<Self, ::envro::EnvroError> {
                let schema = Self::schema();
                let vars = schema.apply_defaults(vars);
                ::envro::validate(&vars, &schema)?;
                ::std::result::Result::Ok(Self {
                    #(#coerce_fields),*
                })
            }

            fn from_env() -> ::std::result::Result<Self, ::envro::EnvroError> {
                let mut vars = ::envro::EnvroVars::new();
                for key in [#(#env_keys),*] {
                    if let ::std::result::Result::Ok(v) = ::std::env::var(key) {
                        if !v.is_empty() {
                            vars.insert(key.to_string(), v);
                        }
                    }
                }
                // Fast path: skip the whole resolver when no value even
                // contains `$`. This is the common CD case (flat knobs).
                let needs_expand = vars.values().any(|v| v.as_bytes().contains(&b'$'));
                let vars = if needs_expand { ::envro::expand_vars(&vars) } else { vars };
                Self::from_vars(&vars)
            }
        }
    })
}

fn coerce_fn_name(kind: TyKind, optional: bool) -> syn::Ident {
    let base = match kind {
        TyKind::String => "string",
        TyKind::Bool => "bool",
        TyKind::I32 => "i32",
        TyKind::I64 => "i64",
        TyKind::U16 => "u16",
        TyKind::U32 => "u32",
        TyKind::U64 => "u64",
        TyKind::F32 => "f32",
        TyKind::F64 => "f64",
    };
    if optional {
        format_ident!("optional_{base}")
    } else {
        format_ident!("require_{base}")
    }
}

fn rule_tokens(rule: &RuleAttr) -> proc_macro2::TokenStream {
    match rule {
        RuleAttr::Flag(name) => {
            let id = format_ident!("{name}");
            quote! { #id() }
        }
        RuleAttr::MinLen(n) => quote! { min_len(#n) },
        RuleAttr::MaxLen(n) => quote! { max_len(#n) },
        RuleAttr::ExactLen(n) => quote! { exact_len(#n) },
        RuleAttr::StartsWith(s) => quote! { starts_with(#s) },
        RuleAttr::EndsWith(s) => quote! { ends_with(#s) },
        RuleAttr::Contains(s) => quote! { contains(#s) },
        RuleAttr::IntRange(min, max) => quote! { int_range(#min, #max) },
        RuleAttr::FloatRange(min, max) => quote! { float_range(#min, #max) },
        RuleAttr::OneOf(opts) => quote! { one_of(&[#(#opts),*]) },
        RuleAttr::NotOneOf(opts) => quote! { not_one_of(&[#(#opts),*]) },
    }
}

struct ParsedAttrs {
    from: Option<String>,
    default: Option<String>,
    rules: Vec<RuleAttr>,
}

fn parse_envro_attrs(attrs: &[Attribute]) -> SynResult<ParsedAttrs> {
    let mut out = ParsedAttrs {
        from: None,
        default: None,
        rules: Vec::new(),
    };
    for attr in attrs {
        if !attr.path().is_ident("envro") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            let ident = meta
                .path
                .get_ident()
                .ok_or_else(|| meta.error("expected identifier"))?
                .to_string();
            match ident.as_str() {
                "from" => {
                    let value: Lit = meta.value()?.parse()?;
                    let Lit::Str(s) = value else {
                        return Err(meta.error("from expects a string"));
                    };
                    out.from = Some(s.value());
                }
                "default" => {
                    let value: Lit = meta.value()?.parse()?;
                    let Lit::Str(s) = value else {
                        return Err(meta.error("default expects a string"));
                    };
                    out.default = Some(s.value());
                }
                "min_len" => out.rules.push(RuleAttr::MinLen(parse_usize(&meta)?)),
                "max_len" => out.rules.push(RuleAttr::MaxLen(parse_usize(&meta)?)),
                "exact_len" => out.rules.push(RuleAttr::ExactLen(parse_usize(&meta)?)),
                "starts_with" => out.rules.push(RuleAttr::StartsWith(parse_str(&meta)?)),
                "ends_with" => out.rules.push(RuleAttr::EndsWith(parse_str(&meta)?)),
                "contains" => out.rules.push(RuleAttr::Contains(parse_str(&meta)?)),
                "int_range" => {
                    let (min, max) = parse_pair_i64(&meta)?;
                    out.rules.push(RuleAttr::IntRange(min, max));
                }
                "float_range" => {
                    let (min, max) = parse_pair_f64(&meta)?;
                    out.rules.push(RuleAttr::FloatRange(min, max));
                }
                "one_of" => out.rules.push(RuleAttr::OneOf(parse_str_list(&meta)?)),
                "not_one_of" => out.rules.push(RuleAttr::NotOneOf(parse_str_list(&meta)?)),
                // flag rules (no value)
                "port"
                | "boolean"
                | "integer"
                | "positive_integer"
                | "non_negative_integer"
                | "float"
                | "positive_float"
                | "non_negative_float"
                | "email"
                | "url"
                | "uuid"
                | "ipv4"
                | "ip"
                | "hex"
                | "alpha"
                | "alphanumeric"
                | "digits"
                | "ascii"
                | "lowercase"
                | "uppercase" => {
                    if meta.input.peek(syn::Token![=]) {
                        return Err(meta.error(format!("{ident} takes no value")));
                    }
                    // leak is fine for compile-time static rule names in generated code —
                    // we use &'static str via match arm instead:
                    let flag: &'static str = match ident.as_str() {
                        "port" => "port",
                        "boolean" => "boolean",
                        "integer" => "integer",
                        "positive_integer" => "positive_integer",
                        "non_negative_integer" => "non_negative_integer",
                        "float" => "float",
                        "positive_float" => "positive_float",
                        "non_negative_float" => "non_negative_float",
                        "email" => "email",
                        "url" => "url",
                        "uuid" => "uuid",
                        "ipv4" => "ipv4",
                        "ip" => "ip",
                        "hex" => "hex",
                        "alpha" => "alpha",
                        "alphanumeric" => "alphanumeric",
                        "digits" => "digits",
                        "ascii" => "ascii",
                        "lowercase" => "lowercase",
                        "uppercase" => "uppercase",
                        _ => unreachable!(),
                    };
                    out.rules.push(RuleAttr::Flag(flag));
                }
                other => return Err(meta.error(format!("unknown envro attribute: {other}"))),
            }
            Ok(())
        })?;
    }
    Ok(out)
}

fn parse_usize(meta: &syn::meta::ParseNestedMeta<'_>) -> SynResult<usize> {
    let value: Lit = meta.value()?.parse()?;
    match value {
        Lit::Int(i) => i.base10_parse(),
        _ => Err(meta.error("expected integer")),
    }
}

fn parse_str(meta: &syn::meta::ParseNestedMeta<'_>) -> SynResult<String> {
    let value: Lit = meta.value()?.parse()?;
    match value {
        Lit::Str(s) => Ok(s.value()),
        _ => Err(meta.error("expected string")),
    }
}

fn parse_pair_i64(meta: &syn::meta::ParseNestedMeta<'_>) -> SynResult<(i64, i64)> {
    let content;
    syn::parenthesized!(content in meta.input);
    let a: Lit = content.parse()?;
    content.parse::<syn::Token![,]>()?;
    let b: Lit = content.parse()?;
    let Lit::Int(a) = a else {
        return Err(meta.error("int_range expects integers"));
    };
    let Lit::Int(b) = b else {
        return Err(meta.error("int_range expects integers"));
    };
    Ok((a.base10_parse()?, b.base10_parse()?))
}

fn parse_pair_f64(meta: &syn::meta::ParseNestedMeta<'_>) -> SynResult<(f64, f64)> {
    let content;
    syn::parenthesized!(content in meta.input);
    let a: Expr = content.parse()?;
    content.parse::<syn::Token![,]>()?;
    let b: Expr = content.parse()?;
    Ok((expr_to_f64(&a)?, expr_to_f64(&b)?))
}

fn expr_to_f64(expr: &Expr) -> SynResult<f64> {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Float(f), ..
        }) => f.base10_parse(),
        Expr::Lit(ExprLit {
            lit: Lit::Int(i), ..
        }) => i.base10_parse::<f64>(),
        Expr::Unary(u) if matches!(u.op, syn::UnOp::Neg(_)) => Ok(-expr_to_f64(&u.expr)?),
        _ => Err(Error::new_spanned(expr, "expected float literal")),
    }
}

fn parse_str_list(meta: &syn::meta::ParseNestedMeta<'_>) -> SynResult<Vec<String>> {
    let content;
    syn::parenthesized!(content in meta.input);
    let mut out = Vec::new();
    while !content.is_empty() {
        let lit: Lit = content.parse()?;
        let Lit::Str(s) = lit else {
            return Err(meta.error("expected string list"));
        };
        out.push(s.value());
        if content.peek(syn::Token![,]) {
            content.parse::<syn::Token![,]>()?;
        }
    }
    Ok(out)
}

fn unwrap_option(ty: &Type) -> SynResult<(&Type, bool)> {
    if let Type::Path(p) = ty {
        if p.qself.is_none() && p.path.segments.len() == 1 {
            let seg = &p.path.segments[0];
            if seg.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return Ok((inner, true));
                    }
                }
                return Err(Error::new_spanned(
                    ty,
                    "Option must have one type parameter",
                ));
            }
        }
    }
    Ok((ty, false))
}

fn map_type(ty: &Type, field: &syn::Ident) -> SynResult<TyKind> {
    let Type::Path(p) = ty else {
        return Err(Error::new_spanned(
            field,
            "unsupported field type for Envro",
        ));
    };
    if p.qself.is_some() || p.path.segments.len() != 1 {
        return Err(Error::new_spanned(
            field,
            "unsupported field type for Envro (use a primitive or String)",
        ));
    }
    match p.path.segments[0].ident.to_string().as_str() {
        "String" => Ok(TyKind::String),
        "bool" => Ok(TyKind::Bool),
        "i32" => Ok(TyKind::I32),
        "i64" => Ok(TyKind::I64),
        "u16" => Ok(TyKind::U16),
        "u32" => Ok(TyKind::U32),
        "u64" => Ok(TyKind::U64),
        "f32" => Ok(TyKind::F32),
        "f64" => Ok(TyKind::F64),
        other => Err(Error::new_spanned(
            field,
            format!("unsupported Envro field type `{other}`"),
        )),
    }
}

fn check_type_rules(ty: TyKind, rules: &[RuleAttr], field: &syn::Ident) -> SynResult<()> {
    for rule in rules {
        if let RuleAttr::Flag("port") = rule {
            if !matches!(ty, TyKind::U16) {
                return Err(Error::new_spanned(
                    field,
                    "`port` rule requires field type `u16`",
                ));
            }
        }
        if let RuleAttr::Flag("boolean") = rule {
            if !matches!(ty, TyKind::Bool) {
                return Err(Error::new_spanned(
                    field,
                    "`boolean` rule requires field type `bool`",
                ));
            }
        }
    }
    Ok(())
}

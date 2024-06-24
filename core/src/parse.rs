use deluxe::ParseMetaItem;
use derive_syn_parse::Parse;
use itertools::Itertools;
use proc_macro2::TokenStream;
use quote::format_ident;
use syn::{
    parse::{Parse, ParseStream},
    parse_quote, parse_quote_spanned,
    punctuated::Punctuated,
    spanned::Spanned,
    token::Comma,
    Attribute, Error, Expr, FnArg, Generics, Ident, LitStr, PatType, Token,
};

pub struct RouteMeta {
    pub http_path: LitStr,
    pub http_method: Ident,
}

#[derive(ParseMetaItem)]
struct ServerFnsArgs {
    pub path: Option<String>,
    pub method: Option<String>,
}

impl RouteMeta {
    pub fn parse(args: TokenStream, ident: &Ident) -> syn::Result<RouteMeta> {
        let ServerFnsArgs { path, method } = deluxe::parse2(args)?;

        Ok(Self {
            http_path: path
                .or_else(|| Some(format!("/api/{ident}")))
                .map(|p| {
                    let path = p.replace('_', "-").to_lowercase();
                    parse_quote!(#path)
                })
                .unwrap(),
            http_method: method
                .as_deref()
                .or(Some("post"))
                .map(|m| format_ident!("{}", m.to_lowercase()))
                .unwrap(),
        })
    }
}

#[derive(Default)]
pub struct HandlerArgs {
    pub inner: Vec<FnArg>,
    pub outer: Vec<OuterArg>,
}

pub struct OuterArg {
    pub arg: FnArg,
    pub gen: Option<Generics>,
}

struct ArgGroup {
    inner: FnArg,
    outer: OuterArg,
}

impl TryFrom<Punctuated<FnArg, Comma>> for HandlerArgs {
    type Error = syn::Error;

    fn try_from(args: Punctuated<FnArg, Comma>) -> Result<Self, Self::Error> {
        args.into_iter()
            .enumerate()
            .map(ArgGroup::try_from)
            .fold_ok(HandlerArgs::default(), HandlerArgs::push)
    }
}

impl HandlerArgs {
    fn push(mut self, ArgGroup { inner, outer }: ArgGroup) -> Self {
        self.inner.push(inner);
        self.outer.push(outer);
        self
    }
}

impl TryFrom<(usize, FnArg)> for ArgGroup {
    type Error = syn::Error;

    fn try_from((i, fn_arg): (usize, FnArg)) -> Result<Self, Self::Error> {
        let state_attr = parse_quote!(#[state]);

        match fn_arg {
            FnArg::Receiver(rec) => Err(Self::Error::new(
                rec.span(),
                "Reciever type 'self' not allowed in server functions.",
            )),
            FnArg::Typed(
                ref param @ PatType {
                    ref attrs,
                    ref pat,
                    ref ty,
                    ..
                },
            ) if attrs.contains(&state_attr) => {
                let generic_state = format_ident!("State{i}");

                Ok(ArgGroup {
                    outer: OuterArg {
                        arg: parse_quote_spanned! { param.span() =>
                            #(#attrs)* ::axum::extract::State(#pat): ::axum::extract::State<#ty>
                        },
                        gen: Some(Generics {
                            params: parse_quote_spanned! { param.span() => #generic_state },
                            where_clause: Some(parse_quote_spanned! { param.span() =>
                                where
                                    #ty: ::axum::extract::FromRef<#generic_state>,
                                    #generic_state: ::std::marker::Send + ::std::marker::Sync
                            }),
                            ..Generics::default()
                        }),
                    },
                    inner: parse_quote_spanned! { param.span() => #param },
                })
            }
            param => Ok(ArgGroup {
                inner: param.clone(),
                outer: OuterArg {
                    arg: param,
                    gen: None,
                },
            }),
        }
    }
}

#[derive(Parse)]
pub struct Middleware(pub Expr);

pub trait CollectMiddleware {
    fn collect_middleware(&mut self) -> Vec<Middleware>;
}

impl CollectMiddleware for Vec<Attribute> {
    fn collect_middleware(&mut self) -> Vec<Middleware> {
        let mut middlewares = Vec::<Middleware>::new();
        self.retain(|attr| {
            if attr.path().is_ident("middleware") {
                middlewares.push(attr.parse_args().expect("Expected middleware expression."));
                false
            } else {
                true
            }
        });

        middlewares
    }
}

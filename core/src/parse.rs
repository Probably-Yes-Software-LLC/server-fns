use deluxe::ParseMetaItem;
use derive_syn_parse::Parse;
use proc_macro2::TokenStream;
use quote::format_ident;
use syn::{parse_quote, Attribute, Expr, Ident, LitStr};

pub struct RouteMeta {
    pub http_path: LitStr,
    pub http_method: Ident
}

#[derive(ParseMetaItem)]
struct ServerFnsArgs {
    pub path: Option<String>,
    pub method: Option<String>
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
                .unwrap()
        })
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

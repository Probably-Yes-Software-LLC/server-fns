use proc_macro2::TokenStream as TokenStream2;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Attribute, Error, Expr, Token,
};

pub struct RouteMeta {
    pub path: Option<String>,
    pub method: Option<String>,
}

impl Parse for RouteMeta {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let parser = Punctuated::<TokenStream2, Token![,]>::parse_terminated;
        let punct = input.call(parser)?;

        let mut path = None;
        let mut method = None;

        for meta_item in punct {
            let meta = format!("{meta_item}")
                .replace(' ', "")
                .replace('_', "-")
                .to_lowercase();

            match &*meta {
                "get" | "post" => {
                    method = Some(meta);
                }
                p if p.starts_with('/') => {
                    path = Some(meta);
                }
                _ => {
                    return Err(Error::new_spanned(
                        meta_item,
                        format!("Expected GET, POST, or API path '/abc'; found {meta}"),
                    ));
                }
            }
        }

        Ok(Self { path, method })
    }
}

pub struct Middleware(pub Expr);

impl Parse for Middleware {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let expr = input.parse()?;
        Ok(Self(expr))
    }
}

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

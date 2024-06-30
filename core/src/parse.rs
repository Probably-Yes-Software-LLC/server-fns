use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    parse_quote, parse_quote_spanned, Expr, Ident, LitStr, MetaNameValue, Token
};

mod kw {
    use syn::custom_keyword;

    custom_keyword!(middleware);
    custom_keyword!(after);
    custom_keyword!(before);
    custom_keyword!(routing);
}

#[derive(Default)]
pub struct ServerFnArgs {
    pub path: Option<LitStr>,
    pub method: Option<Ident>,
    pub middlewares: Vec<Middleware>
}

pub struct Middleware {
    pub strat: RoutingStrategy,
    pub expr: Expr
}

pub enum RoutingStrategy {
    AfterRouting,
    BeforeRouting
}

impl Parse for ServerFnArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let span = input.span();

        input
            .parse_terminated(MetaNameValue::parse, Token![,])?
            .into_iter()
            .fold(Ok(Self::default()), |args, next| {
                let Ok(mut args) = args else {
                    return args;
                };

                if next.path.is_ident("path") {
                    let path = next.value;
                    args.path = parse_quote_spanned! { span => #path };
                } else if next.path.is_ident("method") {
                    let method = next.value;
                    args.method = parse_quote_spanned! { span => #method };
                } else if next.path.is_ident("middlewares") {
                    let Expr::Array(mids) = next.value else {
                        return Err(syn::Error::new(span, "Unexpected middlewares array value."));
                    };
                    args.middlewares = mids
                        .elems
                        .into_iter()
                        .map(|mid| syn::parse2(mid.into_token_stream()))
                        .collect::<Result<Vec<_>, _>>()?;
                }

                Ok(args)
            })
    }
}

impl Parse for Middleware {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<kw::middleware>()?;

        let content;
        parenthesized!(content in input);

        let strat;
        if content.peek(kw::after) {
            content.parse::<kw::after>()?;
            content.parse::<kw::routing>()?;
            strat = RoutingStrategy::AfterRouting;
        } else if content.peek(kw::before) {
            content.parse::<kw::before>()?;
            content.parse::<kw::routing>()?;
            strat = RoutingStrategy::BeforeRouting;
        } else {
            strat = RoutingStrategy::AfterRouting;
        }

        let expr = content.parse()?;

        Ok(Middleware { strat, expr })
    }
}

impl ToTokens for ServerFnArgs {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Self {
            path,
            method,
            middlewares
        } = self;

        if let Some(path) = path {
            tokens.append_all(quote! { path = #path, });
        }

        if let Some(method) = method {
            let method = LitStr::new(&method.to_string(), Span::mixed_site());
            tokens.append_all(quote! { method = #method, });
        }

        if !middlewares.is_empty() {
            tokens.append_all(quote! { middlewares = [#(#middlewares),*] });
        }
    }
}

impl ToTokens for Middleware {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Self { strat, expr } = self;

        tokens.append_all(match strat {
            RoutingStrategy::AfterRouting => quote! { middleware(after routing { #expr }) },
            RoutingStrategy::BeforeRouting => quote! { middleware(before routing { #expr }) }
        });
    }
}

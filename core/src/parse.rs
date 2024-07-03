use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned, ToTokens, TokenStreamExt};
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    parse_quote, parse_quote_spanned,
    punctuated::Punctuated,
    spanned::Spanned,
    token::Comma,
    Expr, ExprLit, ExprPath, Ident, Lit, LitStr, MetaNameValue, Token
};

#[macro_export]
macro_rules! http_methods {
    (as slice) => {{
        &[
            $crate::http_methods! {
                foreach! (stringify) (sep ,)
            }
        ];
    }};
    (foreach! ($macro:ident)) => {
        $crate::http_methods! {
            @foreach [any delete get head options patch post put trace]
            do ($macro)
        }
    };
    (@foreach [$($method:ident)+] do ($macro:ident)) => {
        $(
            $macro! { $method }
        )+
    };
    (@foreach [$($method:ident,)+] do ($macro:ident)) => {
        $(
            $macro!($method)
        )+
    };
}

const SUPPORTED_HTTP_METHODS: [&str; 9] = [
    "any", "delete", "get", "head", "options", "patch", "post", "put", "trace"
];

#[derive(Debug, Default, PartialEq)]
pub struct ServerFnArgs {
    pub path: Option<LitStr>,
    pub method: Option<Ident>,
    pub middlewares: Vec<Middleware>
}

#[derive(Debug, PartialEq)]
pub struct Middleware {
    pub expr: Expr
}

impl Parse for ServerFnArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let span = input.span();

        let metas = input.parse_terminated(MetaNameValue::parse, Token![,]);
        let metas = match metas {
            Ok(m) => m,
            Err(mut err) => {
                let dbg = syn::Error::new(
                    span,
                    format!("Error parsing MetaNameValue punctuated sequence; found ({input})")
                );
                err.combine(dbg);

                return Err(err);
            }
        };

        // let x = http_methods!(as slice);

        metas
            .into_iter()
            .try_fold(Self::default(), |mut args, next| {
                if next.path.is_ident("path") {
                    match next.value {
                        Expr::Lit(ExprLit {
                            lit: Lit::Str(litstr),
                            ..
                        }) => args.path = Some(litstr),
                        unexpected => {
                            return Err(syn::Error::new(
                                unexpected.span(),
                                format!("Path must be a string literal; found ({unexpected:?})")
                            ));
                        }
                    }
                } else if next.path.is_ident("method") {
                    match next.value {
                        Expr::Lit(ExprLit {
                            lit: Lit::Str(litstr),
                            ..
                        }) if SUPPORTED_HTTP_METHODS
                            .contains(&litstr.value().to_lowercase().as_ref()) =>
                        {
                            args.method = Some(Ident::new(&litstr.value(), litstr.span()))
                        }
                        Expr::Lit(ExprLit {
                            lit: Lit::Str(litstr),
                            ..
                        }) => {
                            return Err(syn::Error::new(
                                litstr.span(),
                                format!(
                                    "Method not supported; found ({:?}), expected one of{:?}",
                                    litstr, SUPPORTED_HTTP_METHODS
                                )
                            ));
                        }
                        unexpected => {
                            return Err(syn::Error::new(
                                unexpected.span(),
                                format!("Method must be a string literal; found ({unexpected:?})")
                            ));
                        }
                    }
                } else if next.path.is_ident("middlewares") {
                    let Expr::Array(mids) = next.value else {
                        return Err(syn::Error::new(span, "Unexpected middlewares array value."));
                    };
                    args.middlewares = mids
                        .elems
                        .into_iter()
                        .map(|mid| syn::parse2(mid.into_token_stream()))
                        .collect::<Result<Vec<_>, _>>()?;
                } else {
                    return Err(syn::Error::new(
                        next.span(),
                        format!(
                            "Unexpected server attribute argument: {:?}",
                            next.path.get_ident()
                        )
                    ));
                }

                Ok(args)
            })
    }
}

impl Parse for Middleware {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        fn after_routing<T: ToTokens + Spanned>(expr: T) -> Expr {
            parse_quote_spanned! { expr.span() =>
                after_routing(#expr)
            }
        }

        let expr = match input.parse::<Expr>() {
            Ok(expr) => expr,
            Err(mut err) => {
                let dbg = syn::Error::new(
                    input.span(),
                    format!("Error parsing middleware args; found ({input})")
                );
                err.combine(dbg);

                return Err(err);
            }
        };

        let expr = if let Expr::Call(call) = expr {
            // Check that the given middleware expr is configured for routing.
            if let Expr::Path(ExprPath { path, .. }) = call.func.as_ref() {
                if path.is_ident("after_routing") || path.is_ident("before_routing") {
                    Expr::Call(call)
                }
                // Call expr isn't a routing function.
                else {
                    after_routing(call)
                }
            }
            // Call expr isn't a path.
            else {
                after_routing(call)
            }
        }
        // Not a call expr.
        else {
            after_routing(expr)
        };

        Ok(Middleware { expr })
    }
}

impl ToTokens for ServerFnArgs {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Self {
            path,
            method,
            middlewares
        } = self;

        let mut args = Punctuated::<MetaNameValue, Comma>::new();

        if let Some(path) = path {
            args.push(parse_quote! { path = #path });
        }

        if let Some(method) = method {
            let method = LitStr::new(&method.to_string(), method.span());
            args.push(parse_quote! { method = #method });
        }

        if !middlewares.is_empty() {
            args.push(parse_quote! { middlewares = [#(#middlewares),*] });
        }

        tokens.append_all(args.into_pairs());
    }
}

impl ToTokens for Middleware {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Self { expr } = self;

        tokens.append_all(quote! { #expr });
    }
}

#[cfg(test)]
mod test {
    use seq_macro::seq;

    use super::*;

    macro_rules! test_parse_method {
        ($method:expr) => {{
            let method = $method;
            let tokens = quote! {
                path = "/test",
                method = #method,
                middlewares = [
                    after_routing(fn_after),
                    before_routing(fn_before)
                ]
            };
            let server_fn_args: ServerFnArgs = syn::parse2(tokens.clone()).unwrap();
            let expected = ServerFnArgs {
                path: parse_quote!("/test"),
                method: Some(Ident::new(method, Span::call_site())),
                middlewares: vec![
                    parse_quote!(after_routing(fn_after)),
                    parse_quote!(before_routing(fn_before)),
                ]
            };
            assert_eq!(server_fn_args, expected);
            assert_eq!(
                server_fn_args.to_token_stream().to_string(),
                tokens.to_string()
            );
        }};
    }

    mod panics {
        use super::*;

        #[test]
        #[should_panic(expected = "Method not supported")]
        fn parse_bad_http_method() {
            test_parse_method!("ping");
        }
    }

    #[test]
    fn parse_server_fn_args() {
        seq!(N in 0..9 {
            test_parse_method!(SUPPORTED_HTTP_METHODS[N]);
        });
    }

    #[test]
    fn parse_middleware() {
        let tokens = quote! {
            axum::middleware::from_fn(some_fn)
        };
        let expr: Expr = parse_quote! {
            after_routing(axum::middleware::from_fn(some_fn))
        };
        let middleware: Middleware = syn::parse2(tokens).unwrap();
        let expected = Middleware { expr: expr.clone() };
        assert_eq!(middleware, expected);
        assert_eq!(
            middleware.to_token_stream().to_string(),
            expr.to_token_stream().to_string()
        );
    }
}

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned, ToTokens, TokenStreamExt};
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    parse_quote, parse_quote_spanned,
    punctuated::Punctuated,
    spanned::Spanned,
    token::Comma,
    Expr, ExprCall, ExprLit, Ident, Lit, LitStr, MetaNameValue, Token
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
                        }) => args.method = Some(Ident::new(&litstr.value(), litstr.span())),
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
        let expr = input.parse();
        let expr = match expr {
            Ok(Expr::Call(call)) => {
                let func = call.func.as_ref();
                let call = match func {
                    Expr::Path(expr_path)
                        if expr_path.path.is_ident("after_routing")
                            || expr_path.path.is_ident("before_routing") =>
                    {
                        call
                    }
                    _ => parse_quote_spanned! { call.span() => #call }
                };
                Expr::Call(call)
            }
            Ok(expr) => parse_quote_spanned! { expr.span() => after_routing(#expr) },
            Err(mut err) => {
                let dbg = syn::Error::new(
                    input.span(),
                    format!("Error parsing middleware args; found ({input})")
                );
                err.combine(dbg);

                return Err(err);
            }
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
            args.push(parse_quote! { method = #method, });
        }

        if !middlewares.is_empty() {
            args.push(parse_quote! { middlewares = [#(#middlewares),*] });
        }

        tokens.append_all(args);
    }
}

impl ToTokens for Middleware {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Self { expr } = self;

        tokens.append_all(quote! { #expr });
    }
}

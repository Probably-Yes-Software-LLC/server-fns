use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote_spanned, ToTokens, TokenStreamExt};
use syn::{parse_quote_spanned, spanned::Spanned, ItemFn};

use crate::{http_methods, parse::ServerFnArgs};

pub struct MiddlewareImpl(pub Span, pub ItemFn);

impl MiddlewareImpl {
    pub fn try_new(args: TokenStream2, mut server_fn: ItemFn) -> Result<Self, syn::Error> {
        let span = server_fn.span();

        let mut server_attr = Ok(None);
        server_fn
            .attrs
            .retain(|attr| match (attr.path().get_ident(), &server_attr) {
                // Server attr has already error; just fast track out of the loop.
                (_, Err(_)) => true,
                // Next ident matches expected value, but server attr already has a value.
                (Some(ident), Ok(Some(first)))
                    if ident == "server"
                        || http_methods!(contains!(&ident.to_string().as_str())) =>
                {
                    server_attr = Err(syn::Error::new(
                        span,
                        format!(
                            "Multiple server function attributes matched; found {:?}, then {:?}",
                            first, attr
                        )
                    ));
                    false
                }
                // Next ident matches expected value and the server attr isn't set.
                (Some(ident), Ok(None))
                    if ident == "server"
                        || http_methods!(contains!(&ident.to_string().as_str())) =>
                {
                    server_attr = Ok(Some(attr.clone()));
                    false
                }
                // Next ident isn't an expected value.
                (_, _) => true
            });

        let Some(server_attr) = server_attr? else {
            return Err(syn::Error::new(span, "#[server] attribute not found"));
        };

        // Parse out the server function arguments so the middleware expression can be added back in.
        let server_attr_span = server_attr.span();
        let attr_args = server_attr.parse_args::<ServerFnArgs>();
        let mut attr_args = match attr_args {
            Ok(attr_args) => attr_args,
            Err(mut err) => {
                let dbg = syn::Error::new(
                    server_attr_span,
                    "Error parsing server attribute arguments."
                );
                err.combine(dbg);

                return Err(err);
            }
        };

        let arg_span = args.span();
        // Parse out the middleware expression.
        let middleware = match syn::parse2(args) {
            Ok(m) => m,
            Err(mut err) => {
                let dbg = syn::Error::new(arg_span, "Error parsing middleware attribute arguments");
                err.combine(dbg);

                return Err(err);
            }
        };

        // Add this middleware to the server function args so it can be rewritten back onto the fn.
        attr_args.middlewares.push(middleware);

        // Write out the same server function attribute name that was found.
        let attr_path = server_attr.path();

        server_fn
            .attrs
            .push(parse_quote_spanned! { span => #[#attr_path(#attr_args)] });

        Ok(Self(span, server_fn))
    }
}

impl ToTokens for MiddlewareImpl {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Self(span, server_fn) = self;

        tokens.append_all(quote_spanned! { *span => #server_fn });
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn new_middleware() {}
}

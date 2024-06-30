use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote_spanned, ToTokens, TokenStreamExt};
use syn::{parse_quote_spanned, spanned::Spanned, ItemFn};

use crate::parse::ServerFnArgs;

pub struct MiddlewareImpl(pub Span, pub ItemFn);

impl MiddlewareImpl {
    pub fn try_new(args: TokenStream2, mut server_fn: ItemFn) -> Result<Self, syn::Error> {
        let span = server_fn.span();

        let mut server_attr = None;
        server_fn.attrs.retain(|attr| {
            if attr.path().is_ident("server") {
                server_attr = Some(attr.clone());
                false
            } else {
                true
            }
        });

        let Some(server_attr) = server_attr else {
            return Err(syn::Error::new(span, "#[server] attribute not found"));
        };
        let mut attr_args: ServerFnArgs = server_attr.parse_args()?;
        attr_args.middlewares.push(syn::parse2(args)?);

        server_fn
            .attrs
            .push(parse_quote_spanned! { span => #[server(#attr_args)] });

        Ok(Self(span, server_fn))
    }
}

impl ToTokens for MiddlewareImpl {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Self(span, server_fn) = self;

        tokens.append_all(quote_spanned! { *span => #server_fn });
    }
}

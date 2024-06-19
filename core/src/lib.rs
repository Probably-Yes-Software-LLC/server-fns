mod macro_traits;
mod parse;

#[cfg(feature = "axum")]
pub mod axum_router;

pub use macro_traits::AttrMacro;
use parse::{CollectMiddleware, RouteMeta};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{parse_quote, Expr, ItemFn, LitStr};

pub struct ServerFnsAttr;

impl AttrMacro for ServerFnsAttr {
    type TokenStream = TokenStream2;
    type Error = syn::Error;

    fn transform2(
        args: Self::TokenStream,
        body: Self::TokenStream,
    ) -> Result<Self::TokenStream, Self::Error> {
        let RouteMeta { path, method } = syn::parse2(args)?;
        let ItemFn {
            mut attrs,
            vis,
            sig,
            block,
        } = syn::parse2(body)?;

        let server_fn_ident = &sig.ident;

        let path = path.unwrap_or_else(|| {
            format!("/api/{}", server_fn_ident)
                .replace('_', "-")
                .to_lowercase()
        });
        let method = method.unwrap_or_else(|| String::from("post"));

        let middlewares = attrs.collect_middleware();

        let method_fn = format_ident!("{method}");
        let router_fn = format_ident!("{}_router", server_fn_ident);
        let path_literal: LitStr = parse_quote! { #path };
        let method_expr: Expr = parse_quote! {
            ::server_fns::axum::routing::#method_fn (#server_fn_ident)
        };

        let server_fn_body = ItemFn {
            attrs,
            vis,
            sig,
            block,
        };

        Ok(quote! {
            #[::server_fns::linkme::distributed_slice(::server_fns::axum_router::COLLATED_ROUTES)]
            static ROUTE: fn() -> ::server_fns::axum::Router = #router_fn;

            fn #router_fn () -> ::server_fns::axum::Router {
                ::server_fns::axum::Router::new()
                    .route(#path_literal, #method_expr)
            }

            #server_fn_body
        })
    }
}

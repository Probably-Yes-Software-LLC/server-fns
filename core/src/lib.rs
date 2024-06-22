mod macro_traits;
mod parse;

#[cfg(feature = "axum")]
pub mod axum_router;

pub use macro_traits::AttrMacro;
use parse::{CollectMiddleware, RouteMeta};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{parse_quote, Expr, ItemFn, LitStr, Signature};

pub struct ServerFnsAttr;

impl AttrMacro for ServerFnsAttr {
    type TokenStream = TokenStream2;
    type Error = syn::Error;

    fn transform2(
        args: Self::TokenStream,
        body: Self::TokenStream,
    ) -> Result<Self::TokenStream, Self::Error> {
        let ItemFn {
            mut attrs,
            vis,
            sig:
                Signature {
                    constness,
                    asyncness,
                    unsafety,
                    abi,
                    fn_token,
                    ident,
                    generics,
                    paren_token,
                    inputs,
                    variadic,
                    output,
                },
            block,
        } = syn::parse2(body)?;
        let RouteMeta {
            http_path,
            http_method,
        } = RouteMeta::new(args, &ident)?;

        let middlewares = attrs.collect_middleware();

        let method_fn = format_ident!("{http_method}");
        let router_fn = format_ident!("{}_router", ident);
        let module = format_ident!("__{router_fn}");

        let path_literal: LitStr = parse_quote! { #http_path };
        let method_expr: Expr = parse_quote! {
            ::server_fns::axum::routing::#method_fn (super::#ident)
        };

        let server_fn_body = ItemFn {
            attrs,
            vis,
            sig: Signature {
                constness,
                asyncness,
                unsafety,
                abi,
                fn_token,
                ident,
                generics,
                paren_token,
                inputs,
                variadic,
                output,
            },
            block,
        };

        Ok(quote! {
            mod #module {

                #[::server_fns::linkme::distributed_slice(::server_fns::axum_router::COLLATED_ROUTES)]
                static ROUTER: fn() -> ::server_fns::axum::Router = #router_fn;

                fn #router_fn <State> (state: State) -> ::server_fns::axum::Router
                where State: ::std::clone::Clone + ::std::marker::Send + ::std::marker::Sync + 'static
                {
                    ::server_fns::axum::Router::new()
                        .route(#path_literal, #method_expr)
                        .with_state(state)
                }
            }

            #server_fn_body
        })
    }
}

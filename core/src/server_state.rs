use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{ToTokens, TokenStreamExt};
use syn::Ident;

pub trait ServerState: Sized {
    type Router;

    fn load_routes() -> axum::Router<Self>;
}

pub struct ServerStateImpl {
    pub span: Span,
    pub ident: Ident,
    pub current_package: String
}

mod server_state_impl {

    use std::env;

    use convert_case::{Case, Casing};
    use quote::{format_ident, quote_spanned};
    use syn::{spanned::Spanned, ItemStruct};

    use super::*;
    use crate::{current_package, make_router};

    impl ServerStateImpl {
        pub fn try_new(item: ItemStruct) -> Result<Self, syn::Error> {
            let current_package = current_package(item.span())?;

            Ok(Self {
                span: item.span(),
                ident: item.ident,
                current_package
            })
        }
    }

    impl ToTokens for ServerStateImpl {
        fn to_tokens(&self, tokens: &mut TokenStream2) {
            let Self {
                span,
                ident,
                current_package
            } = self;

            let ident_str = ident.to_string();
            let module = format_ident!(
                "__{}_{}",
                current_package.to_case(Case::Snake),
                ident_str.to_case(Case::Snake)
            );

            let state_router = make_router(&ident_str);
            let package_router = make_router(current_package);
            let router_fn_type = format_ident!("{state_router}Fn");

            tokens.append_all(quote_spanned! { *span =>
                pub type #package_router = #module::#state_router;

                mod #module {
                    #[automatically_derived]
                    impl ::server_fns::server_state::ServerState for super::#ident {
                        type Router = #state_router;

                        fn load_routes() -> ::server_fns::axum::Router<Self> {
                            let mut router = ::server_fns::axum::Router::new();
                            for next in ::server_fns::inventory::iter::<Self::Router> {
                                router = router.merge((next.router_fn)());
                            }
                            router
                        }
                    }

                    type #router_fn_type = fn() -> ::server_fns::axum::Router<super::#ident>;

                    pub struct #state_router {
                        pub path: &'static str,
                        pub router_fn: #router_fn_type
                    }

                    impl #state_router {
                        pub const fn register(path: &'static str, router_fn: #router_fn_type) -> Self {
                            Self { path, router_fn }
                        }
                    }

                    ::server_fns::inventory::collect!(#state_router);
                }
            });
        }
    }
}

use proc_macro::TokenStream;
use server_fns_core::{AttrMacro, DeriveMacro, MiddlewareAttr, ServerFnsAttr, ServerStateDerive};

#[proc_macro_attribute]
pub fn server(args: TokenStream, body: TokenStream) -> TokenStream {
    ServerFnsAttr::transform(args, body)
}

#[proc_macro_attribute]
pub fn middleware(args: TokenStream, body: TokenStream) -> TokenStream {
    MiddlewareAttr::transform(args, body)
}

#[proc_macro_derive(ServerState)]
pub fn server_state(item: TokenStream) -> TokenStream {
    ServerStateDerive::transform(item)
}

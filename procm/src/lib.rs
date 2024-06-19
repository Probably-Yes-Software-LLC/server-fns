use proc_macro::TokenStream;
use server_fns_core::{AttrMacro, ServerFnsAttr};

#[proc_macro_attribute]
pub fn server(args: TokenStream, body: TokenStream) -> TokenStream {
    ServerFnsAttr::transform(args, body)
}

use deluxe::ParseMetaItem;
use derive_syn_parse::Parse;
use syn::{
    parse::{Parse, ParseStream},
    Attribute, Error, Expr, Token
};

#[derive(ParseMetaItem)]
pub struct RouteMeta {
    pub path: Option<String>,
    pub method: Option<String>
}

#[derive(Parse)]
pub struct Middleware(pub Expr);

pub trait CollectMiddleware {
    fn collect_middleware(&mut self) -> Vec<Middleware>;
}

impl CollectMiddleware for Vec<Attribute> {
    fn collect_middleware(&mut self) -> Vec<Middleware> {
        let mut middlewares = Vec::<Middleware>::new();
        self.retain(|attr| {
            if attr.path().is_ident("middleware") {
                middlewares.push(attr.parse_args().expect("Expected middleware expression."));
                false
            } else {
                true
            }
        });

        middlewares
    }
}

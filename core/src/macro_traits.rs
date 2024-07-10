use proc_macro2::TokenStream as TokenStream2;

pub trait CompileError {
    type TokenStream;

    fn into_comp_err_tokens(self) -> Self::TokenStream;
}

pub trait FnMacro {
    type TokenStream;
    type Error: CompileError<TokenStream = Self::TokenStream>;
    type Result: Into<Result<Self::TokenStream, Self::Error>>;

    fn transform2(&self, item: Self::TokenStream) -> Self::Result;

    fn transform<IntoTS, FromTS>(&self, item: IntoTS) -> FromTS
    where
        IntoTS: Into<Self::TokenStream>,
        FromTS: From<Self::TokenStream>
    {
        match self.transform2(item.into()).into() {
            Ok(ts) => ts.into(),
            Err(err) => err.into_comp_err_tokens().into()
        }
    }
}

pub trait AttrMacro {
    type TokenStream;
    type Error: CompileError<TokenStream = Self::TokenStream>;
    type Result: Into<Result<Self::TokenStream, Self::Error>>;

    fn transform2(&self, args: Self::TokenStream, body: Self::TokenStream) -> Self::Result;

    fn transform<IntoTS, FromTS>(&self, args: IntoTS, body: IntoTS) -> FromTS
    where
        IntoTS: Into<Self::TokenStream>,
        FromTS: From<Self::TokenStream>
    {
        match self.transform2(args.into(), body.into()).into() {
            Ok(ts) => ts.into(),
            Err(err) => err.into_comp_err_tokens().into()
        }
    }
}

pub trait DeriveMacro {
    type TokenStream;
    type Error: CompileError<TokenStream = Self::TokenStream>;
    type Result: Into<Result<Self::TokenStream, Self::Error>>;

    fn transform2(&self, item: Self::TokenStream) -> Self::Result;

    fn transform<IntoTS, FromTS>(&self, item: IntoTS) -> FromTS
    where
        IntoTS: Into<Self::TokenStream>,
        FromTS: From<Self::TokenStream>
    {
        match self.transform2(item.into()).into() {
            Ok(ts) => ts.into(),
            Err(err) => err.into_comp_err_tokens().into()
        }
    }
}

impl CompileError for TokenStream2 {
    type TokenStream = Self;

    fn into_comp_err_tokens(self) -> Self::TokenStream {
        self
    }
}

impl CompileError for syn::Error {
    type TokenStream = TokenStream2;

    fn into_comp_err_tokens(self) -> Self::TokenStream {
        self.into_compile_error()
    }
}

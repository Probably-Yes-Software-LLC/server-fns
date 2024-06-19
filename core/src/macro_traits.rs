use proc_macro2::TokenStream as TokenStream2;

#[allow(unused)]
pub trait CompileError {
    type TokenStream;

    fn into_comp_err_tokens(self) -> Self::TokenStream;
}

#[allow(unused)]
pub trait AttrMacro {
    type TokenStream;
    type Error: CompileError<TokenStream = Self::TokenStream>;

    fn transform2(
        args: Self::TokenStream,
        body: Self::TokenStream,
    ) -> Result<Self::TokenStream, Self::Error>;

    fn transform<IntoTS, FromTS>(args: IntoTS, body: IntoTS) -> FromTS
    where
        IntoTS: Into<Self::TokenStream>,
        FromTS: From<Self::TokenStream>,
    {
        match Self::transform2(args.into(), body.into()) {
            Ok(ts) => ts.into(),
            Err(err) => err.into_comp_err_tokens().into(),
        }
    }
}

impl CompileError for syn::Error {
    type TokenStream = TokenStream2;

    fn into_comp_err_tokens(self) -> Self::TokenStream {
        self.into_compile_error()
    }
}

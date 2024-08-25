use std::{
    env, io,
    path::{Component, Path, PathBuf, MAIN_SEPARATOR}
};

use bytes::Bytes;
use itertools::Itertools;
use mime_guess::Mime;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned, ToTokens, TokenStreamExt};
use syn::{
    parse_quote_spanned, spanned::Spanned, Expr, ExprLit, ExprStruct, Ident, ItemStruct, Lit,
    Member
};
use thiserror::Error;
use tokio::fs;

#[derive(Debug, Default, Clone)]
pub struct EmbeddedAsset {
    pub data: Bytes,
    pub content_type: Option<String>
}

#[derive(Debug, Error)]
pub enum AssetError {
    #[error("Failed to read file at ({0})")]
    FileIO(String, #[source] io::Error),
    #[error("No data found for asset ({0})")]
    NotFound(String)
}

impl EmbeddedAsset {
    pub async fn __inject_embedded_asset(_path: impl AsRef<Path>) -> Self {
        Self::default()
    }

    pub async fn __from_file(path: &Path) -> Result<Self, AssetError> {
        let name = path.display().to_string();
        let bytes = fs::read(path)
            .await
            .map_err(|err| AssetError::FileIO(name.clone(), err))?;
        let mime = mime_guess::from_path(path);

        Ok(Self::new(bytes, mime.first()))
    }

    pub fn __from_static(data: &'static [u8], mime: Option<&'static str>) -> Self {
        Self::new(data, mime)
    }

    fn new(data: impl Into<Bytes>, mime: Option<impl ToString>) -> Self {
        Self {
            data: data.into(),
            content_type: mime.map(|mime| mime.to_string())
        }
    }
}

/// Dummy macro that is replaced at compile time by a server_fn attribute macro.
///
/// Doesn't actually need to be imported.
#[macro_export]
macro_rules! load_asset {
    (@IDENTITY $($tt:tt)+) => {
        $($tt)+
    };
    ($($tt:tt)+) => {
        $crate::embed_asset::EmbeddedAsset::__inject_embedded_asset($($tt)+)
    };
}

pub(crate) struct LoadAssetImpl {
    span: Span,
    asset_type: AssetType,
    base: String,
    path: Expr
}

enum AssetType {
    FileAsset,
    StaticAsset(Vec<StaticAsset>)
}

struct StaticAsset {
    ident: Ident,
    full: String,
    path: String,
    mime: Option<String>
}

impl AssetType {
    fn file() -> Self {
        Self::FileAsset
    }

    fn static_(base: impl AsRef<Path>) -> Result<Self, syn::Error> {
        let base = base.as_ref();

        let files = recurse_all_files(base).map_err(|err| {
            syn::Error::new(
                Span::call_site(),
                format!("Failed to load files at: {base:?}; {err}")
            )
        })?;

        let files = files
            .into_iter()
            .map(|full_path| {
                let asset_path = full_path
                    .strip_prefix(base)
                    .map(ToOwned::to_owned)
                    .map_err(|err| {
                        syn::Error::new(
                            Span::call_site(),
                            format!(
                                "Failed to trim base ({base:?}) from path ({full_path:?}); {err}"
                            )
                        )
                    })?;

                let file_ident = asset_path
                    .components()
                    .filter_map(|comp| {
                        if let Component::Normal(dir_or_file) = comp {
                            Some(
                                dir_or_file
                                    .to_ascii_uppercase()
                                    .to_string_lossy()
                                    .replace(|char_| ['.', '-'].contains(&char_), "_")
                            )
                        } else {
                            None
                        }
                    })
                    .join("_");

                let ident = format_ident!("__{file_ident}");

                let mime = mime_guess::from_path(&asset_path)
                    .first()
                    .map(|m| m.to_string());

                Ok::<_, syn::Error>(StaticAsset {
                    ident,
                    full: full_path.display().to_string(),
                    path: asset_path.display().to_string(),
                    mime
                })
            })
            .try_collect()?;

        Ok(Self::StaticAsset(files))
    }
}

fn recurse_all_files(path: &Path) -> Result<Vec<PathBuf>, io::Error> {
    if path.is_file() {
        return Ok(vec![path.to_owned()]);
    }

    let files = path
        .read_dir()?
        .map_ok(|entry| recurse_all_files(&entry.path()))
        .flatten()
        .flatten_ok()
        .collect::<Result<Vec<_>, _>>()?;

    Ok(files)
}

impl LoadAssetImpl {
    pub fn try_new(item: ExprStruct) -> Result<Self, syn::Error> {
        let span = item.span();

        let mut base = None;
        let mut path = None;

        for field in &item.fields {
            let Member::Named(name) = &field.member else {
                continue;
            };

            match name.to_string().as_ref() {
                "base" => base = Some(field.expr.clone()),
                "path" => path = Some(field.expr.clone()),
                _ => {}
            }
        }

        let base = match base.ok_or_else(|| syn::Error::new(span, "Expected base member."))? {
            Expr::Lit(ExprLit {
                lit: Lit::Str(litstr),
                ..
            }) => litstr.value(),
            _ => {
                return Err(syn::Error::new(span, "Expected base path literal."));
            }
        };
        let path = path.ok_or_else(|| syn::Error::new(span, "Expected path member."))?;

        let asset_type = match item.path.get_ident() {
            Some(ident) if *ident == stringify!(FileAsset) => AssetType::file(),
            Some(ident) if *ident == stringify!(StaticAsset) => AssetType::static_(&base)?,
            _ => {
                return Err(syn::Error::new(span, "Expected asset type (struct name)."));
            }
        };

        Ok(Self {
            span,
            asset_type,
            base,
            path
        })
    }
}

impl ToTokens for LoadAssetImpl {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            span,
            asset_type,
            base,
            path
        } = self;

        match asset_type {
            AssetType::FileAsset => file_asset_to_tokens(tokens, span, base, path),
            AssetType::StaticAsset(static_assets) => {
                static_asset_to_tokens(tokens, span, path, static_assets)
            }
        }
    }
}

fn file_asset_to_tokens(tokens: &mut TokenStream, span: &Span, base: &String, path: &Expr) {
    tokens.append_all(quote_spanned! { *span =>
        ::server_fns::load_asset! {
            @IDENTITY {
                use ::std::path::PathBuf;
                use ::server_fns::embed_asset::EmbeddedAsset;

                let base = #base;
                let path = &#path;
                let file_path = PathBuf::from(base).join(path);
                EmbeddedAsset::__from_file(&file_path).await
            }
        }
    });
}

fn static_asset_to_tokens(
    tokens: &mut TokenStream,
    span: &Span,
    path: &Expr,
    static_assets: &Vec<StaticAsset>
) {
    let matchers = static_assets.iter().map(
        |StaticAsset {
             ident, path, mime, ..
         }| {
            match mime {
                Some(mime) => quote_spanned! { *span =>
                    #path => Ok(EmbeddedAsset::__from_static(#ident, Some(#mime)))
                },
                None => quote_spanned! { *span =>
                    #path => Ok(EmbeddedAsset::__from_static(#ident, None))
                }
            }
        }
    );

    tokens.append_all(quote_spanned! { *span =>
        ::server_fns::load_asset! {
            @IDENTITY {
                use ::server_fns::embed_asset::EmbeddedAsset;
                use ::server_fns::embed_asset::AssetError;

                #(#static_assets)*

                match &#path {
                    #(#matchers),*
                    unknown => Err(AssetError::NotFound(unknown.to_string()))
                }
            }
        }
    });
}

impl ToTokens for StaticAsset {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self { ident, full, .. } = self;

        tokens.append_all(quote! {
            static #ident: &'static [u8] = ::std::include_bytes!(#full);
        });
    }
}

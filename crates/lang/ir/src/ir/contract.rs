// Copyright 2018-2020 Parity Technologies (UK) Ltd.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::{ast, ir};
use core::convert::TryFrom;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use quote::ToTokens;
use std::str::FromStr;
use syn::spanned::Spanned;
use syn::visit_mut::{self, VisitMut};
use syn::ItemMod;

/// An ink! contract definition consisting of the ink! configuration and module.
///
/// This is the root of any ink! smart contract definition. It contains every
/// information accessible to the ink! smart contract macros. It is also used
/// as the root source for the ink! code generation.
///
/// # Example
///
/// ```no_compile
/// #[ink::contract(/* optional ink! configurations */)]
/// mod my_contract {
///     /* ink! and Rust definitions */
/// }
/// ```
pub struct Contract {
    /// The parsed Rust inline module.
    ///
    /// Contains all Rust module items after parsing. Note that while parsing
    /// the ink! module all ink! specific items are moved out of this AST based
    /// representation.
    item: ir::ItemMod,

    // original_item keep original Rust ast.
    original_item: syn::ItemMod,
    /// The specified ink! configuration.
    config: ir::Config,
}

impl Contract {
    /// Creates a new ink! contract from the given ink! configuration and module
    /// token streams.
    ///
    /// The ink! macro should use this constructor in order to setup ink!.
    ///
    /// # Note
    ///
    /// - The `ink_config` token stream must properly decode into [`ir::Config`].
    /// - The `ink_module` token stream must properly decode into [`ir::ItemMod`].
    ///
    /// # Errors
    ///
    /// Returns an error if the provided token stream cannot be decoded properly
    /// into a valid ink! configuration or ink! module respectively.
    pub fn new(
        ink_config: TokenStream2,
        ink_module: TokenStream2,
    ) -> Result<Self, syn::Error> {
        let config = syn::parse2::<ast::AttributeArgs>(ink_config)?;
        let module = syn::parse2::<syn::ItemMod>(ink_module.clone())?;
        let ink_config = ir::Config::try_from(config)?;
        let original_module = Self::remove_ink_attrs(&ink_config, ink_module.clone());
        // let original_module= ink_module.clone();
        let original_item = syn::parse2::<syn::ItemMod>(original_module.clone())?;
        let ink_module = ir::ItemMod::try_from(module.clone())?;
        Ok(Self {
            item: ink_module,
            original_item,
            config: ink_config,
        })
    }

    fn remove_ink_attrs(config: &ir::Config, ink_module: TokenStream2) -> TokenStream2 {
        #[derive(Default)]
        struct InkAttrEraser {
            mod_count: usize,
            original_name: String,
        };

        impl VisitMut for InkAttrEraser {
            // rewrite module name when meet the first module
            fn visit_item_mod_mut(&mut self, module: &mut ItemMod) {
                if self.mod_count == 0 {
                    module.ident =
                        syn::Ident::new(self.original_name.as_str(), module.ident.span());
                }
                self.mod_count += 1;
                visit_mut::visit_item_mod_mut(self, module);
            }

            // remove all ink related attrs
            fn visit_attribute_mut(&mut self, attr: &mut syn::Attribute) {
                if attr.path.is_ident("ink") {
                    let old_attr = attr.clone();
                    let path = attr.path.clone();
                    attr.path = syn::Path {
                        leading_colon: None,
                        segments: syn::punctuated::Punctuated::new(),
                    };
                    attr.path
                        .segments
                        .push(syn::PathSegment::from(syn::Ident::new(
                            "doc",
                            path.span(),
                        )));
                    attr.tokens =
                        TokenStream2::from_str("(inline)").expect("logic error");
                } else {
                    visit_mut::visit_attribute_mut(self, attr);
                }
            }
        }
        let mut tree = syn::parse2(ink_module).unwrap();
        let mut eraser = InkAttrEraser::default();
        eraser.original_name =
            config
                .original_mod_name()
                .map_or("original".to_string(), |val| {
                    val.get_ident()
                        .expect("need a new legal module name for original code")
                        .to_string()
                });
        eraser.visit_file_mut(&mut tree);
        tree.into_token_stream()
    }

    /// Returns the ink! inline module definition.
    ///
    /// # Note
    ///
    /// The ink! inline module definition is the module that comprises the
    /// whole ink! smart contract, e.g.:
    ///
    /// ```no_compile
    /// #[ink::contract]
    /// mod my_contract {
    ///     // ... definitions
    /// }
    /// ```
    pub fn module(&self) -> &ir::ItemMod {
        &self.item
    }

    pub fn original_module(&self) -> &ItemMod {
        return &self.original_item;
    }

    /// Returns the configuration of the ink! smart contract.
    ///
    /// # Note
    ///
    /// The configuration is given via the `#[ink::contract(config))]` attribute
    /// macro annotation itself within the `(config)` part. The available fields
    /// are the following:
    ///
    /// - `types`: To specify `Environment` different from the default environment
    ///            types.
    /// - `storage-alloc`: If `true` enables the dynamic storage allocator
    ///                    facilities and code generation of the ink! smart
    ///                    contract. Does incure some overhead. The default is
    ///                    `true`.
    /// - `as-dependency`: If `true` compiles this ink! smart contract always as
    ///                    if it was a dependency of another smart contract.
    ///                    This configuration is mainly needed for testing and
    ///                    the default is `false`.
    ///
    /// Note that we might add more configuration fields in the future if
    /// necessary.
    pub fn config(&self) -> &ir::Config {
        &self.config
    }
}

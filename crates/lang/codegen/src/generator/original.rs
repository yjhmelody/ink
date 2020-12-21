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

use crate::GenerateCode;
use derive_more::From;
use quote::{ToTokens};
use proc_macro2::TokenStream as TokenStream2;
use std::str::FromStr;
use syn::spanned::Spanned;
use syn::visit_mut::{self, VisitMut};
use syn::ItemMod;
/// Generates code for the ink! environment of the contract.
#[derive(From)]
pub struct Original<'a> {
    contract: &'a ir::Contract,
}

impl GenerateCode for Original<'_> {
    fn generate_code(&self) -> TokenStream2 {
        #[derive(Default)]
        struct InkAttrEraser {
            mod_count: usize,
            original_name: String,
        };

        impl VisitMut for InkAttrEraser {
            // TODO: erase `ink::trait_definition` or change its usage to `ink(trait_definition)`

            // remove all ink related attrs
            fn visit_attribute_mut(&mut self, attr: &mut syn::Attribute) {
                if attr.path.is_ident("trait_definition") {
                    dbg!(&attr);
                }
                if attr.path.is_ident("ink") {
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
                        TokenStream2::from_str("(inline)").expect("ink attr internal logic error");
                } else {
                    visit_mut::visit_attribute_mut(self, attr);
                }
            }

            // rewrite module name when meet the first module
            fn visit_item_mod_mut(&mut self, module: &mut ItemMod) {
                if self.mod_count == 0 {
                    module.ident =
                        syn::Ident::new(self.original_name.as_str(), module.ident.span());
                }
                self.mod_count += 1;
                visit_mut::visit_item_mod_mut(self, module);
            }
        }
        let mut tree = syn::parse2(self.contract .ink_module()).unwrap();
        let mut eraser = InkAttrEraser::default();
        eraser.original_name =
            self.contract.config()
                .original_mod_name()
                .map_or("original".to_string(), |val| {
                    val.get_ident()
                        .expect("need a new legal module name for original code")
                        .to_string()
                });
        eraser.visit_file_mut(&mut tree);
        tree.into_token_stream()
    }
}

use syn::spanned::Spanned;
use syn::{Abi, Signature, Token, Visibility};

use syn::Field;
use syn::{ForeignItem, ForeignItemFn, ForeignItemMacro, ForeignItemStatic, ForeignItemType};
use syn::{ImplItem, ImplItemConst, ImplItemFn, ImplItemMacro, ImplItemType};
use syn::{
    Item, ItemConst, ItemEnum, ItemExternCrate, ItemFn, ItemForeignMod, ItemImpl, ItemMacro,
    ItemMod, ItemStatic, ItemStruct, ItemTrait, ItemTraitAlias, ItemType, ItemUnion, ItemUse,
};
use syn::{TraitItem, TraitItemConst, TraitItemFn, TraitItemMacro, TraitItemType};

use crate::parse::{
    FlexibleItemConst, FlexibleItemFn, FlexibleItemStatic, FlexibleItemType, Qualifiers,
};

/// Qualification [lens](https://rust-unofficial.github.io/patterns/functional/lenses.html).
pub(crate) struct QualifiersLens<'a> {
    visibility: Option<&'a mut Visibility>,
    defaultness: Option<&'a mut Option<Token![default]>>,
    constness: Option<&'a mut Option<Token![const]>>,
    asyncness: Option<&'a mut Option<Token![async]>>,
    unsafety: Option<&'a mut Option<Token![unsafe]>>,
    abi: Option<&'a mut Option<Abi>>,
}

impl<'a> QualifiersLens<'a> {
    #[must_use]
    fn new() -> Self {
        Self {
            visibility: None,
            defaultness: None,
            constness: None,
            asyncness: None,
            unsafety: None,
            abi: None,
        }
    }

    #[must_use]
    fn visibility(self, visibility: &'a mut Visibility) -> Self {
        Self {
            visibility: Some(visibility),
            ..self
        }
    }

    #[must_use]
    fn defaultness(self, defaultness: &'a mut Option<Token![default]>) -> Self {
        Self {
            defaultness: Some(defaultness),
            ..self
        }
    }

    #[must_use]
    fn constness(self, constness: &'a mut Option<Token![const]>) -> Self {
        Self {
            constness: Some(constness),
            ..self
        }
    }

    #[must_use]
    fn asyncness(self, asyncness: &'a mut Option<Token![async]>) -> Self {
        Self {
            asyncness: Some(asyncness),
            ..self
        }
    }

    #[must_use]
    fn unsafety(self, unsafety: &'a mut Option<Token![unsafe]>) -> Self {
        Self {
            unsafety: Some(unsafety),
            ..self
        }
    }

    #[must_use]
    fn abi(self, abi: &'a mut Option<Abi>) -> Self {
        Self {
            abi: Some(abi),
            ..self
        }
    }

    #[must_use]
    fn signature(self, signature: &'a mut Signature) -> Self {
        self.constness(&mut signature.constness)
            .asyncness(&mut signature.asyncness)
            .unsafety(&mut signature.unsafety)
            .abi(&mut signature.abi)
    }

    /// Applies the given qualifiers to the item.
    pub fn apply(self, qualifiers: Qualifiers) -> syn::Result<()> {
        let Self {
            visibility: target_visibility,
            defaultness: target_defaultness,
            constness: target_constness,
            asyncness: target_asyncness,
            unsafety: target_unsafety,
            abi: target_abi,
        } = self;
        let Qualifiers {
            visibility,
            defaultness,
            constness,
            asyncness,
            unsafety,
            abi,
        } = qualifiers;

        // TODO: emit only a single error with every unsupported qualifier in one message?
        let mut errors = Vec::new();

        if let Some(visibility) = visibility {
            if let Some(target_visibility) = target_visibility {
                *target_visibility = visibility;
            } else {
                errors.push(syn::Error::new(
                    visibility.span(),
                    "visibility qualifier not supported here",
                ));
            }
        }

        if defaultness.is_some() {
            if let Some(target_defaultness) = target_defaultness {
                *target_defaultness = defaultness;
            } else {
                errors.push(syn::Error::new(
                    defaultness.span(),
                    "defaultness qualifier not supported here",
                ));
            }
        }

        if constness.is_some() {
            if let Some(target_constness) = target_constness {
                *target_constness = constness;
            } else {
                errors.push(syn::Error::new(
                    constness.span(),
                    "constness qualifier not supported here",
                ));
            }
        }

        if asyncness.is_some() {
            if let Some(target_asyncness) = target_asyncness {
                *target_asyncness = asyncness;
            } else {
                errors.push(syn::Error::new(
                    asyncness.span(),
                    "asyncness qualifier not supported here",
                ));
            }
        }

        if unsafety.is_some() {
            if let Some(target_unsafety) = target_unsafety {
                *target_unsafety = unsafety;
            } else {
                errors.push(syn::Error::new(
                    unsafety.span(),
                    "unsafety qualifier not supported here",
                ));
            }
        }

        if abi.is_some() {
            if let Some(target_abi) = target_abi {
                *target_abi = abi;
            } else {
                errors.push(syn::Error::new(
                    defaultness.span(),
                    "ABI qualifier not supported here",
                ));
            }
        }

        if let Some(error) = errors.into_iter().reduce(|mut error, next| {
            error.combine(next);
            error
        }) {
            Err(error)
        } else {
            Ok(())
        }
    }
}

pub(crate) trait Qualify {
    /// Extracts an item's qualifiers.
    fn qualify(&mut self) -> QualifiersLens;
}

//
// Items
//

impl Qualify for Item {
    fn qualify(&mut self) -> QualifiersLens {
        match self {
            Self::Const(item_const) => item_const.qualify(),
            Self::Enum(item_enum) => item_enum.qualify(),
            Self::ExternCrate(item_extern_crate) => item_extern_crate.qualify(),
            Self::Fn(item_fn) => item_fn.qualify(),
            Self::ForeignMod(item_foreign_mod) => item_foreign_mod.qualify(),
            Self::Impl(item_impl) => item_impl.qualify(),
            Self::Macro(item_macro) => item_macro.qualify(),
            Self::Mod(item_mod) => item_mod.qualify(),
            Self::Static(item_static) => item_static.qualify(),
            Self::Struct(item_struct) => item_struct.qualify(),
            Self::Trait(item_trait) => item_trait.qualify(),
            Self::TraitAlias(item_trait_alias) => item_trait_alias.qualify(),
            Self::Type(item_type) => item_type.qualify(),
            Self::Union(item_union) => item_union.qualify(),
            Self::Use(item_use) => item_use.qualify(),
            _ => QualifiersLens::new(),
        }
    }
}

impl Qualify for ItemConst {
    fn qualify(&mut self) -> QualifiersLens {
        QualifiersLens::new().visibility(&mut self.vis)
    }
}

impl Qualify for ItemEnum {
    fn qualify(&mut self) -> QualifiersLens {
        QualifiersLens::new().visibility(&mut self.vis)
    }
}

impl Qualify for ItemExternCrate {
    fn qualify(&mut self) -> QualifiersLens {
        QualifiersLens::new().visibility(&mut self.vis)
    }
}

impl Qualify for ItemFn {
    fn qualify(&mut self) -> QualifiersLens {
        QualifiersLens::new()
            .visibility(&mut self.vis)
            .signature(&mut self.sig)
    }
}

impl Qualify for ItemForeignMod {
    fn qualify(&mut self) -> QualifiersLens {
        QualifiersLens::new().unsafety(&mut self.unsafety)
    }
}

impl Qualify for ItemImpl {
    fn qualify(&mut self) -> QualifiersLens {
        QualifiersLens::new()
            .defaultness(&mut self.defaultness)
            .unsafety(&mut self.unsafety)
    }
}

impl Qualify for ItemMacro {
    fn qualify(&mut self) -> QualifiersLens {
        QualifiersLens::new()
    }
}

impl Qualify for ItemMod {
    fn qualify(&mut self) -> QualifiersLens {
        QualifiersLens::new()
            .visibility(&mut self.vis)
            .unsafety(&mut self.unsafety)
    }
}

impl Qualify for ItemStatic {
    fn qualify(&mut self) -> QualifiersLens {
        // TODO: mutability?
        QualifiersLens::new().visibility(&mut self.vis)
    }
}

impl Qualify for ItemStruct {
    fn qualify(&mut self) -> QualifiersLens {
        QualifiersLens::new().visibility(&mut self.vis)
    }
}

impl Qualify for ItemTrait {
    fn qualify(&mut self) -> QualifiersLens {
        // TODO: auto?
        QualifiersLens::new()
            .visibility(&mut self.vis)
            .unsafety(&mut self.unsafety)
    }
}

impl Qualify for ItemTraitAlias {
    fn qualify(&mut self) -> QualifiersLens {
        QualifiersLens::new().visibility(&mut self.vis)
    }
}

impl Qualify for ItemType {
    fn qualify(&mut self) -> QualifiersLens {
        QualifiersLens::new().visibility(&mut self.vis)
    }
}

impl Qualify for ItemUnion {
    fn qualify(&mut self) -> QualifiersLens {
        QualifiersLens::new().visibility(&mut self.vis)
    }
}

impl Qualify for ItemUse {
    fn qualify(&mut self) -> QualifiersLens {
        QualifiersLens::new().visibility(&mut self.vis)
    }
}

//
// Foreign Items
//

impl Qualify for ForeignItem {
    fn qualify(&mut self) -> QualifiersLens {
        match self {
            Self::Fn(item_fn) => item_fn.qualify(),
            Self::Macro(item_macro) => item_macro.qualify(),
            Self::Static(item_static) => item_static.qualify(),
            Self::Type(item_type) => item_type.qualify(),
            _ => QualifiersLens::new(),
        }
    }
}

impl Qualify for ForeignItemFn {
    fn qualify(&mut self) -> QualifiersLens {
        QualifiersLens::new()
            .visibility(&mut self.vis)
            .signature(&mut self.sig)
    }
}

impl Qualify for ForeignItemMacro {
    fn qualify(&mut self) -> QualifiersLens {
        QualifiersLens::new()
    }
}

impl Qualify for ForeignItemStatic {
    fn qualify(&mut self) -> QualifiersLens {
        // TODO: mutability?
        QualifiersLens::new().visibility(&mut self.vis)
    }
}

impl Qualify for ForeignItemType {
    fn qualify(&mut self) -> QualifiersLens {
        QualifiersLens::new().visibility(&mut self.vis)
    }
}

//
// Impl Items
//

impl Qualify for ImplItem {
    fn qualify(&mut self) -> QualifiersLens {
        match self {
            Self::Const(item_const) => item_const.qualify(),
            Self::Fn(item_fn) => item_fn.qualify(),
            Self::Macro(item_macro) => item_macro.qualify(),
            Self::Type(item_type) => item_type.qualify(),
            _ => QualifiersLens::new(),
        }
    }
}

impl Qualify for ImplItemConst {
    fn qualify(&mut self) -> QualifiersLens {
        QualifiersLens::new()
            .visibility(&mut self.vis)
            .defaultness(&mut self.defaultness)
    }
}

impl Qualify for ImplItemFn {
    fn qualify(&mut self) -> QualifiersLens {
        QualifiersLens::new()
            .visibility(&mut self.vis)
            .defaultness(&mut self.defaultness)
            .signature(&mut self.sig)
    }
}

impl Qualify for ImplItemMacro {
    fn qualify(&mut self) -> QualifiersLens {
        QualifiersLens::new()
    }
}

impl Qualify for ImplItemType {
    fn qualify(&mut self) -> QualifiersLens {
        QualifiersLens::new()
            .visibility(&mut self.vis)
            .defaultness(&mut self.defaultness)
    }
}

//
// Trait Items
//

impl Qualify for TraitItem {
    fn qualify(&mut self) -> QualifiersLens {
        match self {
            Self::Const(item_const) => item_const.qualify(),
            Self::Fn(item_fn) => item_fn.qualify(),
            Self::Macro(item_macro) => item_macro.qualify(),
            Self::Type(item_type) => item_type.qualify(),
            _ => QualifiersLens::new(),
        }
    }
}

impl Qualify for TraitItemConst {
    fn qualify(&mut self) -> QualifiersLens {
        QualifiersLens::new()
    }
}

impl Qualify for TraitItemFn {
    fn qualify(&mut self) -> QualifiersLens {
        QualifiersLens::new().signature(&mut self.sig)
    }
}

impl Qualify for TraitItemMacro {
    fn qualify(&mut self) -> QualifiersLens {
        QualifiersLens::new()
    }
}

impl Qualify for TraitItemType {
    fn qualify(&mut self) -> QualifiersLens {
        QualifiersLens::new()
    }
}

//
// Fields
//

impl Qualify for Field {
    fn qualify(&mut self) -> QualifiersLens {
        // TODO: mutability?
        QualifiersLens::new().visibility(&mut self.vis)
    }
}

//
// Flexible Items
//

impl Qualify for FlexibleItemConst {
    fn qualify(&mut self) -> QualifiersLens {
        QualifiersLens::new()
            .visibility(&mut self.vis)
            .defaultness(&mut self.defaultness)
    }
}

impl Qualify for FlexibleItemFn {
    fn qualify(&mut self) -> QualifiersLens {
        QualifiersLens::new()
            .visibility(&mut self.vis)
            .defaultness(&mut self.defaultness)
            .signature(&mut self.sig)
    }
}

impl Qualify for FlexibleItemStatic {
    fn qualify(&mut self) -> QualifiersLens {
        // TODO: mutability?
        QualifiersLens::new().visibility(&mut self.vis)
    }
}

impl Qualify for FlexibleItemType {
    fn qualify(&mut self) -> QualifiersLens {
        QualifiersLens::new()
            .visibility(&mut self.vis)
            .defaultness(&mut self.defaultness)
    }
}

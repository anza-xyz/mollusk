#![doc = include_str!("../README.md")]
// Templated by `cargo-generate` using https://github.com/danielhenrymantilla/proc-macro-template
#![allow(nonstandard_style, unused_imports)]
#![forbid(unsafe_code)]

use ::core::{
    mem,
    ops::Not as _,
};
use ::proc_macro::{
    TokenStream,
};
use ::proc_macro2::{
    Span,
    TokenStream as TokenStream2,
    TokenTree as TT,
};
use ::quote::{
    format_ident,
    quote,
    quote_spanned,
    ToTokens,
};
use ::syn::{*,
    parse::{Parse, Parser, ParseStream},
    punctuated::Punctuated,
    Result, // Explicitly shadow it
    spanned::Spanned,
};

/// [`#[::core::prelude::v1::cfg_eval]`](
/// https://doc.rust-lang.org/1.70.0/core/prelude/v1/attr.cfg_eval.html)
/// in stable Rust.
///
///   - Note: this macro, by default, only works on `struct`, `enum`, and
///     `union` definitions (_i.e._, on `#[derive]` input).
///
///     Enable `features = ["items"]` to get support for arbitary items.
///
/// ## Example
///
/// ```rust
/// use ::macro_rules_attribute::apply;
///
/// #[macro_use]
/// extern crate cfg_eval;
///
/// fn main()
/// {
///     let output_without_cfg_eval = {
///         #[apply(stringify!)]
///         enum Foo {
///             Bar,
///
///             #[cfg(FALSE)]
///             NonExisting,
///         }
///     };
///     // This is usually not great.
///     assert!(output_without_cfg_eval.contains("NonExisting"));
///
///     let output_with_cfg_eval = {
///         #[cfg_eval]
///         #[apply(stringify!)]
///         enum Foo {
///             Bar,
///
///             #[cfg(FALSE)]
///             NonExisting,
///         }
///     };
///     assert_eq!(output_with_cfg_eval, stringify! {
///         enum Foo {
///             Bar,
///         }
///     });
/// }
/// ```
///
/// ## How it works
///
/// The way this is achieved is by taking advantage of **`#[derive(SomeDerive)]`
/// having `#[cfg_eval]` semantics built in**.
///
///   - This means that if you have the luxury of hesitating between offering a
///     derive macro, or an attribute macro, and need `#[cfg_eval]` semantics,
///     you'd be better off using a derive than `#[cfg_eval]`!
///
///   - but the reality is that certain macros do want to modify their input,
///     which rules out implementing it as a `#[derive]`.
///
/// #### Illustration
///
/// With that knowledge, let's see a step-by-step description of what the macro
/// is doing.
///
/// Consider, for instance, the following snippet:
///
/// ```rust
/// # #[cfg(any())] macro_rules! ignore {
/// #[cfg_eval]
/// #[other attrs...]
/// #[cfg_attr(all(), for_instance)]
/// struct Foo {
///     x: i32,
///     #[cfg(any())]
///     y: u8,
/// }
/// # }
/// ```
///
///   - Remember: `all()` is `cfg`-speak for `true`, and `any()`, for `false`.
///
/// **What `#[cfg_eval]` does**, then, on this snippet, is emitting the
/// following:
///
/// ```rust
/// # #[cfg(any())] macro_rules! ignore {
/// #[derive(RemoveExterminate)] // 👈
/// #[exterminate]               // 👈
/// #[other attrs...]
/// #[cfg_attr(all(), for_instance)]
/// struct Foo {
///     x: i32,
///     #[cfg(any())]
///     y: u8,
/// }
/// # }
/// ```
///
/// With the added macros doing what their names indicate. If this is not clear
/// enough, then feel free to read the following more detailed section:
///
/// <details class="custom"><summary><span class="summary-box"><span>Click to show</span></span></summary>
///
/// The `#[derive(RemoveExterminate)]` invocation leads to the following two
/// snippets of code, as per the rules of `#[derive()]`s (_i.e._, that the
/// annotated item be emitted independently of what the macro emits):
///
///   - _independently of what `RemoveExterminate` does_, we have:
///
///     ```rust
///     # #[cfg(any())] macro_rules! ignore {
///     #[exterminate] // 👈
///     #[other attrs...]
///     #[cfg_attr(all(), for_instance)]
///     struct Foo {
///         x: i32,
///         #[cfg(any())]
///         y: u8,
///     }
///     # }
///     ```
///
///     which then, thus, calls `#[exterminate]`.
///
///     What `#[exterminate]` does, as hinted to the attentive reader by its
///     otherwise totally unconspicuous name, is removing the annotated item:
///
///     ```rust
///     # #[cfg(any())] macro_rules! ignore {
///     /* nothing here! */
///     # }
///     ```
///
///   - _independently of the previous bullet_, `#[derive(RemoveExterminate)]`
///     gets called, and we get:
///
///     ```rust
///     # #[cfg(any())] macro_rules! ignore {
///     //! Pseudo-code!
///
///     RemoveExterminate! {
///         // Note: thanks to `#[derive]` Magic™, the following tokens have
///         // been `#[cfg_eval]`-cleaned up! // ----+
///         #[exterminate]                        // |
///         #[other attrs...]                     // |
///         #[for_instance] // <---------------------+
///         struct Foo {                          // |
///             x: i32,                           // |
///         /* removed! <----------------------------+
///             #[cfg(any())]
///             y: u8,
///          */
///         }
///     }
///     # }
///     ```
///
///     From here, this `RemoveExterminate` macro knows it has been invoked on an item
///     doomed to die. It can thus, **contrary to usual derives, reëmit the item
///     it receives, modifying it at leisure**.
///
///     What it actually does, then, is reëmitting it almost as-is, but for that
///     pesky `#[exterminate]` which is no longer useful to us.
///
///     ```rust
///     # #[cfg(any())] macro_rules! ignore {
///     #[other attrs...]
///     #[for_instance]
///     struct Foo {
///         x: i32,
///     }
///     # }
///     ```
///
/// Which matches the expected `#[cfg_eval]` semantics on that original input:
///
/// ```rust
/// # #[cfg(any())] macro_rules! ignore {
/// #[cfg_eval]
/// #[other attrs...]
/// #[cfg_attr(all(), for_instance)]
/// struct Foo {
///     x: i32,
///     #[cfg(any())]
///     y: u8,
/// }
/// # }
/// ```
///
/// ___
///
/// </details>
///
/// ## Reëxporting this macro: the `crate = ::some::path` optional attribute arg
///
/// The following section is only meaningful to macro authors wishing to
/// reëxport <code>#[[cfg_eval]]</code> / reüse it from further downstream code.
///
/// <details class="custom"><summary><span class="summary-box"><span>Click to show</span></span></summary>
///
/// Given the above implementation, it should be no surprise that
/// <code>#[[cfg_eval]]</code> needs to refer to sibling helper macros in a
/// path-robust manner.
///
/// By default, it uses `::cfg_eval::*` paths, expecting there to be an external
/// `::cfg_eval` crate with the items of this very crate (like proc-macros are
/// known to do, given lack of `$crate` for non-function-like proc-macros), as
/// **a direct dependency**.
///
/// This means that if you have your own macro which, internally (or publicly),
/// needs <code>#[[cfg_eval]]</code>, then **your downstream dependents, which
/// are likely not to depend on `::cfg_eval` _directly_**, will be unable to
/// make it work.
///
/// The solution, to this, is an optional `crate = ::some::path` attribute arg:
///
/// ```rust
/// # // for the test
/// # extern crate std as cfg_eval;
/// # #[cfg(any())]
/// # extern crate cfg_eval;
/// #
/// # extern crate self as your_crate;
/// #
/// #[doc(hidden)] /** Not part of the public API */
/// pub mod __internal {
///     # #[cfg(any())] // for the test
///     pub use ::cfg_eval;
///     # pub extern crate cfg_eval;
/// }
///
/// #[macro_export]
/// macro_rules! example {( $input:item ) => (
///     #[$crate::__internal::cfg_eval::cfg_eval(
///         // Add this:
///         crate = $crate::__internal::cfg_eval // 👈
///     )]
///     $input
/// )}
///
/// // so that downstream users can write:
/// ::your_crate::example! {
///     struct Foo { /* … */ }
/// }
/// # fn main() {}
/// ```
///
/// ___
///
/// </details>
///
/// [cfg_eval]: [macro@cfg_eval]
#[proc_macro_attribute] pub
fn cfg_eval(
    args: TokenStream,
    input: TokenStream,
) -> TokenStream
{
    cfg_eval_impl(args.into(), input.into())
    //  .map(|ret| { println!("{}", ret); ret })
        .unwrap_or_else(|err| {
            let mut errors =
                err .into_iter()
                    .map(|err| Error::new(
                        err.span(),
                        format_args!("`#[cfg_eval::cfg_eval]`: {}", err),
                    ))
            ;
            let mut err = errors.next().unwrap();
            errors.for_each(|cur| err.combine(cur));
            err.to_compile_error()
        })
        .into()
}

fn cfg_eval_impl(
    args: TokenStream2,
    input: TokenStream2,
) -> Result<TokenStream2>
{
    // Handle an optional `crate = some::path` arg
    let krate: Option<(Span, TokenStream2)> = Parser::parse2(
        |input: ParseStream<'_>| Result::<_>::Ok({
            let krate: Option<Token![crate]> = input.parse()?;
            if let Some(krate) = krate {
                let _: Token![=] = input.parse()?;
                let path: TokenStream2 = input.parse()?;
                let mut _span = krate.span();
                // not worth it, I think.
                // let path = path.into_iter().map(|tt| { _span = tt.span(); tt }).collect();
                Some((_span, path))
            } else {
                None
            }
        }),
        args,
    )?;

    let (span, krate) = krate.unwrap_or_else(|| (
        Span::mixed_site(),
        quote_spanned!(Span::mixed_site() =>
            ::cfg_eval
        ),
    ));

    let attrs_to_add = quote_spanned!(span=>
        #[::core::prelude::v1::derive(
            #krate::ඞRemoveExterminate
        )]
        #[#krate::ඞdalek_exterminate]
    );

    if cfg!(feature = "items") {
        return Ok(quote_spanned!(Span::mixed_site()=>
            #attrs_to_add
            enum ඞ { ඞ = { #input } }
        ));
    }

    Ok(quote_spanned!(Span::mixed_site()=>
        #attrs_to_add
        #input
    ))
}

#[doc(hidden)] /** Not part of the public API */
#[proc_macro_derive(ඞRemoveExterminate)] pub
fn __(input: TokenStream)
  -> TokenStream
{
    if cfg!(not(feature = "items")) {
        // The only thing left for us to do is removing the
        // `#` and `[ …exterminate ]`
        return input.into_iter().skip(2).collect();
    }

    // From:
    // ```rs
    // #[…exterminate]
    // enum ඞ {
    //     ඞ = { #input }
    // }
    // ```
    // to:
    // ```rs
    // #input
    // ```
    let mut tts = input.into_iter();

    // Remove `#[…] enum ඞ`
    tts.by_ref().take(4).for_each(drop);
    // Remove the `{}` in `{ ඞ = … }`
    if let Some(::proc_macro::TokenTree::Group(g)) = tts.next() {
        tts = g.stream().into_iter();
    }
    // Remove `ඞ =`
    tts.by_ref().take(2).for_each(drop);
    // Remove the `{}` in `{ #input }`
    if let Some(::proc_macro::TokenTree::Group(g)) = tts.next() {
        tts = g.stream().into_iter();
    }

    tts.collect()
}

#[doc(hidden)] /** Not part of the public API */
#[proc_macro_attribute] pub
fn ඞdalek_exterminate(_: TokenStream, _: TokenStream)
  -> TokenStream
{
    <_>::default()
}

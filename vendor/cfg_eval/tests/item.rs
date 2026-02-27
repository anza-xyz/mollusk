macro_rules! stringify_rhs {(
    const _: () = $value:tt ;
) => (
    stringify! $value
)}

macro_rules! cfg_eval_stringify {(
    $($body:tt)*
) => ({
    #[::cfg_eval::cfg_eval]
    #[::macro_rules_attribute::apply(stringify_rhs!)]
    const _: () = { $($body)* };
})}

#[test]
fn main()
{
    assert_eq!(
        cfg_eval_stringify! {
            enum _WithoutCfgEval {
                #[cfg(any())]
                NonExistingVariant,
            }

            #[cfg(all())]
            #[cfg_attr(all(), foo)]
            fn foo()
            {}

            cfg!(any());

            #[cfg(all())] {
                42
            }

            #[cfg(any())] {
                27
            }
        },
        stringify! {
            enum _WithoutCfgEval {
            }

            #[cfg(all())] // <- Redundantly kept.
            #[foo]
            fn foo()
            {}

            // Not expanded.
            cfg!(any());

            #[cfg(all())] /* <- Redundantly kept. */ {
                42
            }
        },
    );
}

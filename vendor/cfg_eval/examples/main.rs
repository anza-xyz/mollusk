#[macro_use]
extern crate cfg_eval;

#[macro_use]
extern crate macro_rules_attribute;

macro_rules! debugger {( $($input:tt)* ) => (
    println! { "{}", stringify!($($input)*) }
)}

fn main()
{
    #[apply(debugger)]
    enum _WithoutCfgEval {
        #[cfg(any())]
        NonExistingVariant,
    }

    #[cfg_eval]
    #[apply(debugger)]
    enum _WithCfgEval {
        #[cfg(any())]
        NonExistingVariant,
    }

    #[cfg_eval]
    #[apply(debugger)]
    const _: () = {
        enum _WithoutCfgEval {
            #[cfg(any())]
            NonExistingVariant,
        }
        #[cfg(all())] #[cfg_attr(all(), foo)] fn foo() {}
        #[cfg(all())] { 42 }
        #[cfg(any())] { 27 }
    };
}

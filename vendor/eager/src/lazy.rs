
///
/// [[eager!](macro.eager.html)] Used within an [`eager!`](macro.eager.html) to revert to lazy expansion.
///
/// If this macro is called independently of `eager!`, it expands to `eager!{lazy!{...}}`.
///
#[macro_export]
macro_rules! lazy {
	($($all:tt)*) => {
		eager!{
			lazy!{
				$($all)*
			}
		}
	};
}

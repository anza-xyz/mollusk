
///
/// Declares [eager!](macro.eager.html)-enabled macros.
///
/// # Usage
///
/// Wraps the usual `macro_rules!` syntax. First an identifier must be given, preceded by '$'.
/// Then any number of macro declarations can be given using the usual `macro_rules!` syntax.
/// Documentation and attributes are also given in the
/// usual way just before each `macro_rules!`, i.e. inside `eager_macro_rules!`.
///
/// Some restrictions apply to the `macro_rules!` declarations:
///
/// * The identifier given at the beginning must not collide with any macro variable name
/// used in any rule in any macro to be declared.
/// * No rules should accept `@eager` as the first token, as this could conflict with the
/// implementation of `eager!`. Wildcards are acceptable, as `eager_macro_rules!` will automatically
/// resolve the ambiguity with the `eager!` implementation.
///
/// # `eager!`-enabling example
///
/// [eager!](macro.eager.html)-enabling the following macro:
/// ```
/// /// Some documentation
/// #[macro_export]
/// macro_rules! some_macro{
/// 	()=>{};
/// }
/// ```
/// is done by wrapping it in `eager_macro_rules!` as follows:
/// ```
/// #[macro_use] extern crate eager;
/// eager_macro_rules!{ $eager_1
/// 	/// Some documentation
///     #[macro_export]
///     macro_rules! some_macro{
/// 	    ()=>{};
///     }
/// }
/// ```
/// where `()=>{};` is the list of rules that comprise the macro, and no macro variable is called
/// `$eager_1`.
///
#[macro_export]
macro_rules! eager_macro_rules{

// Start by decoding the initial values
	(
		$dollar1:tt $id_1:ident
		$(
			$(#[$($metas:tt)*])*
			macro_rules! $macro_name:ident {
				$($rules:tt => $expansions:tt);* $(;)*
			}
		)+
	)=>{
		$(
			eager_macro_rules_internal!{
				@first[
					$(#[$($metas)*])*
					$macro_name $dollar1 $id_1
				]
				$($rules => $expansions)*
			}
		)+
	};
}

#[macro_export]
#[doc(hidden)]
macro_rules! eager_macro_rules_internal{
// If there are no more rules, finish
	(
		@first[
			$(#[$($metas:tt)*])*
			$macro_name:ident $dollar1:tt $id_1:ident
			$($prev_grammar:tt => $prev_expansion:tt)*
		]
	) => {
		eager_macro_rules_internal!{
			@final[
				$(#[$($metas)*])*
				$macro_name$dollar1 $id_1
				$($prev_grammar => $prev_expansion)*
			]
		}
	};

//Handle the 3 different block type before the '=>'
	(
		@first[
			$(#[$($metas:tt)*])*
			$macro_name:ident $dollar1:tt $id_1:ident
			$($prev_grammar:tt => $prev_expansion:tt)*
		]
		{$($next_grammar:tt)*} $($rest:tt)+
	) => {
		eager_macro_rules_internal!{
			@expansion[
				$(#[$($metas)*])*
				$macro_name$dollar1 $id_1
				$($prev_grammar => $prev_expansion)*
				[$($next_grammar)*]
			]
			$($rest)+
		}
	};
	(
		@first[
			$(#[$($metas:tt)*])*
			$macro_name:ident $dollar1:tt $id_1:ident
			$($prev_grammar:tt => $prev_expansion:tt)*
		]
		($($next_grammar:tt)*) $($rest:tt)+
	) => {
		eager_macro_rules_internal!{
			@expansion[
				$(#[$($metas)*])*
				$macro_name$dollar1 $id_1
				$($prev_grammar => $prev_expansion)*
				[$($next_grammar)*]
			]
			$($rest)+
		}
	};
	(
		@first[
			$(#[$($metas:tt)*])*
			$macro_name:ident $dollar1:tt $id_1:ident
			$($prev_grammar:tt => $prev_expansion:tt)*
		]
		[$($next_grammar:tt)*] $($rest:tt)+
	) => {
		eager_macro_rules_internal!{
			@expansion[
				$(#[$($metas)*])*
				$macro_name$dollar1 $id_1
				$($prev_grammar => $prev_expansion)*
				[$($next_grammar)*]
			]
			$($rest)+
		}
	};
	
// Handle the 3 different block types after the '=>'
	(
		@expansion[
			$(#[$($metas:tt)*])*
			$macro_name:ident $dollar1:tt $id_1:ident
			$({$($prev_grammar:tt)*} => $prev_expansion:tt)*
			[$($next_grammar:tt)*]
		]
		 => {$($next_expansion:tt)*} $($rest:tt)*
	) => {
		eager_macro_rules_internal!{
			@first[
				$(#[$($metas)*])*
				$macro_name$dollar1 $id_1
				$({$($prev_grammar)*}  => $prev_expansion)*
				{$($next_grammar)*} => {$($next_expansion)*}
			]
			$($rest)*
		}
	};
	(
		@expansion[
			$(#[$($metas:tt)*])*
			$macro_name:ident $dollar1:tt $id_1:ident
			$({$($prev_grammar:tt)*} => $prev_expansion:tt)*
			[$($next_grammar:tt)*]
		]
		 => ($($next_expansion:tt)*) $($rest:tt)*
	) => {
		eager_macro_rules_internal!{
			@first[
				$(#[$($metas)*])*
				$macro_name$dollar1 $id_1
				$({$($prev_grammar)*}  => $prev_expansion)*
				{$($next_grammar)*} => {$($next_expansion)*}
			]
			$($rest)*
		}
	};
	(
		@expansion[
			$(#[$($metas:tt)*])*
			$macro_name:ident $dollar1:tt $id_1:ident
			$({$($prev_grammar:tt)*} => $prev_expansion:tt)*
			[$($next_grammar:tt)*]
		]
		 => [$($next_expansion:tt)*] $($rest:tt)*
	) => {
		eager_macro_rules_internal!{
			@first[
				$(#[$($metas)*])*
				$macro_name$dollar1 $id_1
				$({$($prev_grammar)*}  => $prev_expansion)*
				{$($next_grammar)*} => {$($next_expansion)*}
			]
			$($rest)*
		}
	};

// Output
	(	@final[
			$(#[$($metas:tt)*])*
			$macro_name:ident $dollar1:tt $id_1:ident
			$({$($rules_grammar:tt)*} => {$($rules_expansion:tt)*})+
		]
	)=>{
		$(#[$($metas)*])*
		macro_rules! $macro_name{
			$(
				// First the eager supporting version
				{
					@eager[$dollar1($dollar1 $id_1:tt)*]
					$($rules_grammar)*
				} => {
					eager_internal!{
						@from_macro[$dollar1($dollar1 $id_1)*]
						$($rules_expansion)*
					}
				};
			)+
			
			$(
				// Then the pure version. We put the pure versions
				// last such that if it contains a '$($all:tt)*' rule,
				// the pure version will not catch an eager call.
				{$($rules_grammar)*} => {$($rules_expansion)*};
			)+
		}
	};
}



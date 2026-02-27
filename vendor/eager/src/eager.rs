///
/// Emulates eager expansion of macros.
///
/// # Example
/// ```
/// #[macro_use]
/// extern crate eager;
///
/// //Declare an eager macro
/// eager_macro_rules!{ $eager_1
///     macro_rules! plus_1{
///         ()=>{+ 1};
///     }
/// }
///
/// fn main(){
/// 	// Use the macro inside an eager! call to expand it eagerly
/// 	assert_eq!(4, eager!{2 plus_1!() plus_1!()});
/// }
/// ```
///
/// # Usage
///
/// `eager!` can wrap any code, and if that code contains a macro call, that macro will be
/// expanded before its consumer. This means:
///
/// * If a macro call is given as an argument to another macro, the first macro will be expanded
/// first.
/// * All macros will be fully expanded before `eager!` expands. Therefore, otherwise illegal
/// intermediate expansion steps are possible.
///
/// `eager!` does not work with any macro; only macros declared using [`eager_macro_rules!`] may be
/// used. Such macros are said to be `eager!`-enabled.
///
/// To enable the use of non-`eager!`-enabled macros inside an `eager!` call,
/// a `lazy!` block can be inserted. Everything inside the `lazy!` block will be lazily expanded,
/// while everything outside it will continue to be eagerly expanded. Since, `lazy!` reverts
/// to the usual rules for macro expansion, an `eager!` block can be inserted inside the `lazy!`
/// block, to re-enable eager expansion for some subset of it.
///
/// [`eager_macro_rules!`]: macro.eager_macro_rules.html
/// [`lazy!`]: macro.lazy.html
/// # Cons
///
/// * Because of the way `eager!` is implemented - being a hack of recursive macros - the compiler's
/// default macro recursion limit is quickly exceeded. Therefore, `#![recursion_limit="256"]`
/// must be used in most situations - potentially with a higher limit -
/// such that expansion can happen.
///
/// * Debugging an eagerly expanded macro is very difficult and requires intimate knowledge
/// of the implementation of `eager!`. There is no way to mitigate this, except to try and
/// recreate the bug without using `eager!`. Likewise, the error messages the compiler will
/// emit are exponentially more cryptic than they already would have been.
///
/// * Only `eager!`-enabled macros can be eagerly expanded, so existing macros do not gain much.
/// The `lazy!` block alleviates this a bit, by allowing the use of existing macros in it,
/// while eager expansion can be done around them.
/// Luckily, `eager!`-enabling an existing macro should not be too much
/// trouble using [`eager_macro_rules!`].
///
/// ---
/// # Macro expansions
///
/// Rust is lazy when it comes to macro expansion. When the compiler sees a macro call, it will
/// try to expand the macro without looking at its arguments or what the expansion becomes.
/// Using `eager!`, previously illegal macro expansions can be made possible.
///
/// The following is a non-exhaustive list of illegal macro patterns that can be used with `eager!`.
///
/// ### The arguments to a macro usually cannot be the resulting expansion of another macro call:
/// Say you have a macro that adds two numbers:
/// ```ignore
/// macro_rules! add{
///     ($e1:expr, $e2:expr)=> {$e1 + $e2}
/// }
/// ```
/// And a macro that expands to two comma-separated numbers:
///
/// ```ignore
/// macro_rules! two_and_three{
///     ()=>{2,3}
/// }
/// ```
///
/// You cannot use the expansion of `two_and_three!` as an argument to `add!`:
/// ```ignore
/// let x = add!(two_and_three!()); // error
/// ```
/// The compiler will complain about no rule in `add!` accepting `two_and_three`, since it does not
/// get expanded before the `add!`, who requires two expressions and not just one.
///
/// With eager expansion, this can be made possible:
/// ```
/// #[macro_use]
/// extern crate eager;
///
/// eager_macro_rules!{ $eager_1
///     macro_rules! add{
///         ($e1:expr, $e2:expr)=> {$e1 + $e2}
///     }
///
///     macro_rules! two_and_three{
///     	()=>{2,3}
///     }
/// }
///
/// fn main(){
/// 	let x = eager!{add!(two_and_three!())};
/// 	assert_eq!(5, x);
/// }
/// ```
///
/// ### Macros are illegal in some contexts (e.g. as an identifier)
///
/// Say you have a macro that expands to an identifier:
/// ```ignore
/// macro_rules! id{
///     ()=> {SomeStruct}
/// }
/// ```
/// And want to use it to declare a struct:
/// ```ignore
/// struct id!(){
///     v: u32
/// }
/// ```
///
/// This will not compile since macros are illegal in identifier position. The compiler does
/// not check whether the expansion of the macro will result in valid Rust code.
///
/// With eager expansion, `id!` will expand before the `eager!` block , making it possible to use it
/// in an identifier position:
/// ```
/// #[macro_use]
/// extern crate eager;
///
/// eager_macro_rules!{ $eager_1
///     macro_rules! id{
///         ()=> {SomeStruct}
/// 	}
/// }
///
/// eager!{
///     struct id!(){
///         v: u32
///     }
/// }
///
/// fn main(){
/// 	let some_struct = SomeStruct{v: 4};
///     assert_eq!(4, some_struct.v);
/// }
/// ```
/// To circumvent any restriction on where macros can be used, we can therefore just wrap
/// the code surrounding the macro call with `eager!`. The `eager!` must still be in a valid position,
/// but in the worst case it can be put around the whole item
/// (struct, trait, implement, function, etc.).
///
///
/// ### No intermediate expansion step can include invalid syntax
///
/// Say we want to create a macro that interprets natural language, converting it into an expression.
///
/// We start by declaring a macro that interprets operator words:
/// ```ignore
/// macro_rules! op{
///     ( plus ) => { + };
///     ( minus ) => { - };
/// }
/// ```
///
/// We then declare a macro that interprets integer words:
/// ```ignore
/// macro_rules! integer{
///     ( one ) => { 1 };
///     ( two ) => { 2 };
/// }
/// ```
///
/// Lastly, we declare the top-level macro that uses the previous two macros to
/// expand into an expression:
/// ```ignore
/// macro_rules! calculate{
///     ( $lhs:tt $op:tt $rhs:tt ) => {
///          integer!{$lhs} op!{$op} integer!{$rhs}
///     };
/// }
/// ```
///
/// Using this macro will fail to compile:
/// ```ignore
/// let x = calculate!(one plus two); //Error
/// ```
///
/// Looking at the first expansion step we can see that three macro calls in a sequence
/// are not a valid expression:
/// ```ignore
/// let x = integer!(one) op!{plus} integer!(two); //Error
/// ```
///
/// We can circumvent this restriction, by having `calculate!` wrap its output in an `eager!`:
///
/// ```
/// #[macro_use]
/// extern crate eager;
///
/// eager_macro_rules!{ $eager_1
///     macro_rules! op{
///         ( plus ) => { + };
///         ( minus ) => { - };
///     }
///
///     macro_rules! integer{
///         ( one ) => { 1 };
///         ( two ) => { 2 };
///     }
///
///     macro_rules! calculate{
///         ( $lhs:tt $op:tt $rhs:tt ) => {
///              eager!{integer!{$lhs} op!{$op} integer!{$rhs}}
///         };
/// 	}
/// }
///
/// fn main(){
/// 	let x = calculate!(one plus two);
/// 	assert_eq!(3, x);
/// }
/// ```
/// In this case, `calculate!` does not actually have to be `eager!`-enabled, since it is not inserted
/// into an `eager!` block. Though - as per [the conventions](#conventions) - we do enable it such
/// that others may later use it inside an `eager!` block.
///
///
/// # Conventions
///
/// Since we expect the use of this macro to be broadly applicable, we propose the following
/// conventions for the Rust community to use, to ease interoperability.
///
/// ### Documentation
///
/// To make it clearly visible that a given macro is `eager!`-enabled, its short rustdoc description
/// must start with a pair of brackets, within which a link to the official `eager!` macro documentation
/// must be provided. The link's visible text must be 'eager!' and
/// the brackets must not be part of the link.
///
/// ### Auxiliary variable
///
/// The auxiliary variable that must always be provided to `eager_macro_rules!`
/// must use the identifier `eager_1`. This makes it easier for everyone to
/// get used to its presence and ignore it. By having it be the same in every project,
/// no one has to think about why a given project uses some specific identifier.
///
/// # Trivia
///
/// * Ironically, `eager!` is not technically `eager!`-enabled. Instead, it ignores itself if
/// it is nested or a macro expands into an `eager!` block.
/// Likewise, `eager_macro_rules!` is not `eager!`-enabled, though this might be possible.
///
/// * `lazy!` is treated by `eager!` as a keyword and not a macro.
///
/// * `eager_macro_rules!`'s auxiliary variable is affectionately called `Simon`.
/// This nickname should probably not be used as the identifier in production code.
/// Before reaching production, though...
///
/// * Simon once had a brother called `Garkel`.
///
/// * It requires continuous effort from [Emoun](http://github.com/Emoun) to not
/// forcibly rename `eager_macro_rules!` to `eager_macros_rule`.
///
///
#[macro_export]
macro_rules! eager{
	(
		$($all:tt)*
	)=>{
		eager_internal!{
			@check_expansion[
				[[][][][]]
			]
			$($all)*
		}
	};
}

/*
Decoded format:
[ [] [] [] [] {} ]
  1  2  3  4  5

1. The mode, either `[]` for eager or `[@lazy]` for lazy. Specifies whether the
current decode mode is eager or lazy. If there is more input, then that input must be
decoded in the mode. If there is no input, then the modefix must be decoded in the opposite mode.

2. The modefix (mode postfix). Contains input that has yet to be decoded, and that would have
to be decoded in the opposite mode to what is in 1.

3. The block prefix, i.e. input that came before the current block (if there is one).
Contains input that has been decoded and expanded completely. It is in
reverse order that what it should be in the final result.

4. The block postfix, i.e. input that came after the current block. If there is no
block, then this must be empty.

5. The current block, optional, and can be either `{}`, `[]`, or `()` which specifies the type
of the block in the input. While the contents of the block are being decoded, it is empty.
When the content has been decoded, checked, and expanded where appropriate it is input into
the block.

## Decoding workflow

The decoding starts with an empty level that is by default in eager mode: `[[] [] [] []]`.

Any input token that is not a block needs no work, therefore it is immediately put
in the prefix as it can be output as is. We call these tokens simple tokens.
We always token munch, which means the prfix is always in reverse order.
We will unreverse it in the end using the `reverse!` macro.
So if the input starts with the simple tokens `1 2 3`, the level will look like this:
`[[] [] [3 2 1] []]`.

Say the rest of the input is `{4 5 6} 7 8`. The block could contain a macro call that needs
to be eagerly expanded, so we cannot just add the block to the prefix yet. To check the contents
of the block we add the block to the current level, add the rest of the input (after the block)
to the postfix, and make the contents of the block as our input. We also add a new level, which
we will use to decode the blocks content.
The levels are a stack, always using the top to decode, and putting new one on the top.
The result is our decoded input becomes:
```
[[] [] [] []] // Used to decode the block contents
[[] [] [3 2 1] [7 8] {}]
```
And our input is `4 5 6`. We can now ignore the second level (the original level) and trivially
decode the simple tokens that were in the block:
```
[[] [] [6 5 4] []]
[[] [] [3 2 1] [7 8] {}]
```
At this point we have no more input, which means the content of the block have been decoded
and checked for any expansion needs. To signal that the block is done, we promote the prefix
to the block in the second level, and pop the first level. Since the prefix is in reverse order,
we unreverse it when putting it in the block. This is done with token munching:
```
[[] [] [3 2 1] [7 8] {4 5 6}]
```
Since we still have no more input to decode, but we can see that we have checked a block, we
promote the block to prefix. At the same time, we take the blocks postfix and put it as input,
such that it can be decoded:
```
[[] [] [{4 5 6} 3 2 1] []]
```
The block contents do not need to be reversed since the prefix is only reversed on the token tree
level. Since the input after the block was just simple tokens we get:
```
[[] [] [8 7 {4 5 6} 3 2 1] []]
```
Now that we have no more input, no block, and no postfix, we know we have decoded everything
and the contents of the prefix are our result. so we output the prefix in reverse order:
`1 2 3 4 5 6 7 8`.

To see how we handle macros, say our input has a macro invocation instead of the blocks:
`1 2 some_macro!{t1 t2} 5 6`. We start, as usual, by decoding the first simple tokens.
```
[[] [] [! some_macro 2 1] []]
```
Note how the macro invocation is also put in the prefix. We then decode the block, checking its
contents. When they have been checked and promoted into the block we will have:
```
[[] [] [! some_macro 2 1] [5 6]{t1 t2}]
```
At this point we would previously have promoted the block to the prefix, but we can now see
that the prefix contains a macro invocation. Since we have checked the contents of the block,
we know that we can safely call the macro with it. We do so, removing the invocation from the
prefix, and removing the block. When the macro returns it will put its result as input, so the
first thing we do is extract the postfix to the input too, putting it after the macros result,
where it belongs. Our level will no look like:
```
[[] [] [2 1] []]
```
And say the macro expands to `3 4`, we will have the input `3 4 5 6`.
Using our previous rules, the result will be `1 2 3 4 5 6`.

Say we have a lazy block: `eager_macro_1!{} lazy!{ lazy_macro!{}} eager_macro_2!{}`
Say `eager_macro_1!` expands to `1 2`, and `eager_macro_2!` expands to `3 4`, and both are
`eager!`-enabled. `lazy_macro!` on the other hand is not `eager!`enabled.

We startin the usual way, and after expanding the first macro we will have the levels:
```
[[] [] [2 1] []]
```
and the input `lazy!{ lazy_macro!{}} eager_macro_2!{}`. We can now see that the `lazy!` block
is the opposite of the current mode (eager), so we will have to do a mode change. We take all
input that is after the `lazy!` block, and put it is modefix as it still needs to be eagerly
expanded. Then we change the mode to lazy. Lastly, we extract the content of the `lazy!` block
and put it as input. This will result in the levels:
```
[[@lazy] [eager_macro_2!{}] [2 1] []]
```
and the input `lazy_macro!{}`. At this point we continue as we would have previously done,
resulting in the block being checked (trivial since its empty) and no more input:
```
[[@lazy] [eager_macro_2!{}] [! lazy_macro 2 1] []{}]
```
Previously, we would have invoked `lazy_macro!`, since its block has been checked, but this time
we are in lazy expansion mode. Therefore, there is no need to call the macro and we just promote
the block to prefix immediately:
```
[[@lazy] [eager_macro_2!{}] [{}! lazy_macro 2 1] []]
```
At this point we have no more input and no block, but we still have something in the modefix.
So, we extract that into input, and switch the mode, since the input in modefix always needs to
be decoded in the opposite mode. So we get the levels:
```
[[] [] [{}! lazy_macro 2 1] []]
```
and input `eager_macro_2!{}`. The decoding now proceeds as previously described, resulting in
`1 2 lazy_macro!{} 3 4` as output.

Notes:

* When decoding blocks, managing the type of the block is critical. This is the reason the block
is not always `[]`. Therefore, when promoting blocks to prefix, make sure the block type is maintained.
The following are examples of the same promotion, except the input uses the different blocks:
`[[] [] [] [] {something}]` to `[[] [] [{something}] []]`.
`[[] [] [] [] (something)]` to `[[] [] [(something)] []]`.
`[[] [] [] [] [something]]` to `[[] [] [[something]] []]`.

* Promoting modefix to input (the last step above) must only be done after all other input and
blocks have been decoded fully.
*/
#[macro_export]
#[doc(hidden)]
macro_rules! eager_internal{
// Handle return from eager macro expansion
	(
		@from_macro[
			[$lazy:tt $modefix:tt $prefix:tt[$($postfix:tt)*]]
			$($rest_decoded:tt)*
		]
		$($expanded:tt)*
	) => {
		eager_internal!{
			@check_expansion[
				[$lazy $modefix $prefix []]
				$($rest_decoded)*
			]
			$($expanded)* $($postfix)*
		}
	};
// Decode input stream
	(	// If the next token is a block, check it (brace type)
		@check_expansion[
			[$lazy:tt $modefix:tt [$($prefix:tt)*][]]
			$($rest_decoded:tt)*
		]
		{$($body:tt)*} $($rest:tt)*
	)=>{
		eager_internal!{
			@check_expansion[
				[$lazy [][][]]
				[$lazy $modefix [$($prefix)*][$($rest)*]{}]
				$($rest_decoded)*
			]
			$($body)*
		}
	};
	(	// If the next token is a block, check it (parenthesis type)
		@check_expansion[
			[$lazy:tt $modefix:tt [$($prefix:tt)*][]]
			$($rest_decoded:tt)*
		]
		($($body:tt)*) $($rest:tt)*
	)=>{
		eager_internal!{
			@check_expansion[
				[$lazy [][][]]
				[$lazy $modefix [$($prefix)*][$($rest)*]()]
				$($rest_decoded)*
			]
			$($body)*
		}
	};
	(	// If the next token is a block, check it (bracket type)
		@check_expansion[
			[$lazy:tt $modefix:tt [$($prefix:tt)*][]]
			$($rest_decoded:tt)*
		]
		[$($body:tt)*] $($rest:tt)*
	)=>{
		eager_internal!{
			@check_expansion[
				[$lazy [][][]]
				[$lazy $modefix [$($prefix)*][$($rest)*][]]
				$($rest_decoded)*
			]
			$($body)*
		}
	};
// eager/lazy mode changes
	(	// If the next token is an 'eager!' macro call and we are already
		// in eager mode, ignore it, extracting the body. (brace type)
		@check_expansion[
			[[]$modefix:tt[$($prefix:tt)*][]]
			$($rest_decoded:tt)*
		]
		eager!{$($body:tt)*} $($rest:tt)*
	)=>{
		eager_internal!{
			@check_expansion[
				[[]$modefix[$($prefix)*][]]
				$($rest_decoded)*
			]
			$($body)* $($rest)*
		}
	};
	(	// If the next token is an 'eager!' macro call and we are already
		// in eager mode, ignore it, extracting the body. (parenthesis type)
		@check_expansion[
			[[]$modefix:tt[$($prefix:tt)*][]]
			$($rest_decoded:tt)*
		]
		eager!($($body:tt)*) $($rest:tt)*
	)=>{
		eager_internal!{
			@check_expansion[
				[[]$modefix[$($prefix)*][]]
				$($rest_decoded)*
			]
			$($body)* $($rest)*
		}
	};
	(	// If the next token is an 'eager!' macro call and we are already
		// in eager mode, ignore it, extracting the body. (bracket type)
		@check_expansion[
			[[]$modefix:tt[$($prefix:tt)*][]]
			$($rest_decoded:tt)*
		]
		eager![$($body:tt)*] $($rest:tt)*
	)=>{
		eager_internal!{
			@check_expansion[
				[[]$modefix[$($prefix)*][]]
				$($rest_decoded)*
			]
			$($body)* $($rest)*
		}
	};
	(	// If the next token is an 'lazy!' macro call and we are already
		// in lazy mode, ignore it, extracting the body. (brace type)
		@check_expansion[
			[[@lazy]$modefix:tt[$($prefix:tt)*][]]
			$($rest_decoded:tt)*
		]
		lazy!{$($body:tt)*} $($rest:tt)*
	)=>{
		eager_internal!{
			@check_expansion[
				[[@lazy]$modefix[$($prefix)*][]]
				$($rest_decoded)*
			]
			$($body)* $($rest)*
		}
	};
	(	// If the next token is an 'lazy!' macro call and we are already
		// in lazy mode, ignore it, extracting the body. (parenthesis type)
		@check_expansion[
			[[@lazy]$modefix:tt[$($prefix:tt)*][]]
			$($rest_decoded:tt)*
		]
		lazy!($($body:tt)*) $($rest:tt)*
	)=>{
		eager_internal!{
			@check_expansion[
				[[@lazy]$modefix[$($prefix)*][]]
				$($rest_decoded)*
			]
			$($body)* $($rest)*
		}
	};
	(	// If the next token is an 'lazy!' macro call and we are already
		// in lazy mode, ignore it, extracting the body. (bracket type)
		@check_expansion[
			[[@lazy]$modefix:tt[$($prefix:tt)*][]]
			$($rest_decoded:tt)*
		]
		lazy![$($body:tt)*] $($rest:tt)*
	)=>{
		eager_internal!{
			@check_expansion[
				[[@lazy]$modefix[$($prefix)*][]]
				$($rest_decoded)*
			]
			$($body)* $($rest)*
		}
	};
	(	// If the next token is an 'eager!' macro call and we are
		// in lazy mode (brace type)
		@check_expansion[
			[[@lazy][][$($prefix:tt)*][]]
			$($rest_decoded:tt)*
		]
		eager!{$($body:tt)*} $($rest:tt)*
	)=>{
		eager_internal!{
			@check_expansion[
				[[][$($rest)*][$($prefix)*][]]
				$($rest_decoded)*
			]
			$($body)*
		}
	};
	(	// If the next token is an 'eager!' macro call and we are
		// in lazy mode, ignore it, extracting the body. (parenthesis type)
		@check_expansion[
			[[@lazy][][$($prefix:tt)*][]]
			$($rest_decoded:tt)*
		]
		eager!($($body:tt)*) $($rest:tt)*
	)=>{
		eager_internal!{
			@check_expansion[
				[[][$($rest)*][$($prefix)*][]]
				$($rest_decoded)*
			]
			$($body)*
		}
	};
	(	// If the next token is an 'eager!' macro call and we are
		// in lazy mode, ignore it, extracting the body. (bracket type)
		@check_expansion[
			[[@lazy][][$($prefix:tt)*][]]
			$($rest_decoded:tt)*
		]
		eager![$($body:tt)*] $($rest:tt)*
	)=>{
		eager_internal!{
			@check_expansion[
				[[][$($rest)*][$($prefix)*][]]
				$($rest_decoded)*
			]
			$($body)*
		}
	};
	(	// If the next token is an 'lazy!' macro call and we are
		// in eager mode, ignore it, extracting the body. (brace type)
		@check_expansion[
			[[][][$($prefix:tt)*][]]
			$($rest_decoded:tt)*
		]
		lazy!{$($body:tt)*} $($rest:tt)*
	)=>{
		eager_internal!{
			@check_expansion[
				[[@lazy][$($rest)*][$($prefix)*][]]
				$($rest_decoded)*
			]
			$($body)*
		}
	};
	(	// If the next token is an 'eager!' macro call and we are already
		// in eager mode, ignore it, extracting the body. (parenthesis type)
		@check_expansion[
			[[][][$($prefix:tt)*][]]
			$($rest_decoded:tt)*
		]
		lazy!($($body:tt)*) $($rest:tt)*
	)=>{
		eager_internal!{
			@check_expansion[
				[[@lazy][$($rest)*][$($prefix)*][]]
				$($rest_decoded)*
			]
			$($body)*
		}
	};
	(	// If the next token is an 'eager!' macro call and we are already
		// in eager mode, ignore it, extracting the body. (bracket type)
		@check_expansion[
			[[][][$($prefix:tt)*][]]
			$($rest_decoded:tt)*
		]
		lazy![$($body:tt)*] $($rest:tt)*
	)=>{
		eager_internal!{
			@check_expansion[
				[[@lazy][$($rest)*][$($prefix)*][]]
				$($rest_decoded)*
			]
			$($body)*
		}
	};
// end eager/lazy mode switches
	(	// If the next token isn't any of the above
		// it is safe to add it to the prefix
		@check_expansion[
			[$lazy:tt $modefix:tt [$($prefix:tt)*][]]
			$($rest_decoded:tt)*
		]
		$next:tt $($rest:tt)*
	)=>{
		eager_internal!{
			@check_expansion[
				[$lazy $modefix[$next $($prefix)*][]]
				$($rest_decoded)*
			]
			$($rest)*
		}
	};
// Done decoding input
// Expanding macros in eager mode
	(	// When there is no more input, the last input was a macro call,
		// and we are in eager mode, call the macro eagerly
		// (brace type)
		@check_expansion[
			[[]$modefix:tt[! $macro_name:tt $($prefix:tt)*][$($postfix:tt)*]{$($body:tt)*}]
			$($rest_decoded:tt)*
		]
	)=>{
		$macro_name!{
			@eager[
				[[]$modefix[$($prefix)*][$($postfix)*]]
				$($rest_decoded)*
			]
			$($body)*
		}
	};
	(	// When there is no more input and the last input was a macro call,
		// and we are in eager mode, call the macro eagerly
		// (parenthesis type)
		@check_expansion[
			[[]$modefix:tt[! $macro_name:tt $($prefix:tt)*][$($postfix:tt)*]($($body:tt)*)]
			$($rest_decoded:tt)*
		]
	)=>{
		$macro_name!{
			@eager[
				[[]$modefix[$($prefix)*][$($postfix)*]]
				$($rest_decoded)*
			]
			$($body)*
		}
	};
	(	// When there is no more input and the last input was a macro call,
		// and we are in eager mode, call the macro eagerly
		// (bracket type)
		@check_expansion[
			[[]$modefix:tt[! $macro_name:tt $($prefix:tt)*][$($postfix:tt)*][$($body:tt)*]]
			$($rest_decoded:tt)*
		]
	)=>{
		$macro_name!{
			@eager[
				[[]$modefix[$($prefix)*][$($postfix)*]]
				$($rest_decoded)*
			]
			$($body)*
		}
	};
// Promote modefix to input
	(	// When there is no more input, but there is some postfix,
		// if the current mode is eager, redecode the postfix in lazy mode
		@check_expansion[
			[[][$($modefix:tt)+] $prefix:tt []]
			$($rest:tt)*
		]
	)=>{
		eager_internal!{
			@check_expansion[
				[[@lazy][] $prefix []]
				$($rest)*
			]
			$($modefix)+
		}
	};
	(	// When there is no more input, but there is some postfix,
		// if the current mode is lazy, redecode the postfix in eager mode
		@check_expansion[
			[[@lazy][$($modefix:tt)+] $prefix:tt []]
			$($rest:tt)*
		]
	)=>{
		eager_internal!{
			@check_expansion[
				[[][] $prefix []]
				$($rest)*
			]
			$($modefix)+
		}
	};
// end Promote modefix to input
// Promote prefix
	(	// When there is no more input and the last input wasn't a macro call in eager mode
		// insert it into the previous block (brace type)
		@check_expansion[
			[$lazy_0:tt $modefix_0:tt [$last:tt $($last_rest:tt)*] []]
			[$lazy:tt $modefix:tt $prefix:tt $postfix:tt {$($body:tt)*}]
			$($rest:tt)*
		]
	)=>{
		eager_internal!{
			@check_expansion[
				[$lazy_0 $modefix_0 [$($last_rest)*] []]
				[$lazy $modefix $prefix $postfix {$last $($body)*}]
				$($rest)*
			]
		}
	};
	(	// When there is no more input and the last input wasn't a macro call in eager mode
		// insert it into the previous block (parenthesis type)
		@check_expansion[
			[$lazy_0:tt $modefix_0:tt[$last:tt $($last_rest:tt)*] []]
			[$lazy:tt $modefix:tt $prefix:tt $postfix:tt ($($body:tt)*)]
			$($rest:tt)*
		]
	)=>{
		eager_internal!{
			@check_expansion[
				[$lazy_0 $modefix_0 [$($last_rest)*] []]
				[$lazy $modefix $prefix $postfix ($last $($body)*)]
				$($rest)*
			]
		}
	};
	(	// When there is no more input and the last input wasn't a macro call in eager mode
		// insert it into the previous block (bracket type)
		@check_expansion[
			[$lazy_0:tt $modefix_0:tt[$last:tt $($last_rest:tt)*] []]
			[$lazy:tt $modefix:tt $prefix:tt $postfix:tt [$($body:tt)*]]
			$($rest:tt)*
		]
	)=>{
		eager_internal!{
			@check_expansion[
				[$lazy_0 $modefix_0 [$($last_rest)*] []]
				[$lazy $modefix $prefix $postfix [$last $($body)*]]
				$($rest)*
			]
		}
	};
	(	// When there is no more input, prefix or postfix,
		// but there is a previous block, remove the input catcher
		@check_expansion[
			[$lazy_0:tt[][][]]
			$([$lazy:tt $modefix:tt $prefix:tt $postfix:tt $body:tt])+
			
		]
	)=>{
		eager_internal!{
			@check_expansion[
				$([$lazy $modefix $prefix $postfix $body])+
			]
		}
	};
// end promote prefix
// Promote block to prefix
	(	// When there is no more input but a block,
		// the block must have already been checked,
		// therefore, begin promoting to prefix (brace type)
		@check_expansion[
			[$lazy:tt $modefix:tt [$($prefix:tt)*][$($postfix:tt)*]{$($body:tt)*}]
			$($rest:tt)*
		]
	)=>{
		eager_internal!{
			@check_expansion[
				[$lazy $modefix [{$($body)*} $($prefix)*][]]
				$($rest)*
			]
			$($postfix)*
		}
	};
	(	// When there is no more input and but a block
		// the block must have already been checked,
		// so output everything (parenthesis type)
		@check_expansion[
			[$lazy:tt $modefix:tt [$($prefix:tt)*][$($postfix:tt)*]($($body:tt)*)]
			$($rest:tt)*
		]
	)=>{
		eager_internal!{
			@check_expansion[
				[$lazy $modefix [($($body)*) $($prefix)*][]]
				$($rest)*
			]
			$($postfix)*
		}
	};
	(	// When there is no more input and but a block
		// the block must have already been checked,
		// so output everything (bracket type)
		@check_expansion[
			[$lazy:tt $modefix:tt [$($prefix:tt)*][$($postfix:tt)*][$($body:tt)*]]
			$($rest:tt)*
		]
	)=>{
		eager_internal!{
			@check_expansion[
				[$lazy $modefix [[$($body)*] $($prefix)*][]]
				$($rest)*
			]
			$($postfix)*
		}
	};
// End Promote block to prefix
// Finished
	(	// When there is no more input and no block
		// output the result, reversing it to ensure correct order
		@check_expansion[
			[$lazy:tt [][$($result:tt)*][]]
		]
	)=>{
		eager_internal!{
			@reverse_tt[
				[$($result)*]
				[]
			]
		}
	};
	
// To finish, reverse-output the result
	(
		// While there is more to reverse
		@reverse_tt[
			[$to_reverse_next:tt $($to_reverse_rest:tt)+]
			[$($reversed:tt)*]
		]
	) => {
		eager_internal!{
			@reverse_tt[
				[$($to_reverse_rest)+]
				[$to_reverse_next $($reversed)*]
			]
		}
	};
	(
		// Done reversing
		@reverse_tt[
			[$to_reverse_last:tt]
			[$($reversed:tt)*]
		]
	) => {
		$to_reverse_last $($reversed)*
	};
}



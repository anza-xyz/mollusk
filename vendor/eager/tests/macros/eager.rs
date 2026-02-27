#![allow(dead_code)]

mod test_prefix{
	/*
	Tests that input can be followed by a macro call
	*/
	eager_macro_rules! {$eager_1
		macro_rules! test_macro{
			{ ! ! } =>{
				eager!{
					struct
					test_macro!{??}
				}
			};
			{ ?? } =>{
				SomeName test_macro!{| |}
			};
			{ | | } =>{
				{field: u32}
			};
		}
	}
	test_macro!(!!);
}
mod test_postfix{
	/*
	Tests that a macro call can be precede some non-macro tokens.
	*/
	eager_macro_rules! {$eager_1
		macro_rules! test_macro{
			(!!)=>{
				eager!{
					struct
					test_macro!{??}
				}
			};
			(??)=>{
				test_macro!{||}
				{field: u32}
			};
			(||)=>{
				SomeStruct
			};
		}
	}
	test_macro!(!!);
}
mod test_multiple_calls{
	/*
	Test that multiple macro call can be done after each other
	*/
	use std::marker::PhantomData;
	eager_macro_rules! {$eager_1
		macro_rules! mac1{
			{
				$typ:tt
			}=>{
				$typ
			};
		}
	}
	macro_rules! mac2{
		($V:ident,$eq:tt)=>{
			eager!{
				struct $V<V,W> where W:
				mac1!{$eq},
				V:
				mac1!{$eq}
				{ph1: PhantomData<W>,ph2: PhantomData<V>}
			}
		}
	}
	mac2!(SomeStruct, PartialEq);
}
mod test_nested_calls{
	/*
	Tests that macro call can be nested, i.e. one macro's expansion is the input
	to another macro.
	*/
	use std::marker::PhantomData;
	eager_macro_rules! {$eager_1
		macro_rules! mac1{
			(!!)=>{
				ph1: PhantomData<W>,ph2: PhantomData<V>
			};
		}
	
		macro_rules! mac2{
			{
				$($to_encapsulate:tt)*
			}=>{
				{$($to_encapsulate)*}
			}
		}
		
		macro_rules! mac3 {
			($some:ident)=>{
				eager!{
					struct $some<V,W>
					mac2!{
						mac1!{mac3!{{SomeThing}}}
					}
				}
			};
			(
				{SomeThing}
			)=>{
				!!
			};
		}
	}
	mac3!{SomeStruct}
}
mod test_non_call_block_ignored{
	/*
	Tests that a block that is not part of a macro call is left as is.
	(Assuming there is no macro call in it).
	*/
	eager_macro_rules! {$eager_1
		macro_rules! test_macro{
			() => {
				eager!{
					test_macro!{1}
					{field: i32}
					struct SomeSecondStruct{}
				}
			};
			( 1 ) => {
				struct SomeStruct
			};
		}
	}
	test_macro!{}
}
mod test_nested_eagers{
	/*
	Tests that using the eager! macro inside the body of another eager! call
	does nothing.
	*/
	eager_macro_rules! {$eager_1
		macro_rules! test_macro{
			() => {
				eager!{
					struct
					eager!{
						test_macro!{1}
					}
					{}
				}
			};
			( 1 ) => {
				 SomeStruct
			};
		}
	}
	test_macro!{}
}
mod test_recursive_eagers{
	/*
	Tests that if an expansion creates a new 'eager!' call, it is ignored.
	*/
	eager_macro_rules! {$eager_1
		macro_rules! test_macro{
			() => {
				eager!{
					struct
					test_macro!{1}
					{}
				}
			};
			( 1 ) => {
				eager!{SomeStruct}
			};
		}
	}
	test_macro!{}
}
mod test_sequencial_blocks_arent_merged{
	/*
	Test that if two block are immediately after each other, they are not merged into one block.
	*/
	eager_macro_rules!{$eager_1
		macro_rules! test_macro{
			({1}{2}) => {"Success"};
			({1 2}) => {"Failure"};
		}
	}
	#[test]
	fn test(){
		assert_eq!("Success", eager!{test_macro!({1}{2})});
	}
}
mod test_block_before_macro_isnt_merged_with_expansion{
	/*
	Test that if there is a block before a macro call, whos expansion result starts
	with the same block type, that these two blocks arent merged after the macro is expanded.
	I.e. ' {1} some_macro!()' becomes '{1}{2}' and not '{1 2}'.
	*/
	eager_macro_rules!{$eager_1
		macro_rules! test_macro{
			({1}{2}) => {"Success"};
			({1 2}) => {"Failure"};
			() => {{2}}
		}
	}
	#[test]
	fn test(){
		assert_eq!("Success", eager!{
			test_macro!{
				{1}
				test_macro!()
			}
		});
	}
}

// Same tests as above, but with the '()' block type
mod paren_test_prefix{
	/*
	Tests that input can be followed by a macro call
	*/
	eager_macro_rules!{$eager_1
		macro_rules! test_macro{
			(!! ) =>{
				eager!{
					const N: i32 = test_macro!(1);
				}
			};
			( 1 ) =>{
				(5+5) test_macro!(2)
			};
			( 2	) =>{
				+ 1 test_macro!(3)
			};
			( 3 ) =>{
				+ (5)
			};
		}
	}
	test_macro!(!!);
	#[test]
	fn test(){
		assert_eq!(16, N);
	}
}
mod paren_test_postfix{
	/*
	Tests that a macro call can be followed by a macro call
	*/
	eager_macro_rules!{$eager_1
		macro_rules! test_macro{
			(!! ) =>{
				eager!{
					const N: i32 = test_macro!(1);
				}
			};
			( 1 ) =>{
				test_macro!(2) (5+5)
			};
			( 2 ) =>{
				test_macro!(3) 1 +
			};
			( 3 ) =>{
				(5) +
			};
		}
	}
	test_macro!(!!);
	#[test]
	fn test(){
		assert_eq!(16, N);
	}
}
mod paren_test_multiple_calls{
	/*
	Tests that multliple macro calls can be done in serial
	*/
	eager_macro_rules!{$eager_1
		macro_rules! test_macro{
			(!! ) =>{
				eager!{
					const N: i32 = test_macro!(1) + test_macro!(1) + test_macro!(1);
				}
			};
			( 1 ) =>{
				(5+5)
			};
		}
	}
	test_macro!(!!);
	#[test]
	fn test(){
		assert_eq!(30, N);
	}
}
mod paren_test_nested_calls{
	/*
	Tests that a macro call can be nested, where the input to one macro is the expansion of another.
	*/
	macro_rules ! test_macro_1 {
		(!!) =>{
			eager!{
				const N: i32 = test_macro_2!(test_macro_3!(test_macro_4!()));
			}
		};
	}
	eager_macro_rules!{$eager_1
		macro_rules! test_macro_2{
			( $($all:tt)* ) =>{
				($($all)*) + 2
			};
		}
		
		macro_rules! test_macro_3{
			( $($all:tt)* ) =>{
				1 + ($($all)*) + 2
			};
		}
	
		macro_rules! test_macro_4{
			( ) =>{
				4
			};
		}
	}
	test_macro_1!(!!);
	#[test]
	fn test(){
		assert_eq!(9, N);
	}
}
mod paren_test_non_call_block_ignored{
	
	eager_macro_rules!{$eager_1
		macro_rules! test_macro{
			() => {
				eager!{
					test_macro!(1)
					(4 + 4)
					 + 4
				}
			};
			( 1 ) => {
				4 +
			};
		}
	}
	#[test]
	fn test(){
		assert_eq!(16, test_macro!());
	}
}
mod paren_test_nested_eagers{
	/*
	Tests that using the eager! macro inside the body of another eager! call
	does nothing.
	*/
	eager_macro_rules!{$eager_1
		macro_rules! test_macro{
			() => {
				eager!(
					1
					eager!(
						test_macro!(1)
					)
				)
			};
			( 1 ) => {
				+ 2
			};
		}
	}
	#[test]
	fn test(){
		assert_eq!(3, test_macro!());
	}
}
mod pare_test_recursive_eagers{
	/*
	Tests that if an expansion creates a new 'eager!' call, it is ignored.
	*/
	eager_macro_rules!{$eager_1
		macro_rules! test_macro{
			() => {
				eager!(
					1 test_macro!(1)
				)
			};
			( 1 ) => {
				eager!(+ 2)
			};
		}
	}
	#[test]
	fn test(){
		assert_eq!(3, test_macro!());
	}
}

// Same tests as the two above, but with the '[]' block type
mod bracket_test_prefix{
	/*
	Tests that input can be followed by a macro call
	*/
	eager_macro_rules! {$eager_1
		macro_rules! test_macro{
			[ ! ! ] =>{
				eager![
					struct
					test_macro![??]
				];
			};
			[ ?? ] =>{
				SomeName test_macro![| |]
			};
			[ | | ] =>{
				{field: u32}
			};
		}
	}
	test_macro![!!];
}
mod bracket_test_postfix{
	/*
	Tests that a macro call can be precede some non-macro tokens.
	*/
	eager_macro_rules! {$eager_1
		macro_rules! test_macro{
			(!!)=>{
				eager![
					struct
					test_macro![??]
				];
			};
			(??)=>{
				test_macro![||]
				{field: u32}
			};
			(||)=>{
				SomeStruct
			};
		}
	}
	test_macro![!!];
}
mod bracket_test_multiple_calls{
	/*
	Test that multiple macro call can be done after each other
	*/
	use std::marker::PhantomData;
	eager_macro_rules! {$eager_1
		macro_rules! mac1{
			{
				$typ:tt
			}=>{
				$typ
			};
		}
	}
	macro_rules! mac2{
		($V:ident,$eq:tt)=>{
			eager!{
				struct $V<V,W> where W:
				mac1![$eq],
				V:
				mac1![$eq]
				{ph1: PhantomData<W>,ph2: PhantomData<V>}
			}
		}
	}
	mac2![SomeStruct, PartialEq];
}
mod bracket_test_nested_calls{
	/*
	Tests that macro call can be nested, i.e. one macro's expansion is the input
	to another macro.
	*/
	use std::marker::PhantomData;
	eager_macro_rules! {$eager_1
		macro_rules! mac1{
			(!!)=>{
				ph1: PhantomData<W>,ph2: PhantomData<V>
			};
		}
	
		macro_rules! mac2{
			{
				$($to_encapsulate:tt)*
			}=>{
				{$($to_encapsulate)*}
			}
		}
		
		macro_rules! mac3 {
			($some:ident)=>{
				eager!{
					struct $some<V,W>
					mac2![
						mac1![mac3![[SomeThing]]]
					]
				}
			};
			(
				[SomeThing]
			)=>{
				!!
			};
		}
	}
	mac3!{SomeStruct}
}
mod bracket_test_non_call_block_ignored{
	/*
	Tests that a block that is not part of a macro call is left as is.
	(Assuming there is no macro call in it).
	*/
	eager_macro_rules! {$eager_1
		macro_rules! test_macro{
			( $id:ident ) => {
				eager!{
					test_macro![1 $id]
					[1u32, 2u32, 3u32]
					;
				}
			};
			( 1 $id:ident ) => {
				let $id: [u32;3] =
			};
		}
	}
	
	#[test]
	fn test(){
		test_macro![array];
		assert_eq!{1,array[0]};
		assert_eq!{2,array[1]};
		assert_eq!{3,array[2]};
	}
}
mod bracket_test_nested_eagers{
	/*
	Tests that using the eager! macro inside the body of another eager! call
	does nothing.
	*/
	eager_macro_rules! {$eager_1
		macro_rules! test_macro{
			() => {
				eager!{
					struct
					eager![
						test_macro![1]
					]
					{}
				}
			};
			( 1 ) => {
				 SomeStruct
			};
		}
	}
	test_macro!{}
}
mod bracket_test_recursive_eagers{
	/*
	Tests that if an expansion creates a new 'eager!' call, it is ignored.
	*/
	eager_macro_rules! {$eager_1
		macro_rules! test_macro{
			() => {
				eager!{
					struct
					test_macro![1]
					{}
				}
			};
			( 1 ) => {
				eager![SomeStruct]
			};
		}
	}
	test_macro!{}
}
mod bracket_test_sequencial_blocks_arent_merged{
	/*
	Test that if two block are immediately after each other, they are not merged into one block.
	*/
	eager_macro_rules!{$eager_1
		macro_rules! test_macro{
			([1][2]) => {"Success"};
			([1 2]) => {"Failure"};
		}
	}
	#[test]
	fn test(){
		assert_eq!("Success", eager!{test_macro![[1][2]]});
	}
}
mod bracket_test_block_before_macro_isnt_merged_with_expansion{
	/*
	Test that if there is a block before a macro call, whos expansion result starts
	with the same block type, that these two blocks arent merged after the macro is expanded.
	I.e. ' {1} some_macro!()' becomes '{1}{2}' and not '{1 2}'.
	*/
	eager_macro_rules!{$eager_1
		macro_rules! test_macro{
			([1][2]) => {"Success"};
			([1 2]) => {"Failure"};
			() => {[2]}
		}
	}
	#[test]
	fn test(){
		assert_eq!("Success", eager!{
			test_macro![
				[1]
				test_macro![]
			]
		});
	}
}
//! We're keeping most of the generating together, to ease reading here and in `cargo_expand`.
//! For this reason, we also use more locals and types.
//! These locals, types, casts should be optimized away.
//! In simple cases they indeed are optimized away, but if some case is not, please report.
//!
//! ## Important
//!
//! We often do thing like:
//! ```ignore
//! quote! {
//!     #value_shifted
//!     value_shifted
//! }
//! ```
//! By convention, `#value_shifted` has its name because we define a `let value_shifted` inside that `TokenStream`.
//! So the above code means we're returning the value of `let value_shifted`.
//! Earlier on, we would have done something like this:
//! ```ignore
//! quote! {
//!     let value_shifted = { #value_shifted };
//!     value_shifted
//! }
//! ```
//! which aids in reading this here macro code, but doesn't help reading the generated code since it introduces
//! lots of new scopes (curly brackets). We need the scope since `#value_shifted` expands to multiple lines.
use super::*;

/// Top-level function which initializes the cursor and offsets it to what we want to read
///
/// `is_array_elem_getter` allows us to generate an array_at getter more easily
pub(crate) fn generate_getter_value(ty: &Type, offset: &TokenStream, is_array_elem_getter: bool) -> TokenStream {
    // if we generate `fn array_at(index)`, we need to offset to the array element
    let elem_offset = if is_array_elem_getter {
        let size = shared::generate_type_bitsize(ty);
        quote! {
            let size = #size;
            // cursor now starts at this element
            cursor >>= size * index;
        }
    } else {
        quote!()
    };

    let inner = generate_getter_inner(ty, true);
    quote! {
        // for ease of reading
        type ArbIntOf<T> = <T as Bitsized>::ArbitraryInt;
        type BaseIntOf<T> = <ArbIntOf<T> as Number>::UnderlyingType;
        // cursor is the value we read from and starts at the struct's first field
        let mut cursor = self.value.value();
        // this field's offset
        let field_offset = #offset;
        // cursor now starts at this field
        cursor >>= field_offset;
        #elem_offset

        #inner
    }
}

/// We heavily rely on the fact that transmuting into a nested array [[T; N1]; N2] can
/// be done in the same way as transmuting into an array [T; N1*N2].
/// Otherwise, nested arrays would generate even more code.
///
/// `is_getter` allows us to generate a try_from impl more easily
pub(crate) fn generate_getter_inner(ty: &Type, is_getter: bool) -> TokenStream {
    use Type::*;
    match ty {
        Tuple(tuple) => {
            let unbraced = tuple
                .elems
                .iter()
                .map(|elem| {
                    // for every tuple element, generate its getter code
                    let getter = generate_getter_inner(elem, is_getter);
                    // and add a scope around it
                    quote! { {#getter} }
                })
                .reduce(|acc, next| {
                    // join all getter codes with:
                    if is_getter {
                        // comma, to later produce (val_1, val_2, ...)
                        quote!(#acc, #next)
                    } else {
                        // bool-and, since for try_from we just generate bools
                        quote!(#acc && #next)
                    }
                })
                // `field: (),` will be handled like this:
                .unwrap_or_else(|| quote!());
            // add tuple braces, to produce (val_1, val_2, ...)
            quote! { (#unbraced) }
        }
        Array(array) => {
            // [[T; N1]; N2] -> (N1*N2, T)
            let (len_expr, elem_ty) = length_and_type_of_nested_array(array);
            // generate the getter code for one array element
            let array_elem = generate_getter_inner(&elem_ty, is_getter);
            // either generate an array or only check each value
            if is_getter {
                quote! {
                    // constness: iter, array::from_fn, for-loop, range are not const, so we're using while loops
                    // Modified version of the array init example in [`MaybeUninit`]:
                    let array = {
                        // [T; N1*N2]
                        let mut array: [::core::mem::MaybeUninit<#elem_ty>; #len_expr] = unsafe {
                            ::core::mem::MaybeUninit::uninit().assume_init()
                        };
                        let mut i = 0;
                        while i < #len_expr {
                            // for every element, get its value
                            let elem_value = {
                                #array_elem
                            };
                            // and write it to the output array
                            array[i].write(elem_value);
                            i += 1;
                        }
                        // [T; N1*N2] -> [[T; N1]; N2]
                        unsafe { ::core::mem::transmute(array) }
                    };
                    array
                }
            } else {
                quote! { {
                    let mut is_filled = true;
                    let mut i = 0;
                    // TODO: this could be simplified for always-filled values
                    while i < #len_expr {
                        // for every element, get its filled check
                        let elem_filled = {
                            #array_elem
                        };
                        // and join it with the others
                        is_filled = is_filled && elem_filled;
                        i += 1;
                    }
                    is_filled
                } }
            }
        }
        Path(_) => {
            // get the size, so we can shift to the next element's offset
            let size = shared::generate_type_bitsize(ty);
            // get the mask, so we can get this element's value
            let mask = generate_ty_mask(ty);

            // do all steps until conversion
            let elem_value = quote! {
                // the element's mask
                let mask = #mask;
                // the cursor starts at this element's offset, now get its value
                let raw_value = cursor & mask;
                // after getting the value, we can shift by the element's size
                // TODO: we could move this into tuple/array (and try_from, below)
                let size = #size;
                cursor = cursor.wrapping_shr(size as u32);
                // cast the element value (e.g. u32 -> u8),
                let raw_value: BaseIntOf<#ty> = raw_value as BaseIntOf<#ty>;
                // which allows it to be used here (e.g. u4::new(u8))
                let elem_value = <#ty as Bitsized>::ArbitraryInt::new(raw_value);
            };

            if is_getter {
                // generate the real value from the arbint `elem_value`
                quote! {
                    #elem_value
                    match <#ty>::try_from(elem_value) {
                        Ok(v) => v,
                        Err(_) => panic!("unreachable"),
                    }
                }
            } else {
                // generate only the filled check
                if shared::is_always_filled(ty) {
                    // skip the obviously filled values
                    quote! {
                        // we still need to shift by the element's size
                        let size = #size;
                        cursor = cursor.wrapping_shr(size as u32);
                        true
                    }
                } else {
                    // handle structs, enums - everything which can be unfilled
                    quote! { {
                        #elem_value
                        // so, has try_from impl
                        // note this is available even if the type is `From`
                        <#ty>::try_from(elem_value).is_ok()
                    } }
                }
            }
        }
        _ => unreachable(()),
    }
}

/// Top-level function which initializes the offset, masks other values and combines the final value
///
/// `is_array_elem_setter` allows us to generate a set_array_at setter more easily
pub(crate) fn generate_setter_value(ty: &Type, offset: &TokenStream, is_array_elem_setter: bool) -> TokenStream {
    // if we generate `fn set_array_at(index, value)`, we need to offset to the array element
    let elem_offset = if is_array_elem_setter {
        let size = shared::generate_type_bitsize(ty);
        quote! {
            let size = #size;
            // offset now starts at this element
            offset += size * index;
        }
    } else {
        quote!()
    };

    let value_shifted = generate_setter_inner(ty);
    // get the mask, so we can set this field's value
    let mask = generate_ty_mask(ty);
    quote! {
        type ArbIntOf<T> = <T as Bitsized>::ArbitraryInt;
        type BaseIntOf<T> = <ArbIntOf<T> as Number>::UnderlyingType;

        // offset now starts at this field
        let mut offset = #offset;
        #elem_offset

        let field_mask = #mask;
        // shift the mask into place
        let field_mask: BaseIntOf<Self> = field_mask << offset;
        // all other fields as a mask
        let others_mask: BaseIntOf<Self> = !field_mask;
        // the current struct value
        let struct_value: BaseIntOf<Self> = self.value.value();
        // mask off the field getting set
        let others_values: BaseIntOf<Self> = struct_value & others_mask;

        // get the new field value, shifted into place
        #value_shifted

        // join the values using bit-or
        let new_struct_value = others_values | value_shifted;
        self.value = <ArbIntOf<Self>>::new(new_struct_value);
    }
}

/// We heavily rely on the fact that transmuting into a nested array [[T; N1]; N2] can
/// be done in the same way as transmuting into an array [T; N1*N2].
/// Otherwise, nested arrays would generate even more code.
fn generate_setter_inner(ty: &Type) -> TokenStream {
    use Type::*;
    match ty {
        Tuple(tuple) => {
            // to index into the tuple value
            let mut tuple_index = syn::Index::from(0);
            let value_shifted = tuple
                .elems
                .iter()
                .map(|elem| {
                    let elem_name = quote!(value.#tuple_index);
                    tuple_index.index += 1;
                    // for every tuple element, generate its setter code
                    let value_shifted = generate_setter_inner(elem);
                    // set the value and add a scope around it
                    quote! { {
                        let value = #elem_name;
                        #value_shifted
                        value_shifted
                    } }
                })
                // join all setter codes with bit-or
                .reduce(|acc, next| quote!(#acc | #next))
                // `field: (),` will be handled like this:
                .unwrap_or_else(|| quote!(0));
            quote! {
                let value_shifted = #value_shifted;
            }
        }
        Array(array) => {
            // [[T; N1]; N2] -> (N1*N2, T)
            let (len_expr, elem_ty) = length_and_type_of_nested_array(array);
            // generate the setter code for one array element
            let value_shifted = generate_setter_inner(&elem_ty);
            quote! {
                // [[T; N1]; N2] -> [T; N1*N2], for example: [[(u2, u2); 3]; 4] -> [(u2, u2); 12]
                #[allow(clippy::useless_transmute)]
                let value: [#elem_ty; #len_expr] = unsafe { ::core::mem::transmute(value) };
                // constness: iter, for-loop, range are not const, so we're using while loops
                // [u4; 8] -> u32
                let mut acc = 0;
                let mut i = 0;
                while i < #len_expr {
                    let value = value[i];
                    // for every element, shift its value into its place
                    #value_shifted
                    // and bit-or them together
                    acc |= value_shifted;
                    i += 1;
                }
                let value_shifted = acc;
            }
        }
        Path(_) => {
            // get the size, so we can reach the next element afterwards
            let size = shared::generate_type_bitsize(ty);
            quote! {
                // the element's value as it's underlying type
                let value: BaseIntOf<#ty> = <ArbIntOf<#ty>>::from(value).value();
                // cast the element value (e.g. u8 -> u32),
                // which allows it to be combined with the struct's value later
                let value: BaseIntOf<Self> = value as BaseIntOf<Self>;
                let value_shifted = value << offset;
                // increase the offset to allow the next element to be read
                offset += #size;
            }
        }
        _ => unreachable(()),
    }
}

/// The constructor code just needs every field setter.
///
/// [`super::generate_struct`] contains the initialization of `offset`.
pub(crate) fn generate_constructor_part(ty: &Type, name: &Ident, offset: &TokenStream) -> TokenStream {
    let value_shifted = generate_setter_inner(ty);
    // setters look like this: `fn set_field1(&mut self, value: u3)`
    // constructors like this: `fn new(field1: u3, field2: u4) -> Self`
    // so we need to rename `field1` -> `value` and put this in a scope
    quote! { {
        let mut offset = #offset;
        let value = #name;
        #value_shifted
        value_shifted
    } }
}

/// We mostly need this in [`generate_setter_value`], to mask the whole field.
/// It basically combines a bunch of `Bitsized::MAX` values into a mask.
fn generate_ty_mask(ty: &Type) -> TokenStream {
    use Type::*;
    match ty {
        Tuple(tuple) => {
            let mut previous_elem_sizes = vec![];
            tuple
                .elems
                .iter()
                .map(|elem| {
                    // for every element, generate a mask
                    let mask = generate_ty_mask(elem);
                    // get it's size
                    let elem_size = shared::generate_type_bitsize(elem);
                    // generate it's offset from all previous sizes
                    let elem_offset = previous_elem_sizes.iter().cloned().reduce(|acc, next| quote!((#acc + #next)));
                    previous_elem_sizes.push(elem_size);
                    // the first field doesn't need to be shifted
                    if let Some(elem_offset) = elem_offset {
                        quote!(#mask << #elem_offset)
                    } else {
                        quote!(#mask)
                    }
                })
                // join all shifted masks with bit-or
                .reduce(|acc, next| quote!(#acc | #next))
                // `field: (),` will be handled like this:
                .unwrap_or_else(|| quote!(0))
        }
        Array(array) => {
            let elem_ty = &array.elem;
            let len_expr = &array.len;
            // generate the mask for one array element
            let mask = generate_ty_mask(elem_ty);
            // and the size
            let ty_size = shared::generate_type_bitsize(elem_ty);
            quote! { {
                let mask = #mask;
                let mut field_mask = 0;
                let mut i = 0;
                while i < #len_expr {
                    // for every element, shift its mask into its place
                    // and bit-or them together
                    field_mask |= mask << (i * #ty_size);
                    i += 1;
                }
                field_mask
            } }
        }
        Path(_) => quote! {
            // Casting this is needed in some places, but it might not be needed in some others.
            // (u2, u12) -> u8 << 0 | u16 << 2 -> u8 | u16 not possible
            (<#ty as Bitsized>::MAX.value() as BaseIntOf<Self>)
        },
        _ => unreachable(()),
    }
}

/// We compute nested length here, to fold [[T; N]; M] to [T; N * M].
fn length_and_type_of_nested_array(array: &syn::TypeArray) -> (TokenStream, Type) {
    let elem_ty = &array.elem;
    let len_expr = &array.len;
    if let Type::Array(array) = &**elem_ty {
        let (child_len, child_ty) = length_and_type_of_nested_array(array);
        (quote!((#len_expr) * (#child_len)), child_ty)
    } else {
        (quote!(#len_expr), *elem_ty.clone())
    }
}

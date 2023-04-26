use super::*;

/// We're keeping most of the generating together, to ease reading here and in `cargo_expand`.
/// For this reason, we also use more locals and types.
/// This should all be optimized away - if not, we will change back and have a debug mode.
pub(crate) fn generate_getter_value(ty: &Type, offset: &TokenStream) -> TokenStream {
    let inner = generate_getter_inner(ty, true);
    quote! {
        type ArbIntOf<T> = <T as Bitsized>::ArbitraryInt;
        type BaseIntOf<T> = <ArbIntOf<T> as Number>::UnderlyingType;
        // cursor starts at struct's first field
        let mut cursor = self.value.value();
        let field_offset = #offset;
        // cursor now starts at this field
        cursor >>= field_offset;
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
            let unbraced = tuple.elems.iter()
                .map(|elem| {
                    let getter = generate_getter_inner(elem, is_getter);
                    quote! { {#getter} }
                })
                .reduce(|acc, next| {
                    if is_getter {
                        quote!(#acc, #next)
                    } else {
                        quote!(#acc && #next)
                    }
                })
                // `field: (),` will be handled like this:
                .unwrap_or_else(|| quote!());
            quote! { (#unbraced) }
        },
        Array(array) => {
            let (len_expr, elem_ty) = length_and_type_of_nested_array(array);
            let array_elem = generate_getter_inner(&elem_ty, is_getter);
            if is_getter {
                quote! {
                    // constness: iter, array::from_fn, for-loop, range are not const, so we're using while loops
                    // Modified version of the array init example in [`MaybeUninit`]:
                    use core::mem::MaybeUninit;
                    let array = {
                        let mut array: [MaybeUninit<#elem_ty>; #len_expr] = unsafe {
                            MaybeUninit::uninit().assume_init()
                        };
                        let mut i = 0;
                        while i < #len_expr {
                            let elem_value = {
                                #array_elem
                            };
                            array[i].write(elem_value);
                            i += 1;
                        }
                        unsafe { core::mem::transmute(array) }
                    };
                    array
                }
            } else {
                quote! {
                    let mut is_filled = true;
                    let mut i = 0;
                    while i < #len_expr {
                        let elem_value = {
                            #array_elem
                        };
                        is_filled = is_filled && elem_value;
                        i += 1;
                    }
                    is_filled
                }
            }
        },
        Path(_) => {
            let size = shared::generate_field_bitsize(ty);
            let mask = generate_ty_mask(ty);
            let return_statement = if is_getter {
                quote! {
                    match #ty::try_from(elem_value) {
                        Ok(v) => v,
                        Err(_) => panic!("unreachable"),
                    }
                }
            } else {
                if shared::is_always_filled(ty) {
                    return quote! {
                        // we still need to shift by the element's size
                        let size = #size;
                        cursor >>= size;
                        true
                    };
                }
                quote! {
                    // so, has try_from impl
                    if !#ty::FILLED {
                        #ty::try_from(elem_value).is_ok()
                    } else {
                        true
                    }
                }
            };
            quote! { {
                let mask = #mask;
                let raw_value = cursor & mask;
                // after getting the value, we can shift by the element's size
                let size = #size;
                cursor >>= size;
                let raw_value = raw_value as <<#ty as Bitsized>::ArbitraryInt as Number>::UnderlyingType;
                let elem_value = <#ty as Bitsized>::ArbitraryInt::new(raw_value);
                #return_statement
            } }
        },
        _ => unreachable(()),
    }
}

/// We're keeping most of the generating together, to ease reading here and in `cargo_expand`.
/// For this reason, we also use more locals and types.
/// This should all be optimized away - if not, we will change back and have a debug mode.
pub(crate) fn generate_setter_value(ty: &Type, offset: &TokenStream) -> TokenStream {
    let inner = generate_setter_inner(ty);
    let mask = generate_ty_mask(ty);
    quote! {
        type ArbIntOf<T> = <T as Bitsized>::ArbitraryInt;
        type BaseIntOf<T> = <ArbIntOf<T> as Number>::UnderlyingType;

        // offset now starts at this field
        let mut offset = #offset;
        let field_mask = #mask;
        let field_mask: BaseIntOf<Self> = field_mask as BaseIntOf<Self>;
        let field_mask: BaseIntOf<Self> = field_mask << offset;
        // all other fields as a mask
        let others_mask: BaseIntOf<Self> = !field_mask;
        let struct_value: BaseIntOf<Self> = self.value.value();
        // the current struct value, masking off the field getting set
        let others_values: BaseIntOf<Self> = struct_value & others_mask;

        let field_value_shifted: BaseIntOf<Self>  = #inner;

        let new_struct_value = others_values | field_value_shifted;
        self.value = <ArbIntOf<Self>>::new(new_struct_value);
    }
}

fn generate_setter_inner(ty: &Type) -> TokenStream {
    use Type::*;
    match ty {
        Tuple(tuple) => {
            let mut tuple_index = syn::Index::from(0);
            tuple.elems.iter()
                .map(|elem| {
                    let elem_name = quote!(value.#tuple_index);
                    tuple_index.index += 1;
                    let setter = generate_setter_inner(elem);
                    quote! { {
                        let value = #elem_name;
                        let setter = #setter;
                        setter
                    } }
                })
                .reduce(|acc, next| quote!(#acc | #next))
                // `field: (),` will be handled like this:
                .unwrap_or_else(|| quote!(0))
        },
        Array(array) => {
            // We are merging higher level arrays into simple arrays: [[]] -> []
            let (len_expr, elem_ty) = length_and_type_of_nested_array(array);
            let set_inner = generate_setter_inner(&elem_ty);
            quote! { {
                // [[(u2, u2); 3]; 4] -> [(u2, u2); 12]
                #[allow(clippy::useless_transmute)]
                let value: [#elem_ty; #len_expr] = unsafe { core::mem::transmute(value) };
                // constness: iter, for-loop, range are not const, so we're using while loops
                // [u4; 8] -> u32
                let mut acc = 0;
                let mut i = 0;
                while i < #len_expr {
                    let value = value[i];
                    let elem_value_shifted = #set_inner;
                    acc |= elem_value_shifted;
                    i += 1;
                }
                acc
            } }
        },
        Path(_) => {
            let size = shared::generate_field_bitsize(ty);
            quote! { {
                let value: BaseIntOf<#ty> = <ArbIntOf<#ty>>::from(value).value();
                let value: BaseIntOf<Self> = value as BaseIntOf<Self>;
                let value_shifted = value << offset;
                offset += #size;
                value_shifted
            } }
        },
        _ => unreachable(()),
    }
}


pub(crate) fn generate_constructor_part(ty: &Type, name: &Ident) -> TokenStream {
    let setter = generate_setter_inner(ty);
    quote! { {
        let value = #name;
        let field = #setter;
        field
    } }
}


fn generate_ty_mask(ty: &Type) -> TokenStream {
    use Type::*;
    match ty {
        Tuple(tuple) => {
            let mut previous_elem_sizes = vec![];
            tuple.elems.iter()
                .map(|elem| {
                    let mask = generate_ty_mask(elem);
                    let elem_size = shared::generate_field_bitsize(elem);
                    let elem_offset = previous_elem_sizes.iter().cloned().reduce(|acc, next| quote!((#acc + #next)));
                    previous_elem_sizes.push(elem_size);
                    if let Some(elem_offset) = elem_offset {
                        quote!(#mask << #elem_offset)
                    } else {
                        quote!(#mask)
                    }
                })
                .reduce(|acc, next| quote!(#acc | #next))
                // `field: (),` will be handled like this:
                .unwrap_or_else(|| quote!(0))
        },
        Array(array) => {
            let elem_ty = &array.elem;
            let len_expr = &array.len;
            let mask = generate_ty_mask(elem_ty);
            let ty_size = shared::generate_field_bitsize(elem_ty);
            quote! { {
                let mask = #mask;
                let mut field_mask = 0;
                let mut i = 0;
                while i < #len_expr {
                    field_mask |= mask << (i * #ty_size);
                    i += 1;
                }
                field_mask
            } }
        },
        Path(_) => quote! {
            // Casting this is needed in some places, but it might not be needed in some others.
            // (u2, u12) -> u8 << 0 | u16 << 2 -> u8 | u16 not possible
            (<#ty as Bitsized>::MAX.value() as BaseIntOf<Self>)
        },
        _ => unreachable(()),
    }
}

// We compute nested length here, to fold [[T; N]; M] to [T; N * M].
// Recursion also stops when we hit a Tuple, which is handled outside.
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

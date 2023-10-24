// #[no_mangle]
// pub unsafe extern "C" fn min_max_rust(numbers: *mut i32, numbers_length: i32, out_max: &mut i32, out_min: &mut i32) {
//     let mut local_max = *numbers.offset(0);
//     let mut local_min = *numbers.offset(0);
//     for i in 0..numbers_length {
//         if *numbers.offset(i as isize) > local_max {
//             local_max = *numbers.offset(i as isize);
//         }
//         if *numbers.offset(i as isize) < local_min {
//             local_min = *numbers.offset(i as isize);
//         }
//     }
//     *out_max = local_max;
//     *out_min = local_min;
// }

#[repr(C)]
pub struct MinMax {
    pub max: i32,
    pub min: i32,
}

#[no_mangle]
pub unsafe extern "C" fn min_max_rust_idiomatic(numbers: *mut i32, numbers_length: i32) -> MinMax {
    let slice = std::slice::from_raw_parts_mut(numbers, numbers_length as usize);

    slice.iter().fold(MinMax { max: 0, min: 0 }, |mut acc, &x| {
        if x > acc.max {
            acc.max = x;
        }
        if x < acc.min {
            acc.min = x;
        }
        acc
    })
}

fn main() {}

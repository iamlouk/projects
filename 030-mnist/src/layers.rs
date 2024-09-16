use std::process::Output;

use crate::{Layer, BATCHSIZE};

fn get_flat<const N: usize, const M: usize, const DEPTH: usize>(
    tensor: &mut std::rc::Rc<[[[[f32; N]; M]; DEPTH]; BATCHSIZE]>,
) -> &mut [f32] {
    unsafe {
        debug_assert!(std::rc::Rc::strong_count(tensor) == 1);
        let ptr = std::rc::Rc::get_mut_unchecked(tensor) as *mut _ as *mut f32;
        std::slice::from_raw_parts_mut(ptr, N * M * DEPTH * BATCHSIZE)
    }
}

pub struct ReLU<const N: usize, const M: usize, const DEPTH: usize> {
    input: Option<std::rc::Rc<[[[[f32; N]; M]; DEPTH]; BATCHSIZE]>>,
}

impl<const N: usize, const M: usize, const DEPTH: usize> Layer<N, M, DEPTH> for ReLU<N, M, DEPTH> {
    const N_OUT: usize = N;
    const M_OUT: usize = M;
    const DEPTH_OUT: usize = DEPTH;

    fn forward(
        &mut self,
        mut input_buf: std::rc::Rc<[[[[f32; N]; M]; DEPTH]; BATCHSIZE]>,
    ) -> std::rc::Rc<[[[[f32; Self::N_OUT]; Self::M_OUT]; Self::DEPTH_OUT]; BATCHSIZE]> {
        self.input = Some(input_buf.clone());
        let mut output_buf = Self::alloc_output_tensor();
        let input = get_flat(&mut input_buf);
        let output = get_flat(&mut output_buf);
        for i in 0..input.len() {
            output[i] = input[i].max(0.);
        }

        output_buf
    }

    fn backward(
        &mut self,
        mut error_buf: std::rc::Rc<
            [[[[f32; Self::N_OUT]; Self::M_OUT]; Self::DEPTH_OUT]; BATCHSIZE],
        >,
    ) -> std::rc::Rc<[[[[f32; N]; M]; DEPTH]; BATCHSIZE]> {
        let mut output_buf = Self::alloc_input_tensor();
        let input = get_flat(self.input.as_mut().unwrap());
        let error = get_flat(&mut error_buf);
        let output = get_flat(&mut output_buf);
        for i in 0..input.len() {
            output[i] = if input[i] <= 0. { 0. } else { error[i] };
        }

        output_buf
    }
}

use rand::Rng;

use crate::{Layer, BATCHSIZE};

fn get_flat<const N: usize, const M: usize, const CHANNELS: usize>(
    tensor: &mut std::rc::Rc<[[[[f32; N]; M]; CHANNELS]; BATCHSIZE]>,
) -> &mut [f32] {
    unsafe {
        debug_assert!(std::rc::Rc::strong_count(tensor) == 1);
        let ptr = std::rc::Rc::get_mut_unchecked(tensor) as *mut _ as *mut f32;
        std::slice::from_raw_parts_mut(ptr, N * M * CHANNELS * BATCHSIZE)
    }
}

fn range_2d(n: usize, m: usize) -> impl std::iter::Iterator<Item = (usize, usize)> {
    (0..n).flat_map(move |a| (0..m).map(move |b| (a, b)))
}

#[allow(dead_code)]
pub struct ReLU<const N: usize, const M: usize, const CHANNELS: usize> {
    input: Option<std::rc::Rc<[[[[f32; N]; M]; CHANNELS]; BATCHSIZE]>>,
}

impl<const N: usize, const M: usize, const CHANNELS: usize> Layer<N, M, CHANNELS, N, M, CHANNELS>
    for ReLU<N, M, CHANNELS>
{
    fn new(_rng: &mut rand::prelude::ThreadRng) -> Self {
        Self { input: None }
    }

    fn forward(
        &mut self,
        mut input_buf: std::rc::Rc<[[[[f32; N]; M]; CHANNELS]; BATCHSIZE]>,
    ) -> std::rc::Rc<[[[[f32; N]; M]; CHANNELS]; BATCHSIZE]> {
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
        mut error_buf: std::rc::Rc<[[[[f32; N]; M]; CHANNELS]; BATCHSIZE]>,
    ) -> std::rc::Rc<[[[[f32; N]; M]; CHANNELS]; BATCHSIZE]> {
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

#[allow(dead_code)]
struct Convolution<
    const N_IN: usize,
    const M_IN: usize,
    const CHANNELS_IN: usize,
    const CHANNELS_OUT: usize,
    const KERNEL_SIZE: usize,
> {
    pub weights: Box<[[[[f32; KERNEL_SIZE]; KERNEL_SIZE]; CHANNELS_IN]; CHANNELS_OUT]>,
    pub biases: Box<[f32; CHANNELS_OUT]>,
    pub input: Option<std::rc::Rc<[[[[f32; N_IN]; M_IN]; CHANNELS_IN]; BATCHSIZE]>>,
}

impl<
        const N_IN: usize,
        const M_IN: usize,
        const CHANNELS_IN: usize,
        const CHANNELS_OUT: usize,
        const KERNEL_SIZE: usize,
    > Convolution<N_IN, M_IN, CHANNELS_IN, CHANNELS_OUT, KERNEL_SIZE>
{
    fn conv(
        bias: f32,
        weights: &[[[f32; KERNEL_SIZE]; KERNEL_SIZE]; CHANNELS_IN],
        input: &[[[f32; N_IN]; M_IN]; CHANNELS_IN],
        output: &mut [[f32; N_IN - ((KERNEL_SIZE - 1) / 2)]; M_IN - ((KERNEL_SIZE - 1) / 2)],
    ) {
        let N_OUT = N_IN - ((KERNEL_SIZE - 1) / 2);
        let M_OUT = M_IN - ((KERNEL_SIZE - 1) / 2);
        for (i_out, j_out) in range_2d(N_OUT, M_OUT) {
            let mut val = bias;
            for c in 0..CHANNELS_IN {
                for (di, dj) in range_2d(KERNEL_SIZE, KERNEL_SIZE) {
                    let x = input[c][i_out + di][j_out + dj] * weights[c][di][dj];
                    val += x;
                }
            }
            output[i_out][j_out] = val;
        }
    }
}

impl<
        const N_IN: usize,
        const M_IN: usize,
        const CHANNELS_IN: usize,
        const CHANNELS_OUT: usize,
        const KERNEL_SIZE: usize,
    >
    Layer<
        N_IN,
        M_IN,
        CHANNELS_IN,
        { N_IN - ((KERNEL_SIZE - 1) / 2) },
        { M_IN - ((KERNEL_SIZE - 1) / 2) },
        CHANNELS_OUT,
    > for Convolution<N_IN, M_IN, CHANNELS_IN, CHANNELS_OUT, KERNEL_SIZE>
{
    fn new(rng: &mut rand::prelude::ThreadRng) -> Self {
        let mut layer = Self {
            weights: Box::new([[[[0.0; KERNEL_SIZE]; KERNEL_SIZE]; CHANNELS_IN]; CHANNELS_OUT]),
            biases: Box::new([0.0; CHANNELS_OUT]),
            input: None,
        };

        for c in 0..CHANNELS_OUT {
            layer.biases[c] = rng.gen_range(-1.0..1.0);
        }

        for c_out in 0..CHANNELS_OUT {
            for c_in in 0..CHANNELS_IN {
                for x in 0..KERNEL_SIZE {
                    for y in 0..KERNEL_SIZE {
                        layer.weights[c_out][c_in][x][y] = rng.gen_range(-1.0..1.0);
                    }
                }
            }
        }

        layer
    }

    fn forward(
        &mut self,
        input_buf: std::rc::Rc<[[[[f32; N_IN]; M_IN]; CHANNELS_IN]; BATCHSIZE]>,
    ) -> std::rc::Rc<
        [[[[f32; N_IN - ((KERNEL_SIZE - 1) / 2)]; M_IN - ((KERNEL_SIZE - 1) / 2)]; CHANNELS_OUT];
            BATCHSIZE],
    > {
        self.input = Some(input_buf.clone());
        let mut output_buf = Self::alloc_output_tensor();
        let input = &*input_buf;
        let output = std::rc::Rc::get_mut(&mut output_buf).unwrap();

        // TODO: Rayon parallelize around the batch or channels out dimension?
        for b in 0..BATCHSIZE {
            let input = &input[b];
            for c_out in 0..CHANNELS_OUT {
                let bias = self.biases[c_out];
                let weights = &self.weights[c_out];
                let output = &mut output[b][c_out];
                Self::conv(bias, weights, input, output);
            }
        }

        output_buf
    }

    fn backward(
        &mut self,
        errors: std::rc::Rc<
            [[[[f32; N_IN - ((KERNEL_SIZE - 1) / 2)]; M_IN - ((KERNEL_SIZE - 1) / 2)];
                CHANNELS_OUT]; BATCHSIZE],
        >,
    ) -> std::rc::Rc<[[[[f32; N_IN]; M_IN]; CHANNELS_IN]; BATCHSIZE]> {
        unimplemented!()
    }
}

#![allow(clippy::manual_range_contains)]
#![allow(clippy::type_complexity)]
#![feature(iter_array_chunks)]
#![feature(get_mut_unchecked)]
#![feature(generic_const_exprs)]
#![feature(allocator_api)]

const BATCHSIZE: usize = 15;

trait Layer<const N_IN: usize, const M_IN: usize, const DEPTH_IN: usize> {
    const N_OUT: usize;
    const M_OUT: usize;
    const DEPTH_OUT: usize;

    fn forward(
        &mut self,
        input: std::rc::Rc<[[[[f32; N_IN]; M_IN]; DEPTH_IN]; BATCHSIZE]>,
    ) -> std::rc::Rc<[[[[f32; Self::N_OUT]; Self::M_OUT]; Self::DEPTH_OUT]; BATCHSIZE]>;

    fn backward(
        &mut self,
        errors: std::rc::Rc<[[[[f32; Self::N_OUT]; Self::M_OUT]; Self::DEPTH_OUT]; BATCHSIZE]>,
    ) -> std::rc::Rc<[[[[f32; N_IN]; M_IN]; DEPTH_IN]; BATCHSIZE]>;

    /* Workaround for limitations in Rust's generic trait stuff. */
    fn transmute_in2out(
        tensor: std::rc::Rc<[[[[f32; N_IN]; M_IN]; DEPTH_IN]; BATCHSIZE]>,
    ) -> std::rc::Rc<[[[[f32; Self::N_OUT]; Self::M_OUT]; Self::DEPTH_OUT]; BATCHSIZE]> {
        debug_assert!(N_IN * M_IN * DEPTH_IN == Self::N_OUT * Self::M_OUT * Self::DEPTH_OUT);
        let (ptr, alloc) = std::rc::Rc::into_raw_with_allocator(tensor);
        unsafe {
            let ptr = ptr as *mut [[[[f32; Self::N_OUT]; Self::M_OUT]; Self::DEPTH_OUT]; BATCHSIZE];
            std::rc::Rc::from_raw_in(ptr, alloc)
        }
    }

    /* Workaround for limitations in Rust's generic trait stuff. */
    fn transmute_out2in(
        tensor: std::rc::Rc<[[[[f32; Self::N_OUT]; Self::M_OUT]; Self::DEPTH_OUT]; BATCHSIZE]>,
    ) -> std::rc::Rc<[[[[f32; N_IN]; M_IN]; DEPTH_IN]; BATCHSIZE]> {
        debug_assert!(N_IN * M_IN * DEPTH_IN == Self::N_OUT * Self::M_OUT * Self::DEPTH_OUT);
        let (ptr, alloc) = std::rc::Rc::into_raw_with_allocator(tensor);
        unsafe {
            let ptr = ptr as *mut [[[[f32; N_IN]; M_IN]; DEPTH_IN]; BATCHSIZE];
            std::rc::Rc::from_raw_in(ptr, alloc)
        }
    }

    /* Alloc input sized tensor */
    fn alloc_input_tensor() -> std::rc::Rc<[[[[f32; N_IN]; M_IN]; DEPTH_IN]; BATCHSIZE]> {
        std::rc::Rc::new([[[[0.0f32; N_IN]; M_IN]; DEPTH_IN]; BATCHSIZE])
    }

    /* Alloc output sized tensor */
    fn alloc_output_tensor(
    ) -> std::rc::Rc<[[[[f32; Self::N_OUT]; Self::M_OUT]; Self::DEPTH_OUT]; BATCHSIZE]> {
        std::rc::Rc::new([[[[0.0f32; Self::N_OUT]; Self::M_OUT]; Self::DEPTH_OUT]; BATCHSIZE])
    }
}

mod layers;

fn load_image(filename: &std::path::Path) -> Result<Box<[[f32; 28]; 28]>, String> {
    use png::{BitDepth, ColorType};
    let dec = png::Decoder::new(
        std::fs::File::open(filename).map_err(|e| format!("reading {:?}: {}", filename, e))?,
    );
    let mut reader = dec
        .read_info()
        .map_err(|e| format!("parsing {:?}: {}", filename, e))?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("parsing {:?}: {}", filename, e))?;
    let (coltype, bitdepth) = reader.output_color_type();
    if info.width != 28
        || info.height != 28
        || reader.info().frame_control.is_some()
        || coltype != ColorType::Grayscale
        || bitdepth != BitDepth::Eight
    {
        return Err(format!(
            "unexpected PNG format: {:?}, color-type: {:?}, bitdepth: {:?}",
            info, coltype, bitdepth
        ));
    }
    let bytes = &buf[..info.buffer_size()];
    assert!(bytes.len() == 28 * 28);

    let mut img = Box::new([[0.0f32; 28]; 28]);
    for i in 0..28 {
        for j in 0..28 {
            let byte = bytes[i * 28 + j];
            let val = (byte as f32) / 256.;
            assert!(0. <= val && val < 1.);
            img[i][j] = val;
        }
    }

    Ok(img)
}

fn load_training_data(
    datadir: &str,
    shuffle_seed: u64,
) -> Result<
    (
        Vec<Box<[[[[f32; 28]; 28]; 1]; BATCHSIZE]>>,
        Vec<[i8; BATCHSIZE]>,
    ),
    String,
> {
    let t0 = std::time::Instant::now();

    let mut path = std::path::PathBuf::from(datadir);
    let mut images: Vec<(i8, Box<[[f32; 28]; 28]>)> = Vec::new();
    let readdir =
        std::fs::read_dir(&path).map_err(|e| format!("traversing {:?}: {}", datadir, e))?;
    for dirres in readdir {
        let dir = match dirres {
            Ok(e) => e,
            Err(e) => return Err(format!("traversing {:?}: {}", datadir, e)),
        };

        let name = dir.file_name();
        let class = match name.to_str().unwrap().parse::<i8>() {
            Ok(c) => c,
            Err(e) => {
                return Err(format!(
                    "expected digit, got: {:?}, error: {}",
                    dir.file_name(),
                    e
                ))
            }
        };

        path.push(name);
        let readdir =
            std::fs::read_dir(&path).map_err(|e| format!("traversing {:?}: {}", datadir, e))?;
        for dirres in readdir {
            let dir = match dirres {
                Ok(e) => e,
                Err(e) => return Err(format!("traversing {:?}: {}", datadir, e)),
            };

            let name = dir.file_name();
            path.push(name);
            images.push((class, load_image(&path)?));
            path.pop();
        }
        path.pop();
    }

    use rand::prelude::*;
    let mut rng = StdRng::seed_from_u64(shuffle_seed);
    images.shuffle(&mut rng);
    let n = images.len();
    let mut inputs: Vec<Box<[[[[f32; 28]; 28]; 1]; BATCHSIZE]>> = Vec::with_capacity(n / BATCHSIZE);
    let mut classes: Vec<[i8; BATCHSIZE]> = Vec::with_capacity(n / BATCHSIZE);

    for chunk in images.into_iter().array_chunks::<BATCHSIZE>() {
        // Would need clone of image: classes.push(image.map(|(class, _)| class));
        let mut batch = [-1i8; BATCHSIZE];
        for (i, (class, _)) in chunk.iter().enumerate() {
            batch[i] = *class;
        }
        classes.push(batch);

        inputs.push(Box::new(chunk.map(|(_, image)| [*image])));
    }

    let dt = std::time::Instant::now() - t0;
    eprintln!(
        "load_training_data: {:.3}s, images: {}",
        dt.as_secs_f32(),
        n
    );
    Ok((inputs, classes))
}

fn main() {
    let (inputs, classes) = load_training_data("./data/mnist_png/train", 1234).expect("error");

    assert_eq!(inputs.len(), classes.len());
}

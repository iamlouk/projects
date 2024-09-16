use std::{collections::HashMap, os::{fd::AsRawFd, unix::fs::MetadataExt}};

const NEWLINE: u8 = 10; // '\n'
const AVG_LINE_LEN: usize = 18;

struct WorkerResult<'a> {
    lines: usize,
    cities: HashMap<&'a [u8], (f32, f32, u32, f32)>
}

fn handle_line<'a>(line: &'a [u8]) -> (&'a [u8], f32) {
    let mut name_end = 0;
    while line[name_end] != (';' as u8) {
        name_end += 1;
    }

    let val = std::str::from_utf8(&line[(name_end + 1)..]).unwrap().parse::<f32>().unwrap();
    (&line[0..name_end], val)
}

fn worker<'a>(mut start: usize, end: usize, data: &'a [u8]) -> WorkerResult {
    // Find start of first new line in data after offset:
    if start != 0 && data[start - 1] != NEWLINE {
        while data[start] != NEWLINE {
            start += 1;
        }
        start += 1;
    }

    // Loop over lines:
    assert!(start < end && end <= data.len());
    let mut res = WorkerResult { lines: 0, cities: HashMap::with_capacity((end - start) / AVG_LINE_LEN) };
    let mut pos = start;
    loop {
        assert!(pos == 0 || data[pos - 1] == NEWLINE);

        let line_start = pos;
        while pos < data.len() && data[pos] != NEWLINE {
            pos += 1;
        }

        if pos != line_start && data[line_start] != ('#' as u8) {
            let (name, val) = handle_line(&data[line_start..pos]);
            if let Some(vals) = res.cities.get_mut(name) {
                vals.0 = vals.0.min(val);
                vals.1 = vals.1.max(val);
                vals.2 += 1;
                vals.3 += val;
            } else {
                res.cities.insert(name, (val, val, 1, val));
            }
            res.lines += 1;
        }

        pos += 1;
        if pos > end {
            break;
        }
    }

    res
}

fn main() {
    let start_time = std::time::Instant::now();

    // Map the measurements file into our memory and use it as slice:
    let file = std::fs::File::open("./measurements.txt").unwrap();
    let size = file.metadata().unwrap().size() as usize;
    let fd = file.as_raw_fd();
    let raw_start = unsafe {
        libc::mmap(std::ptr::null_mut(), size as usize, libc::PROT_READ, libc::MAP_SHARED | libc::MAP_FILE, fd, 0)
    };
    assert!(!raw_start.is_null());
    let data = unsafe {
        std::slice::from_raw_parts(raw_start as *const u8, size as usize)
    };

    // Start workers and make them parse the input:
    let nworkers = std::thread::available_parallelism().unwrap().get();
    let chunksize = data.len() / nworkers;
    let mut workers: Vec<std::thread::JoinHandle<WorkerResult>> = Vec::with_capacity(nworkers);
    for i in 0..nworkers {
        let t = std::thread::spawn(move || {
            worker(i * chunksize, size.min(i * chunksize + chunksize), data)
        });
        workers.push(t);
    }

    // TODO: Make this reduction parallel?
    let mut result = workers
        .into_iter()
        .map(|h| h.join().unwrap())
        .reduce(|mut res, worker_res| {
            res.lines += worker_res.lines;
            for (name, worker_vals) in worker_res.cities.iter() {
                if let Some(vals) = res.cities.get_mut(name) {
                    vals.0 = vals.0.min(worker_vals.0);
                    vals.1 = vals.1.max(worker_vals.1);
                    vals.2 = vals.2 + worker_vals.2;
                    vals.2 = vals.2 + worker_vals.2;
                } else {
                    res.cities.insert(name, *worker_vals);
                }
            }
            res
        })
        .unwrap();

    result.cities.iter_mut().for_each(|(_, vals)| vals.3 = vals.3 / (vals.2 as f32));

    let end_time = std::time::Instant::now();
    let d = (end_time - start_time).as_secs_f32();

    println!("lines: {:?}", result.lines);
    for (name, vals) in result.cities {
        println!("city: {:<32}, #measurements: {}, min: {}, max: {}, avg.: {}", std::str::from_utf8(name).unwrap(), vals.2, vals.0, vals.1, vals.3);
    }

    println!("that took: {:?}s", d);
}

use ndarray::{stack, Array, Array1, Array2, ArrayView1, Axis};
use std::io::{BufReader, BufWriter, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{atomic::AtomicBool, Arc};
use std::time::{Duration, Instant};
use std::{thread, vec};

pub type Endpoint<'a> = Box<dyn FnMut(&mut TcpStream) + Send + 'a>;

pub fn run(
    buf_size: impl IntoIterator<Item = usize> + Clone,
    delay: impl IntoIterator<Item = u64> + Clone,
    iters: u32,
    e1: &mut Endpoint,
    e2: &mut Endpoint,
) -> Array2<f64> {
    let mut rows = Vec::new();

    for buf_size in buf_size.into_iter() {
        let row: Array1<f64> = Array::from_iter(delay.clone().into_iter().map(|delay| {
            println!("buf_size = {buf_size}, delay = {delay}");
            run_once(buf_size, delay, iters, e1, e2)
        }));

        rows.push(row);
    }

    let rows: Vec<ArrayView1<f64>> = rows.iter().map(|r| r.view()).collect();

    stack(Axis(0), rows.as_slice()).unwrap()
}

pub fn run_once(
    buf_size: usize,
    delay: u64,
    iters: u32,
    e1: &mut Endpoint,
    e2: &mut Endpoint,
) -> f64 {
    assert!(buf_size > 0);

    let mut time = 0.0;

    let bridge_cont = Arc::new(AtomicBool::new(true));
    let bridge_up = Arc::new(AtomicBool::new(false));

    let bridge = {
        let cont = bridge_cont.clone();
        let up = bridge_up.clone();
        thread::spawn(move || {
            let listener = TcpListener::bind("localhost:8000").unwrap();

            up.store(true, Ordering::Release);

            let (stream1, _) = listener.accept().unwrap();

            let (stream2, _) = listener.accept().unwrap();

            let forward = |mut from: BufReader<&TcpStream>, mut to: BufWriter<&TcpStream>| {
                let cont = cont.clone();
                let mut buf: Vec<u8> = vec![0; buf_size];
                while cont.load(Ordering::Acquire) {
                    let bytes_read = from.read(&mut buf).unwrap();

                    thread::sleep(Duration::from_millis(delay));

                    to.write(&buf[0..bytes_read]).unwrap();
                    to.flush().unwrap();
                }
            };

            thread::scope(|s| {
                let forward12 =
                    s.spawn(|| forward(BufReader::new(&stream1), BufWriter::new(&stream2)));

                let forward21 =
                    s.spawn(|| forward(BufReader::new(&stream2), BufWriter::new(&stream1)));

                forward12.join().unwrap();
                forward21.join().unwrap();
            });
        })
    };

    while !bridge_up.load(Ordering::Acquire) {}

    let clients_iter = Arc::new(AtomicU32::new(0));
    let clients_done = Arc::new(AtomicU32::new(0));

    let start_client = |e: &mut Endpoint| {
        let next_iter = clients_iter.clone();
        let done = clients_done.clone();

        let mut stream = TcpStream::connect("localhost:8000").unwrap();

        let mut i = 0;

        while i < iters {
            let j = next_iter.load(Ordering::Acquire);
            if j > i {
                e(&mut stream);
                i = j;
                done.fetch_add(1, Ordering::AcqRel);
            }
        }
    };

    thread::scope(|s| {
        let client1 = s.spawn(move || start_client(e1));

        let client2 = s.spawn(move || start_client(e2));

        let now = Instant::now();

        for i in 1..=iters {
            clients_iter.store(i, Ordering::Release);

            while clients_done.load(Ordering::Acquire) < 2 {}

            clients_done.store(0, Ordering::Release);
        }

        let elapsed = now.elapsed();
        time = (elapsed.as_millis() as f64) / (iters as f64);

        bridge_cont.store(false, Ordering::Release);

        client1.join().unwrap();
        client2.join().unwrap();
    });

    bridge.join().unwrap();

    time
}
